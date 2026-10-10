//! Exercise production index batches and query cancellation with real local embeddings.
use cachella_lib::{
    embedding, inference, media,
    operation_jobs::{OperationJobs, CANCELLED},
    runtime::{Connection, Runtime, RuntimeConfig},
    store::Store,
};
use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Semaphore;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(code) = cachella_lib::process_guard::entry() {
        std::process::exit(code);
    }
    run()
}

async fn slots(
    client: &reqwest::Client,
    connection: &Connection,
) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(client
        .get(format!("{}/slots", connection.endpoint))
        .bearer_auth(&connection.token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?)
}

fn processing(slots: &Value) -> Result<bool, Box<dyn std::error::Error>> {
    let list = slots.as_array().ok_or("Invalid slots response")?;
    if list.len() != 1 {
        return Err("Expected one runtime slot".into());
    }
    list[0]
        .get("is_processing")
        .and_then(Value::as_bool)
        .ok_or("Missing slot state".into())
}

#[tokio::main]
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 7 {
        return Err(
            "Usage: cancel_index_smoke engine model projector ffmpeg fixtures report.json".into(),
        );
    }
    let config = RuntimeConfig {
        executable: Some(PathBuf::from(&args[1])),
        model: Some(PathBuf::from(&args[2])),
        projector: Some(PathBuf::from(&args[3])),
        bundled_engine: false,
    };
    let profile = embedding::profile(&config)?;
    let mut runtime = Runtime::new(config);
    let connection = runtime.start_embedding()?;
    tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            if !runtime.status()?.running {
                return Err("Engine exited".to_string());
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
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await??;
    runtime.mark_ready()?;
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .tls_built_in_root_certs(false)
        .add_root_certificate(reqwest::Certificate::from_pem(
            connection.certificate.as_bytes(),
        )?)
        .timeout(Duration::from_secs(2))
        .build()?;
    let scratch = tempfile::tempdir()?;
    let database = scratch.path().join("index.sqlite");
    let mut store = Store::open(&database)?;
    for number in 1..=10 {
        store.import_text(
            &format!("Synthetic budget {number}"),
            "txt",
            &format!(
                "Synthetic passage {number}. The Orion monthly cloud allowance is nine dollars. {}",
                "Employees use their own computers for local inference. ".repeat(24)
            ),
        )?;
    }
    drop(store);
    let jobs = OperationJobs::default();
    let gate = Arc::new(Semaphore::new(1));
    let id = jobs.begin()?;
    let (owner, key, shared_gate, conn, db, space) = (
        jobs.clone(),
        id.clone(),
        gate.clone(),
        connection.clone(),
        database.clone(),
        profile.clone(),
    );
    let task = tokio::spawn(async move {
        owner
            .run(
                &key,
                shared_gate,
                embedding::index_batch(&db, &conn, &space),
            )
            .await
    });
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            let count = embedding::index_progress(&database, &profile)?;
            if count.indexed > 0 && processing(&slots(&client, &connection).await?)? {
                return Ok::<_, Box<dyn std::error::Error>>(());
            }
            if task.is_finished() {
                return Err(
                    "Batch finished before observing a committed vector and active slot".into(),
                );
            }
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    })
    .await??;
    let started = Instant::now();
    assert!(jobs.cancel(&id)?);
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(2), task)
            .await??
            .unwrap_err(),
        CANCELLED
    );
    let index_cancel_ms = started.elapsed().as_millis();
    assert_eq!(gate.available_permits(), 1);
    let saved = embedding::index_progress(&database, &profile)?;
    assert!(saved.indexed > 0 && saved.indexed < saved.total);
    println!("PASS: active real index cancelled in {index_cancel_ms} ms; {} of {} committed vectors retained after reopening.", saved.indexed, saved.total);
    let committed: Vec<(String, Vec<u8>)> = {
        let db = rusqlite::Connection::open(&database)?;
        let mut statement =
            db.prepare("SELECT chunk_id, vector FROM embeddings ORDER BY chunk_id")?;
        let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        rows.collect::<Result<_, _>>()?
    };
    let mut batches = 0;
    loop {
        let next = jobs.begin()?;
        assert!(!jobs.cancel(&id)?);
        let count = jobs
            .run(
                &next,
                gate.clone(),
                embedding::index_batch(&database, &connection, &profile),
            )
            .await?;
        batches += 1;
        if count.indexed == count.total {
            break;
        }
        if batches > 4 {
            return Err("Index did not converge".into());
        }
    }
    let db = rusqlite::Connection::open(&database)?;
    for (chunk, vector) in &committed {
        let after: Vec<u8> = db.query_row(
            "SELECT vector FROM embeddings WHERE chunk_id = ?1",
            [chunk],
            |row| row.get(0),
        )?;
        assert_eq!(&after, vector);
    }
    println!(
        "PASS: resumed {batches} batches; preserved {} committed vector bytes.",
        committed.len()
    );
    drop(db);
    let mut store = Store::open(&database)?;
    let mut media_parts = 0;
    for name in ["red-circle.png", "budget.wav", "budget-video.mp4"] {
        let prepared = media::prepare(
            std::path::Path::new(&args[4]),
            &PathBuf::from(&args[5]).join(name),
        )
        .await?;
        media_parts += store.import_media(name, &prepared)?.chunks;
    }
    drop(store);
    loop {
        let next = jobs.begin()?;
        let count = jobs
            .run(
                &next,
                gate.clone(),
                embedding::index_batch(&database, &connection, &profile),
            )
            .await?;
        if count.indexed == count.total {
            break;
        }
    }
    println!("PASS: production batch path indexed {media_parts} image/audio/video parts.");
    let query_id = jobs.begin()?;
    let (owner, key, shared_gate, conn) = (
        jobs.clone(),
        query_id.clone(),
        gate.clone(),
        connection.clone(),
    );
    let task = tokio::spawn(async move {
        owner
            .run(
                &key,
                shared_gate,
                embedding::query(&conn, &"What is our monthly cloud budget? ".repeat(48)),
            )
            .await
    });
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if processing(&slots(&client, &connection).await?)? {
                return Ok::<_, Box<dyn std::error::Error>>(());
            }
            if task.is_finished() {
                return Err("Query completed before observing active slot".into());
            }
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    })
    .await??;
    let started = Instant::now();
    assert!(jobs.cancel(&query_id)?);
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(2), task)
            .await??
            .unwrap_err(),
        CANCELLED
    );
    let query_cancel_ms = started.elapsed().as_millis();
    assert_eq!(gate.available_permits(), 1);
    println!(
        "PASS: active real query cancelled in {query_cancel_ms} ms and operation permit released."
    );
    let next = jobs.begin()?;
    let vector = jobs
        .run(
            &next,
            gate.clone(),
            embedding::query(&connection, "What is the Orion monthly cloud budget?"),
        )
        .await?;
    let hits = Store::open(&database)?.semantic_search(&profile, &vector)?;
    assert!(hits
        .first()
        .is_some_and(|hit| hit.title.starts_with("Synthetic budget")));
    let final_count = embedding::index_progress(&database, &profile)?;
    assert_eq!(final_count.indexed, final_count.total);
    println!(
        "PASS: semantic retry ranked budget evidence first; {} of {} vectors persisted.",
        final_count.indexed, final_count.total
    );
    runtime.stop()?;
    assert!(!runtime.status()?.running);
    println!("PASS: managed embedding runtime stopped.");
    std::fs::write(
        &args[6],
        serde_json::to_vec_pretty(&json!({
            "index_cancel_ms": index_cancel_ms, "query_cancel_ms": query_cancel_ms,
            "retained_vectors": committed.len(), "resume_batches": batches, "media_parts": media_parts,
            "final_indexed": final_count.indexed, "final_total": final_count.total,
            "retry_first_title": hits[0].title, "runtime_stopped": true,
            "scope": "Actual managed embedding engine, production index batch and query functions, isolated temporary SQLite. Timings are workload observations. Desktop IPC and UI require separate checks."
        }))?,
    )?;
    Ok(())
}
