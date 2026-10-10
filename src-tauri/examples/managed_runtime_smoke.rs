use cachella_lib::{
    inference,
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
    if args.len() != 3 {
        return Err(
            "Usage: managed_runtime_smoke /path/to/llama-server /path/to/model.gguf".into(),
        );
    }
    let mut runtime = Runtime::new(RuntimeConfig {
        executable: Some(PathBuf::from(&args[1])),
        bundled_engine: false,
        model: Some(PathBuf::from(&args[2])),
        projector: None,
    });
    let connection = runtime.start()?;
    tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            if !runtime.status()?.running {
                return Err("Engine exited while loading the model".to_string());
            }
            if inference::status_authenticated(
                &connection.endpoint,
                Some(&connection.token),
                Some(&connection.certificate),
            )
            .await
            .is_ok_and(|status| status.connected)
            {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    })
    .await??;
    let wrong_certificate = rcgen::generate_simple_self_signed(vec!["127.0.0.1".into()])?
        .cert
        .pem();
    if inference::status_authenticated(
        &connection.endpoint,
        Some(&connection.token),
        Some(&wrong_certificate),
    )
    .await?
    .connected
    {
        return Err("A mismatched certificate was accepted".into());
    }
    println!("PASS: mismatched TLS certificate rejected before authentication.");
    inference::verify_authentication(&connection.endpoint, Some(&connection.certificate)).await?;
    runtime.mark_ready()?;
    println!("PASS: managed engine ready; unauthenticated requests rejected.");
    let mut db = Store::open(std::path::Path::new(":memory:"))?;
    db.import_text("Decision", "txt", "The Atlas launch moved to Friday because the security review found a permission bug. Priya owns the fix.")?;
    let question = "Why did Atlas launch move to Friday?";
    let answer = inference::answer_authenticated(
        &connection.endpoint,
        question,
        db.search(question)?,
        true,
        Some(&connection.token),
        Some(&connection.certificate),
    )
    .await?;
    if answer.answer.insufficient_evidence || answer.answer.source_ids.is_empty() {
        return Err("Expected a cited answer".into());
    }
    println!(
        "PASS: authenticated answer with {} citation(s), {} ms: {}",
        answer.answer.source_ids.len(),
        answer.elapsed_ms,
        answer.answer.answer
    );
    runtime.stop()?;
    if runtime.status()?.running || runtime.connection().is_ok() {
        return Err("Stopped managed runtime must require an explicit restart".into());
    }
    if inference::status_authenticated(
        &connection.endpoint,
        Some(&connection.token),
        Some(&connection.certificate),
    )
    .await?
    .connected
    {
        return Err("Engine still connected after stop".into());
    }
    println!("PASS: stop reaped the child; connection closed; automatic fallback blocked.");
    let restarted = runtime.start()?;
    if restarted.token == connection.token {
        return Err("Restart reused credentials".into());
    }
    drop(runtime);
    if inference::status_authenticated(
        &restarted.endpoint,
        Some(&restarted.token),
        Some(&restarted.certificate),
    )
    .await?
    .connected
    {
        return Err("Engine remained after owner drop".into());
    }
    println!("PASS: restart rotates credentials; owner drop stops the child.");
    Ok(())
}
