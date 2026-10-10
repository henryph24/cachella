//! Verify cancellation against the bundled engine's observed active slot.
use cachella_lib::{
    inference,
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
    if args.len() != 4 {
        return Err("Usage: cancel_answer_smoke engine model report.json".into());
    }
    let mut runtime = Runtime::new(RuntimeConfig {
        executable: Some(PathBuf::from(&args[1])),
        model: Some(PathBuf::from(&args[2])),
        bundled_engine: false,
        projector: None,
    });
    let connection = runtime.start()?;
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
    assert!(!processing(&slots(&client, &connection).await?)?);
    let scratch = tempfile::tempdir()?;
    let mut store = Store::open(&scratch.path().join("test.sqlite"))?;
    for number in 1..=4 {
        store.import_text(
            &format!("Budget notes {number}"),
            "txt",
            &format!(
                "The Orion infrastructure budget is nine dollars per month. {}",
                "Employees run local inference on their own computers. ".repeat(25)
            ),
        )?;
    }
    let hits = store.search("Orion budget")?;
    let jobs = OperationJobs::default();
    let gate = Arc::new(Semaphore::new(1));
    let id = jobs.begin()?;
    let key = id.clone();
    let owner = jobs.clone();
    let shared_gate = gate.clone();
    let conn = connection.clone();
    let evidence = hits.clone();
    let task = tokio::spawn(async move {
        owner
            .run(
                &key,
                shared_gate,
                inference::answer_authenticated(
                    &conn.endpoint,
                    "What is the Orion infrastructure budget per month?",
                    evidence,
                    false,
                    Some(&conn.token),
                    Some(&conn.certificate),
                ),
            )
            .await
    });
    let observed = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let snapshot = slots(&client, &connection).await?;
            if processing(&snapshot)? {
                return Ok::<_, Box<dyn std::error::Error>>(snapshot);
            }
            if task.is_finished() {
                return Err("Answer completed before observing an active slot".into());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await??;
    println!("PASS: production answer reached the real engine's active slot.");
    let cancelled_at = Instant::now();
    assert!(jobs.cancel(&id)?);
    let result = tokio::time::timeout(Duration::from_secs(2), task).await??;
    assert_eq!(result.unwrap_err(), CANCELLED);
    let client_cancel_ms = cancelled_at.elapsed().as_millis();
    assert_eq!(gate.available_permits(), 1);
    println!("PASS: answer cancelled and operation permit released in {client_cancel_ms} ms.");
    tokio::time::timeout(Duration::from_secs(3), async {
        while processing(&slots(&client, &connection).await?)? {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        Ok::<_, Box<dyn std::error::Error>>(())
    })
    .await??;
    let engine_idle_ms = cancelled_at.elapsed().as_millis();
    println!("PASS: engine slot returned to idle within {engine_idle_ms} ms of cancellation.");
    let retry = jobs.begin()?;
    assert!(!jobs.cancel(&id)?);
    let answer = jobs
        .run(
            &retry,
            gate,
            inference::answer_authenticated(
                &connection.endpoint,
                "What is the Orion infrastructure budget per month?",
                hits,
                true,
                Some(&connection.token),
                Some(&connection.certificate),
            ),
        )
        .await?;
    assert!(!answer.answer.insufficient_evidence && !answer.answer.source_ids.is_empty());
    assert!(
        answer.answer.answer.to_lowercase().contains("nine") || answer.answer.answer.contains('9')
    );
    println!("PASS: retry produced the budget answer with a valid source citation.");
    runtime.stop()?;
    assert!(!runtime.status()?.running);
    println!("PASS: managed runtime stopped.");
    let report = json!({"observed_processing":observed[0]["is_processing"],"client_cancel_ms":client_cancel_ms,"engine_idle_ms":engine_idle_ms,"retry_answer":answer,"runtime_stopped":true,"scope":"One synthetic Qwen Q4 request, actual managed TLS connection and llama.cpp slots; cancellation timing is an observation, not a hardware guarantee."});
    std::fs::write(&args[3], serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}
