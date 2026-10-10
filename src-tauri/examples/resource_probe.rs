//! JSON-lines workload for evaluation/resources/measure.py. Uses production settings.
use cachella_lib::{
    embedding, inference,
    media::MediaPart,
    runtime::{Connection, Runtime, RuntimeConfig},
    store::Hit,
};
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    path::PathBuf,
    time::{Duration, Instant},
};

fn emit(event: Value) {
    println!("{event}");
    std::io::stdout().flush().expect("flush measurement event");
}

async fn phase(name: &str) {
    emit(json!({"event":"phase","name":name}));
    // Provide at least two 250 ms sampling opportunities at each stable boundary.
    tokio::time::sleep(Duration::from_millis(600)).await;
}

async fn start(runtime: &mut Runtime, mode: &str) -> Result<Connection, String> {
    let started = Instant::now();
    let connection = if mode == "embedding" {
        runtime.start_embedding()?
    } else {
        runtime.start()?
    };
    tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            if !runtime.status()?.running {
                return Err("Engine exited during startup".into());
            }
            if inference::status_authenticated(
                &connection.endpoint,
                Some(&connection.token),
                Some(&connection.certificate),
            )
            .await
            .is_ok_and(|s| s.connected)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        inference::verify_authentication(&connection.endpoint, Some(&connection.certificate))
            .await?;
        if mode == "embedding" {
            embedding::query(&connection, "startup compatibility check").await?;
        }
        runtime.mark_ready()?;
        Ok::<_, String>(())
    })
    .await
    .map_err(|_| "Startup timed out".to_string())??;
    emit(json!({"event":"startup","mode":mode,"elapsed_ms":started.elapsed().as_millis()}));
    Ok(connection)
}

fn evidence(long: bool) -> Vec<Hit> {
    let facts = [
        "The Atlas launch moved to Friday because the security review found a permission bug. Priya owns the fix.",
        "Release requires passing permission regression tests and security approval. The project cloud budget is eight dollars per month.",
        "The workspace keeps documents and indexes on the employee computer. Answer citations identify supporting source passages.",
        "The product team approved the revised Friday launch. The next review checks whether Priya's permission fix passes its tests.",
    ];
    facts
        .iter()
        .enumerate()
        .take(if long { 4 } else { 1 })
        .map(|(i, text)| Hit {
            page_number: None,
            id: format!("resource-source-{i}"),
            document_id: format!("resource-doc-{i}"),
            title: format!("Synthetic project record {i}"),
            ordinal: 0,
            text: if long {
                text.repeat(20).chars().take(1800).collect()
            } else {
                (*text).into()
            },
            kind: "txt".into(),
            start_ms: None,
            end_ms: None,
        })
        .collect()
}

async fn generate(connection: &Connection, scenario: &str) -> Result<(), String> {
    for long in [false, true] {
        for repetition in 0..3 {
            for cache in [false, true] {
                let result = inference::answer_authenticated(
                    &connection.endpoint,
                    "Why did the Atlas launch move, and who owns the fix?",
                    evidence(long),
                    cache,
                    Some(&connection.token),
                    Some(&connection.certificate),
                )
                .await?;
                if result.prompt_tokens.is_none() || result.completion_tokens.is_none() {
                    return Err("Engine did not return token accounting".into());
                }
                emit(
                    json!({"event":"generation","scenario":scenario,"long_evidence":long,
                    "repetition":repetition,"cache_enabled":cache,"result":result}),
                );
            }
        }
    }
    Ok(())
}

async fn generate_media(
    connection: &Connection,
    scenario: &str,
    fixtures: &std::path::Path,
) -> Result<(), String> {
    let mut sources = Vec::new();
    let mut media = inference::AnswerMedia::new();
    for (name, mime, kind) in [
        ("red-circle.png", "image/png", "image"),
        ("budget.wav", "audio/wav", "audio"),
    ] {
        let mut data = Vec::new();
        std::fs::File::open(fixtures.join(name))
            .map_err(|e| e.to_string())?
            .take(inference::MAX_ANSWER_MEDIA_PART_BYTES as u64 + 1)
            .read_to_end(&mut data)
            .map_err(|e| e.to_string())?;
        if data.len() > inference::MAX_ANSWER_MEDIA_PART_BYTES {
            return Err("Media fixture exceeds the production answer limit".into());
        }
        sources.push(Hit {
            id: name.into(),
            document_id: name.into(),
            title: name.into(),
            ordinal: 0,
            text: String::new(),
            kind: kind.into(),
            start_ms: None,
            end_ms: None,
            page_number: None,
        });
        media.insert(
            name.into(),
            MediaPart {
                mime: mime.into(),
                data,
                start_ms: None,
                end_ms: None,
                label: String::new(),
            },
        );
    }
    for (case, question, selection) in [
        (
            "image",
            "What shape and color are visible?",
            vec![sources[0].clone()],
        ),
        (
            "audio",
            "What monthly cloud budget is stated?",
            vec![sources[1].clone()],
        ),
        (
            "combined_media",
            "What shape and color are visible, and what monthly cloud budget is stated?",
            vec![sources[1].clone(), sources[0].clone()],
        ),
    ] {
        let selected = selection
            .iter()
            .map(|h| (h.id.clone(), media[&h.id].clone()))
            .collect();
        for cache in [false, true] {
            let result = inference::answer_with_media_authenticated(
                &connection.endpoint,
                question,
                selection.clone(),
                cache,
                Some(&connection.token),
                Some(&connection.certificate),
                Some(&selected),
            )
            .await?;
            emit(
                json!({"event":"media_generation","scenario":scenario,"case":case,"cache_enabled":cache,"result":result}),
            );
        }
    }
    Ok(())
}

async fn embed(
    connection: &Connection,
    scenario: &str,
    fixtures: &std::path::Path,
) -> Result<(), String> {
    for repetition in 0..3 {
        let started = Instant::now();
        embedding::query(connection, "What is the monthly cloud budget?").await?;
        emit(
            json!({"event":"embedding","scenario":scenario,"modality":"text","repetition":repetition,"elapsed_ms":started.elapsed().as_millis()}),
        );
        for (name, mime) in [("red-circle.png", "image/png"), ("budget.wav", "audio/wav")] {
            let data = std::fs::read(fixtures.join(name)).map_err(|e| e.to_string())?;
            let part = MediaPart {
                mime: mime.into(),
                data,
                start_ms: None,
                end_ms: None,
                label: name.into(),
            };
            let started = Instant::now();
            embedding::embed_media(connection, &part).await?;
            emit(
                json!({"event":"embedding","scenario":scenario,"modality":mime,"repetition":repetition,"elapsed_ms":started.elapsed().as_millis()}),
            );
        }
    }
    Ok(())
}

fn stop(runtime: &mut Runtime, connection: &Connection) -> Result<(), String> {
    runtime.stop()?;
    let url = reqwest::Url::parse(&connection.endpoint).map_err(|e| e.to_string())?;
    let port = url.port().ok_or("Missing managed port")?;
    if runtime.status()?.running || std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
        return Err("Owned model survived stop".into());
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(code) = cachella_lib::process_guard::entry() {
        std::process::exit(code);
    }
    run()
}

#[tokio::main]
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 7 && args.len() != 8 {
        return Err("Usage: resource_probe generation-engine q4-model embedding-engine embedding-model projector fixtures-dir [generation-projector]".into());
    }
    let mut generation = Runtime::new(RuntimeConfig {
        executable: Some(PathBuf::from(&args[1])),
        bundled_engine: false,
        model: Some(PathBuf::from(&args[2])),
        projector: args.get(7).map(PathBuf::from),
    });
    let mut embeddings = Runtime::new(RuntimeConfig {
        executable: Some(PathBuf::from(&args[3])),
        bundled_engine: false,
        model: Some(PathBuf::from(&args[4])),
        projector: Some(PathBuf::from(&args[5])),
    });
    let fixtures = PathBuf::from(&args[6]);
    phase("baseline").await;
    phase("generation_startup").await;
    let gen = start(&mut generation, "generation").await?;
    phase("generation_only").await;
    generate(&gen, "generation_only").await?;
    if args.len() == 8 {
        phase("media_generation_only").await;
        generate_media(&gen, "generation_only", &fixtures).await?;
    }
    phase("generation_loaded_idle").await;
    stop(&mut generation, &gen)?;
    phase("embedding_profile_hash").await;
    let started = Instant::now();
    let profile = embedding::profile(&embeddings.config)?;
    emit(json!({"event":"profile","profile":profile,"elapsed_ms":started.elapsed().as_millis()}));
    phase("embedding_startup").await;
    let emb = start(&mut embeddings, "embedding").await?;
    phase("embedding_only").await;
    embed(&emb, "embedding_only", &fixtures).await?;
    phase("embedding_loaded_idle").await;
    phase("combined_startup").await;
    let gen = start(&mut generation, "generation").await?;
    phase("combined").await;
    // Desktop serializes these operations; both engine weights remain loaded.
    embed(&emb, "combined", &fixtures).await?;
    generate(&gen, "combined").await?;
    if args.len() == 8 {
        phase("combined_media_generation").await;
        generate_media(&gen, "combined", &fixtures).await?;
    }
    phase("combined_loaded_idle").await;
    stop(&mut generation, &gen)?;
    stop(&mut embeddings, &emb)?;
    phase("stopped").await;
    emit(
        json!({"event":"complete","engines_stopped":true,"generation_requests":24,"embedding_requests":18,"media_generation_requests":if args.len() == 8 {12} else {0}}),
    );
    Ok(())
}
