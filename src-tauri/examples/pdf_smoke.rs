//! Exercise the production PDF helper, store, embeddings and grounded answers.
use cachella_lib::{
    embedding, inference, pdf, process_guard,
    runtime::{Runtime, RuntimeConfig},
    store::Store,
};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(code) = process_guard::entry().or_else(pdf::entry) {
        std::process::exit(code);
    }
    run()
}

async fn ready(
    runtime: &mut Runtime,
    connection: &cachella_lib::runtime::Connection,
) -> Result<(), String> {
    tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            if !runtime.status()?.running {
                return Err("Local engine exited during startup".into());
            }
            if inference::status_authenticated(
                &connection.endpoint,
                Some(&connection.token),
                Some(&connection.certificate),
            )
            .await
            .is_ok_and(|status| status.connected)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        runtime.mark_ready()
    })
    .await
    .map_err(|_| "Local engine startup timed out".to_string())?
}

#[tokio::main]
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.len() != 7 {
        return Err("Usage: pdf_smoke engine generation.gguf embedding.gguf projector.gguf input.pdf report.json".into());
    }
    let started = Instant::now();
    let prepared = pdf::prepare(Path::new(&args[5])).await?;
    assert_eq!(prepared.pages.len(), 3);
    assert!(prepared.pages[1].text.is_empty());
    let temporary = tempfile::tempdir()?;
    let database = temporary.path().join("pdf.sqlite");
    let mut db = Store::open(&database)?;
    let document = db.import_pdf("Orion project brief", &prepared)?;
    assert_eq!(document.chunks, 2);
    let keyword = db.search("nine dollars")?;
    assert_eq!(keyword[0].page_number, Some(3));
    println!("PASS: guarded PDF import preserves 3 physical pages and searches page 3 after a blank page.");

    let config = RuntimeConfig {
        executable: Some(PathBuf::from(&args[1])),
        bundled_engine: false,
        model: Some(PathBuf::from(&args[3])),
        projector: Some(PathBuf::from(&args[4])),
    };
    let profile = embedding::profile(&config)?;
    let mut embeddings = Runtime::new(config);
    let connection = embeddings.start_embedding()?;
    ready(&mut embeddings, &connection).await?;
    for hit in db.pending_embeddings(&profile, 32)? {
        let vector = embedding::embed(
            &connection,
            &embedding::document_input(&hit.title, &hit.text),
        )
        .await?;
        db.save_embedding(&hit.id, &profile, &vector)?;
    }
    let question = "What is the Orion infrastructure budget per month?";
    let query = embedding::query(&connection, question).await?;
    let semantic = db.semantic_search(&profile, &query)?;
    assert_eq!(semantic[0].page_number, Some(3));
    embeddings.stop()?;
    println!("PASS: actual local embeddings rank budget evidence on page 3 first.");
    drop(db);
    let mut db = Store::open(&database)?;
    assert_eq!(db.pdf_pages(&document.id)?, prepared.pages);
    assert_eq!(db.embedding_counts(&profile)?, (2, 2));
    assert_eq!(
        db.asset_range(&document.id, 0, prepared.source.len())?,
        prepared.source
    );
    println!(
        "PASS: source bytes, blank page, page citations and 2 vectors survive database reopen."
    );

    let mut generation = Runtime::new(RuntimeConfig {
        executable: Some(PathBuf::from(&args[1])),
        bundled_engine: false,
        model: Some(PathBuf::from(&args[2])),
        projector: None,
    });
    let connection = generation.start()?;
    ready(&mut generation, &connection).await?;
    let answer = inference::answer_authenticated(
        &connection.endpoint,
        question,
        db.search(question)?,
        true,
        Some(&connection.token),
        Some(&connection.certificate),
    )
    .await?;
    assert!(!answer.answer.insufficient_evidence);
    assert!(answer.sources.iter().any(
        |source| source.page_number == Some(3) && answer.answer.source_ids.contains(&source.id)
    ));
    let wording = answer.answer.answer.to_lowercase();
    assert!(wording.contains("nine") || wording.contains('9'));
    generation.stop()?;
    println!(
        "PASS: actual Q4 answer states the synthetic budget and cites the stored page-3 evidence."
    );
    let report = serde_json::json!({"pages":prepared.pages,"keyword_hits":keyword,"semantic_hits":semantic,"answer":answer,"total_elapsed_ms":started.elapsed().as_millis(),"scope":"Synthetic selectable-text PDF with a blank physical page; no OCR or original-layout rendering."});
    std::fs::write(Path::new(&args[6]), serde_json::to_vec_pretty(&report)?)?;
    db.delete(&document.id)?;
    assert!(db.pdf_pages(&document.id).is_err());
    assert!(db.search("Orion")?.is_empty());
    assert_eq!(db.embedding_counts(&profile)?, (0, 0));
    println!(
        "PASS: deleting PDF removes pages, search evidence and vectors; both engines stopped."
    );
    Ok(())
}
