use cachella_lib::{
    embedding, inference,
    runtime::{Runtime, RuntimeConfig},
    store::Store,
};
use std::{path::PathBuf, time::Duration};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(code) = cachella_lib::process_guard::entry() {
        std::process::exit(code);
    }
    run()
}

#[tokio::main]
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("Usage: semantic_smoke llama-server embedding.gguf projector.gguf".into());
    }
    let config = RuntimeConfig {
        executable: Some(PathBuf::from(&args[1])),
        bundled_engine: false,
        model: Some(PathBuf::from(&args[2])),
        projector: Some(PathBuf::from(&args[3])),
    };
    let profile = embedding::profile(&config)?;
    let mut runtime = Runtime::new(config);
    let connection = runtime.start_embedding()?;
    tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            if !runtime.status()?.running {
                return Err("Embedding engine exited".to_string());
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
    println!("PASS: owned embedding engine ready with pinned TLS and mandatory authentication.");
    let dir = tempfile::tempdir()?;
    let database = dir.path().join("semantic.sqlite");
    let mut db = Store::open(&database)?;
    let budget = db.import_text(
        "Cloud costs",
        "txt",
        "The project infrastructure allowance is eight dollars each month.",
    )?;
    db.import_text(
        "Lunch",
        "txt",
        "Lunch is tomato soup with fresh bread and green salad.",
    )?;
    let chunks = db.pending_embeddings(&profile, 8)?;
    for chunk in &chunks {
        let vector = embedding::embed(
            &connection,
            &embedding::document_input(&chunk.title, &chunk.text),
        )
        .await?;
        db.save_embedding(&chunk.id, &profile, &vector)?;
    }
    if db.embedding_counts(&profile)? != (2, 2) {
        return Err("Index count mismatch".into());
    }
    drop(db);
    let mut db = Store::open(&database)?;
    if !db.pending_embeddings(&profile, 8)?.is_empty() {
        return Err("Restart lost indexed state".into());
    }
    println!(
        "PASS: 2 normalized 768-dimensional vectors persisted; reopened index has no pending work."
    );
    let question = "What is our monthly cloud budget?";
    let vector = embedding::query(&connection, question).await?;
    let hits = db.semantic_search(&profile, &vector)?;
    if hits.first().map(|h| h.document_id.as_str()) != Some(budget.id.as_str()) {
        return Err("Relevant document was not ranked first".into());
    }
    println!("PASS: paraphrased budget query ranks cloud costs first among 2 sources.");
    db.delete(&budget.id)?;
    if db.embedding_counts(&profile)? != (1, 1)
        || db
            .semantic_search(&profile, &vector)?
            .iter()
            .any(|h| h.document_id == budget.id)
    {
        return Err("Deleted evidence survived".into());
    }
    println!("PASS: deletion removes stored vector and semantic evidence.");
    let port = reqwest::Url::parse(&connection.endpoint)?
        .port()
        .ok_or("Missing port")?;
    runtime.stop()?;
    if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
        return Err("Embedding listener survived stop".into());
    }
    println!("PASS: owned embedding engine stopped and its port closed.");
    Ok(())
}
