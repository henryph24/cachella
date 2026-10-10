//! Controlled production-request evaluation; answer quality requires manual review.
use cachella_lib::{
    inference, media,
    runtime::{Runtime, RuntimeConfig},
    store::Store,
};
use serde_json::json;
use std::{path::PathBuf, time::Duration};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(code) = cachella_lib::process_guard::entry() {
        std::process::exit(code);
    }
    run()
}

#[tokio::main]
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 7 && args.len() != 8 {
        return Err(
            "Usage: media_order_probe engine model projector ffmpeg video report.json [question]"
                .into(),
        );
    }
    let temp = tempfile::tempdir()?;
    let mut store = Store::open(&temp.path().join("probe.sqlite"))?;
    let video = PathBuf::from(&args[5]);
    let prepared = media::prepare(&PathBuf::from(&args[4]), &video).await?;
    let title = video
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("Invalid video filename")?;
    store.import_media(title, &prepared)?;
    let hits = store.pending_embeddings("order-probe", 4)?;
    let evidence = inference::load_answer_media(&store, &hits)?;
    if hits.len() != 2
        || evidence.values().filter(|p| p.mime == "image/png").count() != 1
        || evidence.values().filter(|p| p.mime == "audio/wav").count() != 1
    {
        return Err("Probe requires exactly one frame and one audio segment".into());
    }
    let mut runtime = Runtime::new(RuntimeConfig {
        executable: Some(PathBuf::from(&args[1])),
        bundled_engine: false,
        model: Some(PathBuf::from(&args[2])),
        projector: Some(PathBuf::from(&args[3])),
    });
    let connection = runtime.start()?;
    tokio::time::timeout(Duration::from_secs(120), async {
        loop {
            if !runtime.status()?.running {
                return Err("Generation process exited".to_string());
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
    let question = if let Some(question) = args.get(7) {
        question.to_str().ok_or("Question must be UTF-8")?
    } else {
        "What does the video show, and what monthly cloud budget does it mention?"
    };
    let output = PathBuf::from(&args[6]);
    let mut report = json!({"scope":"single synthetic video; production request path; manual quality review required",
        "question":question,"cases":[],"engine_stopped":false});
    for cache in [false, true] {
        for audio_first in [true, false] {
            for repeat in 1..=2 {
                let mut sources = hits.clone();
                sources.sort_by_key(|h| (evidence[&h.id].mime == "audio/wav") != audio_first);
                let order: Vec<_> = sources
                    .iter()
                    .map(|h| evidence[&h.id].mime.clone())
                    .collect();
                let result = inference::answer_with_media_authenticated(
                    &connection.endpoint,
                    question,
                    sources,
                    cache,
                    Some(&connection.token),
                    Some(&connection.certificate),
                    Some(&evidence),
                )
                .await;
                let result = match result {
                    Ok(answer) => json!({"ok":true,"answer":answer}),
                    Err(error) => json!({"ok":false,"error":error}),
                };
                let case = json!({"cache":cache,"audio_first":audio_first,"repeat":repeat,"order":order,"result":result});
                println!("{case}");
                report["cases"].as_array_mut().unwrap().push(case);
                std::fs::write(&output, serde_json::to_vec_pretty(&report)?)?;
            }
        }
    }
    runtime.stop()?;
    let port = reqwest::Url::parse(&connection.endpoint)?
        .port()
        .ok_or("No port")?;
    if runtime.status()?.running || std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
        return Err("Generation engine survived stop".into());
    }
    report["engine_stopped"] = json!(true);
    std::fs::write(&output, serde_json::to_vec_pretty(&report)?)?;
    println!("PASS: 8 request outcomes recorded; generation engine stopped.");
    Ok(())
}
