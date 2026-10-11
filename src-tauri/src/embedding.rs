//! Local EmbeddingGemma 2 protocol. All requests use the owned engine's pinned TLS.
use crate::{runtime::Connection, store::Result};
use serde_json::Value;

pub const DIMENSIONS: usize = 768;

async fn profile_task<T: Send + 'static>(
    work: impl FnOnce(std::sync::Arc<std::sync::atomic::AtomicBool>) -> Result<T> + Send + 'static,
) -> Result<T> {
    let cancelled = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    struct CancelOnDrop(std::sync::Arc<std::sync::atomic::AtomicBool>);
    impl Drop for CancelOnDrop {
        fn drop(&mut self) {
            self.0.store(true, std::sync::atomic::Ordering::Release);
        }
    }
    // Dropping a blocking JoinHandle does not interrupt its worker. Signal the
    // worker explicitly, including when the enclosing operation is cancelled.
    let _cancel_on_drop = CancelOnDrop(cancelled.clone());
    tokio::task::spawn_blocking(move || work(cancelled))
        .await
        .map_err(|_| "Embedding file verification failed.")?
}

pub async fn profile_cancellable(config: crate::runtime::RuntimeConfig) -> Result<String> {
    profile_task(move |cancelled| {
        profile_checked(&config, || {
            if cancelled.load(std::sync::atomic::Ordering::Acquire) {
                Err("Embedding file verification cancelled.".into())
            } else {
                Ok(())
            }
        })
    })
    .await
}

pub fn parse_vector(body: &Value) -> Result<Vec<f32>> {
    let data = body
        .get("data")
        .and_then(Value::as_array)
        .filter(|v| v.len() == 1)
        .ok_or("Expected exactly one embedding result.")?;
    let values = data[0]
        .get("embedding")
        .and_then(Value::as_array)
        .filter(|v| v.len() == DIMENSIONS)
        .ok_or("Expected a 768-dimensional EmbeddingGemma 2 vector.")?;
    let vector: Vec<f32> = values
        .iter()
        .map(|v| {
            v.as_f64()
                .map(|n| n as f32)
                .filter(|n| n.is_finite())
                .ok_or_else(|| "Embedding contains invalid numbers.".to_string())
        })
        .collect::<Result<_>>()?;
    let norm = vector
        .iter()
        .map(|n| f64::from(*n).powi(2))
        .sum::<f64>()
        .sqrt();
    if !(0.999..=1.001).contains(&norm) {
        return Err("Expected a normalized embedding vector.".into());
    }
    Ok(vector)
}

/// Hash model contents to prevent mixing incompatible embedding spaces after a restart.
pub fn profile(config: &crate::runtime::RuntimeConfig) -> Result<String> {
    profile_checked(config, || Ok(()))
}

fn profile_checked(
    config: &crate::runtime::RuntimeConfig,
    check: impl Fn() -> Result<()>,
) -> Result<String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut digest = Sha256::new();
    digest.update(b"embeddinggemma2-v2:title200bytes:text1500bytes:query1800bytes:media-v1:768");
    for path in [&config.model, &config.projector] {
        check()?;
        let path = crate::runtime::validate_model(
            path.as_ref()
                .ok_or("Choose embedding model and projector first.")?,
        )?;
        let mut file = crate::regular_file::open_regular_file(&path).map_err(|e| e.to_string())?;
        let mut file_hash = Sha256::new();
        let mut buffer = [0; 65536];
        loop {
            check()?;
            let size = file.read(&mut buffer).map_err(|e| e.to_string())?;
            check()?;
            if size == 0 {
                break;
            }
            file_hash.update(&buffer[..size]);
        }
        digest.update(file_hash.finalize());
    }
    check()?;
    Ok(format!("gemma2-v2:{:x}", digest.finalize()))
}

fn prefix_bytes(value: &str, limit: usize) -> &str {
    let mut end = value.len().min(limit);
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    &value[..end]
}

pub fn document_input(title: &str, text: &str) -> String {
    format!(
        "title: {} | text: {}",
        prefix_bytes(title, 200),
        prefix_bytes(text, 1500)
    )
}

pub async fn embed(connection: &Connection, input: &str) -> Result<Vec<f32>> {
    if input.trim().is_empty() || input.len() > 1900 {
        return Err("Embedding input must contain 1 to 1,900 UTF-8 bytes.".into());
    }
    let base =
        crate::inference::connection_endpoint(&connection.endpoint, Some(&connection.certificate))?;
    let response = crate::inference::client(120, Some(&connection.certificate))?
        .post(format!("{base}/v1/embeddings"))
        .bearer_auth(&connection.token)
        .json(&serde_json::json!({"input":input,"encoding_format":"float"}))
        .send()
        .await
        .map_err(|_| "Local embedding failed or exceeded 120 seconds.".to_string())?;
    parse_vector(&crate::inference::bounded_json(response).await?)
}

pub async fn query(connection: &Connection, query: &str) -> Result<Vec<f32>> {
    if query.trim().is_empty() || query.len() > 1800 {
        return Err("Semantic search accepts 1 to 1,800 UTF-8 bytes per query.".into());
    }
    embed(connection, &format!("task: search result | query: {query}")).await
}

fn media_request(props: &Value, part: &crate::media::MediaPart) -> Result<Value> {
    use base64::Engine;
    if part.data.is_empty() || part.data.len() > 2 * 1024 * 1024 {
        return Err("Embedding media part exceeds 2 MB.".into());
    }
    let modality = match part.mime.as_str() {
        "image/png" if part.data.starts_with(b"\x89PNG\r\n\x1a\n") => "vision",
        "audio/wav" if part.data.starts_with(b"RIFF") && part.data.get(8..12) == Some(b"WAVE") => {
            "audio"
        }
        _ => return Err("Unsupported embedding media format.".into()),
    };
    if props
        .get("modalities")
        .and_then(|m| m.get(modality))
        .and_then(Value::as_bool)
        != Some(true)
    {
        return Err(format!(
            "The embedding engine does not advertise {modality} support."
        ));
    }
    let marker = props
        .get("media_marker")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty() && s.len() <= 256 && !s.chars().any(char::is_control))
        .ok_or("The embedding engine returned an invalid media marker.")?;
    Ok(
        serde_json::json!({"input":{"prompt_string":marker,"multimodal_data":[base64::engine::general_purpose::STANDARD.encode(&part.data)]},"encoding_format":"float"}),
    )
}

pub async fn embed_media(
    connection: &Connection,
    part: &crate::media::MediaPart,
) -> Result<Vec<f32>> {
    let base =
        crate::inference::connection_endpoint(&connection.endpoint, Some(&connection.certificate))?;
    let response = crate::inference::client(5, Some(&connection.certificate))?
        .get(format!("{base}/props"))
        .bearer_auth(&connection.token)
        .send()
        .await
        .map_err(|_| "Could not read embedding capabilities.".to_string())?;
    let props = crate::inference::bounded_json(response).await?;
    let request = media_request(&props, part)?;
    let response = crate::inference::client(120, Some(&connection.certificate))?
        .post(format!("{base}/v1/embeddings"))
        .bearer_auth(&connection.token)
        .json(&request)
        .send()
        .await
        .map_err(|_| "Media embedding failed or exceeded 120 seconds.".to_string())?;
    parse_vector(&crate::inference::bounded_json(response).await?)
}

#[derive(Debug, serde::Serialize)]
pub struct IndexProgress {
    pub indexed: usize,
    pub total: usize,
}

pub fn index_progress(database: &std::path::Path, profile: &str) -> Result<IndexProgress> {
    let (indexed, total) = crate::store::Store::open(database)?.embedding_counts(profile)?;
    Ok(IndexProgress { indexed, total })
}

/// Each completed vector is committed before the next network request. Dropping
/// this future preserves those commits and leaves unfinished passages pending.
pub async fn index_batch(
    database: &std::path::Path,
    connection: &Connection,
    profile: &str,
) -> Result<IndexProgress> {
    index_batch_with(database, profile, |chunk, part| async move {
        if let Some(part) = part {
            embed_media(connection, &part).await
        } else {
            embed(connection, &document_input(&chunk.title, &chunk.text)).await
        }
    })
    .await
}

async fn index_batch_with<F, Fut>(
    database: &std::path::Path,
    profile: &str,
    mut embed_part: F,
) -> Result<IndexProgress>
where
    F: FnMut(crate::store::Hit, Option<crate::media::MediaPart>) -> Fut,
    Fut: std::future::Future<Output = Result<Vec<f32>>>,
{
    let store = crate::store::Store::open(database)?;
    for chunk in store.pending_embeddings(profile, 8)? {
        let part = store.media_part(&chunk.id)?;
        let id = chunk.id.clone();
        let vector = embed_part(chunk, part).await?;
        store.save_embedding(&id, profile, &vector)?;
    }
    let (indexed, total) = store.embedding_counts(profile)?;
    Ok(IndexProgress { indexed, total })
}

#[cfg(test)]
mod tests {
    #[test]
    fn profile_cancellation_stops_between_chunks_before_projector_validation() {
        let dir = tempfile::tempdir().unwrap();
        let model = dir.path().join("model.gguf");
        let mut contents = vec![0; 256 * 1024];
        contents[..4].copy_from_slice(b"GGUF");
        std::fs::write(&model, contents).unwrap();
        let config = crate::runtime::RuntimeConfig {
            executable: None,
            bundled_engine: false,
            model: Some(model),
            projector: None,
        };
        let checks = std::cell::Cell::new(0);
        let error = super::profile_checked(&config, || {
            checks.set(checks.get() + 1);
            if checks.get() == 4 {
                Err("cancelled at second read".into())
            } else {
                Ok(())
            }
        })
        .unwrap_err();
        assert_eq!(error, "cancelled at second read");
        assert_eq!(checks.get(), 4);
        let missing = crate::runtime::RuntimeConfig {
            model: None,
            ..config
        };
        assert_eq!(
            super::profile_checked(&missing, || Err("already cancelled".into())).unwrap_err(),
            "already cancelled"
        );
    }

    #[tokio::test]
    async fn async_profile_preserves_digest_and_validation_errors() {
        let dir = tempfile::tempdir().unwrap();
        let model = dir.path().join("model.gguf");
        let projector = dir.path().join("projector.gguf");
        std::fs::write(&model, b"GGUFmodel-one").unwrap();
        std::fs::write(&projector, b"GGUFprojector-one").unwrap();
        let config = crate::runtime::RuntimeConfig {
            executable: None,
            bundled_engine: false,
            model: Some(model),
            projector: Some(projector),
        };
        assert_eq!(
            super::profile_cancellable(config.clone()).await.unwrap(),
            super::profile(&config).unwrap()
        );
        let missing = crate::runtime::RuntimeConfig {
            model: None,
            ..config
        };
        assert_eq!(
            super::profile_cancellable(missing.clone())
                .await
                .unwrap_err(),
            super::profile(&missing).unwrap_err()
        );
    }

    #[tokio::test]
    async fn cancelled_profile_task_signals_already_running_worker() {
        let (started, ready) = tokio::sync::oneshot::channel();
        let (release, resume) = std::sync::mpsc::channel();
        let (observed, result) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(super::profile_task(move |cancelled| {
            started.send(()).unwrap();
            resume
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
            observed
                .send(cancelled.load(std::sync::atomic::Ordering::Acquire))
                .unwrap();
            Ok(())
        }));
        ready.await.unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        release.send(()).unwrap();
        assert!(
            result.await.unwrap(),
            "cancelled startup left verification work active"
        );
    }
    use super::*;
    use serde_json::json;
    #[test]
    fn media_request_requires_correct_modality_marker_and_bounded_payload() {
        let image = crate::media::MediaPart {
            mime: "image/png".into(),
            data: b"\x89PNG\r\n\x1a\nfixture".to_vec(),
            start_ms: None,
            end_ms: None,
            label: "Image".into(),
        };
        let props =
            json!({"media_marker":"<media-random>","modalities":{"vision":true,"audio":false}});
        let request = media_request(&props, &image).unwrap();
        assert_eq!(request["input"]["prompt_string"], "<media-random>");
        assert_eq!(
            request["input"]["multimodal_data"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert!(media_request(
            &json!({"media_marker":"","modalities":{"vision":true}}),
            &image
        )
        .is_err());
        assert!(media_request(
            &json!({"media_marker":"marker","modalities":{"vision":false}}),
            &image
        )
        .is_err());
        let invalid = crate::media::MediaPart {
            data: vec![0; 2 * 1024 * 1024 + 1],
            ..image
        };
        assert!(media_request(&props, &invalid).is_err());
    }
    #[test]
    fn profiles_change_with_file_contents_and_inputs_preserve_unicode() {
        let dir = tempfile::tempdir().unwrap();
        let model = dir.path().join("model.gguf");
        let projector = dir.path().join("projector.gguf");
        std::fs::write(&model, b"GGUFmodel-one").unwrap();
        std::fs::write(&projector, b"GGUFprojector-one").unwrap();
        let config = crate::runtime::RuntimeConfig {
            executable: None,
            bundled_engine: false,
            model: Some(model.clone()),
            projector: Some(projector.clone()),
        };
        let original = profile(&config).unwrap();
        assert_eq!(
            original,
            "gemma2-v2:69190d2703a65dc42c7b2dd9638dd5f44476f6cd15ed920a02182284a2738ffe"
        );
        assert_eq!(original, profile(&config).unwrap());
        std::fs::write(&model, b"GGUFmodel-two").unwrap();
        assert_ne!(original, profile(&config).unwrap());
        std::fs::write(&model, b"GGUFmodel-one").unwrap();
        std::fs::write(&projector, b"GGUFprojector-two").unwrap();
        assert_ne!(original, profile(&config).unwrap());
        let input = document_input(&"🦀".repeat(100), &"语".repeat(1000));
        assert!(input.len() < 1900);
        assert!(input.starts_with("title: "));
        assert!(input.ends_with('语'));
    }
    #[test]
    fn accepts_only_one_finite_normalized_768_vector() {
        let mut vector = vec![0.0; DIMENSIONS];
        vector[0] = 1.0;
        assert_eq!(
            parse_vector(&json!({"data":[{"embedding":vector}]})).unwrap(),
            vector
        );
        for invalid in [
            json!({"data":[]}),
            json!({"data":[{"embedding":[1.0]}]}),
            json!({"data":[{"embedding":vec![0.0; DIMENSIONS]}]}),
            json!({"data":[{"embedding":vec![1e100; DIMENSIONS]}]}),
            json!({"data":[{"embedding":vector},{"embedding":vector}]}),
        ] {
            assert!(parse_vector(&invalid).is_err());
        }
    }
    #[tokio::test]
    async fn cancelled_index_batch_keeps_commits_and_resumes_only_pending_passages() {
        use crate::operation_jobs::{OperationJobs, CANCELLED};
        use std::{sync::Arc, time::Duration};
        use tokio::sync::Semaphore;
        let scratch = tempfile::tempdir().unwrap();
        let database = scratch.path().join("test.sqlite");
        let mut store = crate::store::Store::open(&database).unwrap();
        for n in 0..10 {
            store
                .import_text(
                    &format!("Doc {n}"),
                    "txt",
                    &format!("Unique passage {n} about indexing."),
                )
                .unwrap();
        }
        let jobs = OperationJobs::default();
        let gate = Arc::new(Semaphore::new(1));
        let id = jobs.begin().unwrap();
        let key = id.clone();
        let owner = jobs.clone();
        let db = database.clone();
        let shared_gate = gate.clone();
        let (entered, second_request) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let mut call = 0;
            let mut entered = Some(entered);
            owner
                .run(
                    &key,
                    shared_gate,
                    index_batch_with(&db, "fixture-profile", move |_, part| {
                        assert!(part.is_none());
                        call += 1;
                        let n = call;
                        let signal = if n == 2 { entered.take() } else { None };
                        async move {
                            if let Some(signal) = signal {
                                signal.send(()).unwrap();
                                std::future::pending::<()>().await;
                            }
                            let mut vector = vec![0.0; DIMENSIONS];
                            vector[0] = 1.0;
                            Ok(vector)
                        }
                    }),
                )
                .await
        });
        tokio::time::timeout(Duration::from_secs(1), second_request)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(store.embedding_counts("fixture-profile").unwrap(), (1, 10));
        assert!(jobs.cancel(&id).unwrap());
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), task)
                .await
                .unwrap()
                .unwrap()
                .unwrap_err(),
            CANCELLED
        );
        assert_eq!(gate.available_permits(), 1);
        drop(store);
        let reopened = crate::store::Store::open(&database).unwrap();
        assert_eq!(
            reopened.embedding_counts("fixture-profile").unwrap(),
            (1, 10)
        );
        let completed_id = reopened
            .semantic_search("fixture-profile", &{
                let mut vector = vec![0.0; DIMENSIONS];
                vector[0] = 1.0;
                vector
            })
            .unwrap()[0]
            .id
            .clone();
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        for expected in [9, 10] {
            let next = jobs.begin().unwrap();
            let count = calls.clone();
            let progress = jobs
                .run(
                    &next,
                    gate.clone(),
                    index_batch_with(&database, "fixture-profile", |chunk, _| {
                        assert_ne!(chunk.id, completed_id);
                        count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        std::future::ready(Ok({
                            let mut vector = vec![0.0; DIMENSIONS];
                            vector[0] = 1.0;
                            vector
                        }))
                    }),
                )
                .await
                .unwrap();
            assert_eq!((progress.indexed, progress.total), (expected, 10));
        }
        assert_eq!(calls.load(std::sync::atomic::Ordering::Relaxed), 9);
        assert_eq!(
            index_progress(&database, "fixture-profile")
                .unwrap()
                .indexed,
            10
        );
    }
}
