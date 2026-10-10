//! Real-model integration for the same decoder/store/embedding/protocol used by desktop.
use cachella_lib::{
    asset, embedding, inference, media,
    runtime::{Connection, Runtime, RuntimeConfig},
    store::{Hit, Store},
};
use serde_json::json;
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(code) = cachella_lib::process_guard::entry() {
        std::process::exit(code);
    }
    run()
}

#[tokio::main]
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let single_resident = args.len() == 10 && args[9] == "--single-resident";
    if args.len() != 7 && args.len() != 9 && !single_resident {
        return Err("Usage: multimodal_smoke llama-server model.gguf projector.gguf ffmpeg fixtures-dir report.json [generation.gguf generation-projector.gguf [--single-resident]]".into());
    }
    let config = RuntimeConfig {
        executable: Some(PathBuf::from(&args[1])),
        bundled_engine: false,
        model: Some(PathBuf::from(&args[2])),
        projector: Some(PathBuf::from(&args[3])),
    };
    let profile = embedding::profile(&config)?;
    let mut runtime = Runtime::new(config);
    let mut connection = runtime.start_embedding()?;
    ready(&mut runtime, &connection).await?;
    let temp = tempfile::tempdir()?;
    let database = temp.path().join("multimodal.sqlite");
    let mut db = Store::open(&database)?;
    let mut imports = vec![];
    for name in [
        "red-circle.png",
        "blue-square.png",
        "budget.wav",
        "lunch.wav",
        "budget-video.mp4",
    ] {
        let started = Instant::now();
        let prepared = media::prepare(Path::new(&args[4]), &Path::new(&args[5]).join(name)).await?;
        let doc = db.import_media(name, &prepared)?;
        let preview = asset::serve(&db, &format!("/{}", doc.id), Some("bytes=0-15"), "GET");
        if preview.status != 206 || preview.body != prepared.source[..16] {
            return Err(format!("Preview range mismatch for {name}").into());
        }
        imports.push(json!({"file":name,"kind":doc.kind,"parts":doc.chunks,"preview_bytes":doc.bytes,"decode_and_store_ms":started.elapsed().as_millis()}));
        println!(
            "PASS: decoded, persisted and range-served {name} ({} parts).",
            doc.chunks
        );
    }
    db.import_text(
        "Launch.txt",
        "txt",
        "Priya owns the permission bug fix for the Friday launch.",
    )?;
    let total = db.embedding_counts(&profile)?.1;
    let started = Instant::now();
    for chunk in db.pending_embeddings(&profile, 32)? {
        let vector = if let Some(part) = db.media_part(&chunk.id)? {
            embedding::embed_media(&connection, &part).await?
        } else {
            embedding::embed(
                &connection,
                &embedding::document_input(&chunk.title, &chunk.text),
            )
            .await?
        };
        db.save_embedding(&chunk.id, &profile, &vector)?;
    }
    if db.embedding_counts(&profile)? != (total, total) {
        return Err("Index was incomplete".into());
    }
    let indexing_ms = started.elapsed().as_millis();
    drop(db);
    let mut db = Store::open(&database)?;
    if !db.pending_embeddings(&profile, 32)?.is_empty() {
        return Err("Reopened library lost embedding progress".into());
    }
    let queries = [
        (
            "image",
            "A red circle on a white background",
            "red-circle.png",
            "blue-square.png",
            None,
        ),
        (
            "audio",
            "What is the monthly cloud budget?",
            "budget.wav",
            "lunch.wav",
            None,
        ),
        (
            "video_frame",
            "A red circle on a white background",
            "budget-video.mp4",
            "blue-square.png",
            Some("image/png"),
        ),
        (
            "video_audio",
            "What is the monthly cloud budget?",
            "budget-video.mp4",
            "lunch.wav",
            Some("audio/wav"),
        ),
        (
            "text",
            "Who owns the permission bug fix?",
            "Launch.txt",
            "lunch.wav",
            None,
        ),
    ];
    let mut prepared_queries = Vec::new();
    if single_resident {
        for (_, question, _, _, _) in &queries {
            let vector = embedding::query(&connection, question).await?;
            prepared_queries.push(db.semantic_search(&profile, &vector)?);
        }
        runtime.stop()?;
        closed(&connection)?;
    }
    let mut checks = vec![];
    let mut generation = Runtime::default();
    let generation_connection = if args.len() >= 9 {
        generation.configure(RuntimeConfig {
            executable: Some(PathBuf::from(&args[1])),
            bundled_engine: false,
            model: Some(PathBuf::from(&args[7])),
            projector: Some(PathBuf::from(&args[8])),
        })?;
        let generation_connection = generation.start()?;
        ready(&mut generation, &generation_connection).await?;
        if single_resident && runtime.status()?.running {
            return Err("Embedding engine remained loaded during generation".into());
        }
        Some(generation_connection)
    } else {
        None
    };
    let mut answers = vec![];
    for (index, (case, question, expected, contrast, part_mime)) in queries.into_iter().enumerate()
    {
        let hits = if single_resident {
            prepared_queries[index].clone()
        } else {
            let vector = embedding::query(&connection, question).await?;
            db.semantic_search(&profile, &vector)?
        };
        let expected_position = hits.iter().position(|h| {
            h.title == expected
                && part_mime.is_none_or(|mime| {
                    db.media_part(&h.id)
                        .ok()
                        .flatten()
                        .is_some_and(|p| p.mime == mime)
                })
        });
        let contrast_position = hits.iter().position(|h| h.title == contrast);
        let passed = matches!((expected_position,contrast_position),(Some(a),Some(b)) if a<b);
        checks.push(json!({"case":case,"query":question,"passed":passed,"expected_position":expected_position,"contrast_position":contrast_position,"ranking":hits.iter().map(|h|json!({"source":h.title,"kind":h.kind,"start_ms":h.start_ms,"end_ms":h.end_ms})).collect::<Vec<_>>() }));
        if !passed {
            return Err(format!(
                "Retrieval failed for {case}: {expected_position:?} versus {contrast_position:?}"
            )
            .into());
        }
        println!("PASS: {case} retrieval ranks expected content above contrast.");
        if let Some(connection) = &generation_connection {
            let media = inference::load_answer_media(&db, &hits)?;
            let answer = inference::answer_with_media_authenticated(
                &connection.endpoint,
                question,
                hits,
                true,
                Some(&connection.token),
                Some(&connection.certificate),
                Some(&media),
            )
            .await?;
            answers.push(json!({"case":case,"question":question,"result":answer}));
            println!(
                "PASS: {case} retrieval-to-answer request completed with validated citations."
            );
        }
    }
    let all: Vec<Hit> = db.pending_embeddings("unindexed-profile", 32)?;
    if let Some(connection) = &generation_connection {
        for question in [
            "What does the video show, and what monthly cloud budget does it mention?",
            "What annual company revenue is stated in the video?",
        ] {
            let video_hits: Vec<_> = all
                .iter()
                .filter(|hit| hit.kind == "video")
                .cloned()
                .collect();
            let media = inference::load_answer_media(&db, &video_hits)?;
            let answer = inference::answer_with_media_authenticated(
                &connection.endpoint,
                question,
                video_hits,
                true,
                Some(&connection.token),
                Some(&connection.certificate),
                Some(&media),
            )
            .await?;
            answers.push(json!({"case":"video_only","question":question,"result":answer}));
            println!("PASS: video-only stored frame/audio evidence request completed with validated citations.");
        }
    }
    let video = all
        .iter()
        .find(|h| h.kind == "video")
        .ok_or("Missing video source")?;
    if video.start_ms.is_none() || video.end_ms.is_none() {
        return Err("Video hit lost timestamps".into());
    }
    let video_id = video.document_id.clone();
    let media_only = all.into_iter().filter(|h| h.kind != "txt").collect();
    let answer = inference::answer(
        "http://127.0.0.1:1",
        "What is in these media?",
        media_only,
        true,
    )
    .await?;
    if !answer.answer.insufficient_evidence || !answer.answer.source_ids.is_empty() {
        return Err("Text generator treated media labels as evidence".into());
    }
    generation.stop()?;
    if let Some(connection) = generation_connection {
        let port = reqwest::Url::parse(&connection.endpoint)?
            .port()
            .ok_or("No generation port")?;
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return Err("Generation listener survived stop".into());
        }
    }
    if single_resident {
        if generation.status()?.running {
            return Err("Generation remained loaded before embedding restart".into());
        }
        connection = runtime.start_embedding()?;
        ready(&mut runtime, &connection).await?;
        if embedding::profile(&runtime.config)? != profile
            || db.embedding_counts(&profile)? != (total, total)
        {
            return Err(
                "Embedding profile or persisted index changed across model switching".into(),
            );
        }
        let vector = embedding::query(&connection, "What is the monthly cloud budget?").await?;
        let hits = db.semantic_search(&profile, &vector)?;
        if hits
            .first()
            .is_none_or(|h| h.title != "budget.wav" && h.title != "budget-video.mp4")
        {
            return Err("Semantic retrieval failed after embedding restart".into());
        }
        println!("PASS: single-resident switch and embedding restart preserve the semantic index.");
    }
    db.delete(&video_id)?;
    if asset::serve(&db, &format!("/{video_id}"), None, "GET").status != 404 {
        return Err("Deleted media preview remained available".into());
    }
    runtime.stop()?;
    let port = reqwest::Url::parse(&connection.endpoint)?
        .port()
        .ok_or("No port")?;
    if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
        return Err("Engine listener survived stop".into());
    }
    let report = json!({"scope":"Synthetic macOS integration; no general retrieval quality or performance claim","single_resident":single_resident,"embedding_restarted":single_resident,"profile":profile,"imports":imports,"indexed_parts":total,"indexing_ms":indexing_ms,"checks":checks,"passed":checks.len(),"total":checks.len(),"answers":answers,"persistence":true,"timestamp_metadata":true,"text_only_generation_guard":true,"delete_preview":true,"engine_stopped":true});
    std::fs::write(&args[6], serde_json::to_vec_pretty(&report)?)?;
    println!("PASS: persistence, timestamp metadata, text-only generation guard, deletion and process cleanup verified.");
    Ok(())
}

async fn ready(
    runtime: &mut Runtime,
    connection: &Connection,
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            if !runtime.status()?.running {
                return Err("Engine exited during startup".to_string());
            }
            if inference::status_authenticated(
                &connection.endpoint,
                Some(&connection.token),
                Some(&connection.certificate),
            )
            .await
            .is_ok_and(|s| s.connected)
            {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    })
    .await??;
    inference::verify_authentication(&connection.endpoint, Some(&connection.certificate)).await?;
    runtime.mark_ready()?;
    Ok(())
}

fn closed(connection: &Connection) -> Result<(), Box<dyn std::error::Error>> {
    let port = reqwest::Url::parse(&connection.endpoint)?
        .port()
        .ok_or("No managed port")?;
    if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
        return Err("Previous engine listener survived stop".into());
    }
    Ok(())
}
