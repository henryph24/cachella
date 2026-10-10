//! Captures real outputs for manual rubric review. Structural checks do not grade factual accuracy.
use cachella_lib::{
    inference,
    runtime::{Runtime, RuntimeConfig},
    store::Store,
};
use serde::Deserialize;
use serde_json::json;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Deserialize)]
struct Document {
    title: String,
    text: String,
}
#[derive(Deserialize)]
struct Case {
    id: String,
    question: String,
    documents: Vec<Document>,
    expect_abstention: bool,
    rubric: String,
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
    if args.len() != 5 && args.len() != 6 {
        return Err(
            "Usage: evaluate_local_rag ENGINE MODEL CASES_JSON OUTPUT_JSON [PROJECTOR]".into(),
        );
    }
    let cases: Vec<Case> = serde_json::from_slice(&std::fs::read(&args[3])?)?;
    if cases.is_empty() {
        return Err("No evaluation cases supplied".into());
    }
    let started = Instant::now();
    let mut runtime = Runtime::new(RuntimeConfig {
        executable: Some(PathBuf::from(&args[1])),
        bundled_engine: false,
        model: Some(PathBuf::from(&args[2])),
        projector: args.get(5).map(PathBuf::from),
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
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    })
    .await??;
    inference::verify_authentication(&connection.endpoint, Some(&connection.certificate)).await?;
    runtime.mark_ready()?;
    let startup_ms = started.elapsed().as_millis();
    let mut outputs = Vec::new();
    let mut errors = 0;
    let mut abstention_mismatches = 0;
    for case in cases {
        let mut store = Store::open(std::path::Path::new(":memory:"))?;
        for document in &case.documents {
            store.import_text(&document.title, "txt", &document.text)?;
        }
        for cache in [false, true] {
            let evidence = store.search(&case.question)?;
            let media = inference::AnswerMedia::new();
            match inference::answer_with_media_authenticated(
                &connection.endpoint,
                &case.question,
                evidence,
                cache,
                Some(&connection.token),
                Some(&connection.certificate),
                args.get(5).map(|_| &media),
            )
            .await
            {
                Ok(result) => {
                    let matches = result.answer.insufficient_evidence == case.expect_abstention;
                    if !matches {
                        abstention_mismatches += 1;
                    }
                    println!(
                        "{} cache={} {} ms, abstention_matches={}",
                        case.id, cache, result.elapsed_ms, matches
                    );
                    outputs.push(json!({"id":case.id,"question":case.question,"cache_requested":cache,"rubric":case.rubric,"expected_abstention":case.expect_abstention,"abstention_matches":matches,"result":result}));
                }
                Err(error) => {
                    errors += 1;
                    println!("{} cache={} ERROR: {}", case.id, cache, error);
                    outputs.push(json!({"id":case.id,"question":case.question,"cache_requested":cache,"rubric":case.rubric,"error":error}));
                }
            }
        }
    }
    runtime.stop()?;
    let report = json!({"model":args[2],"projector":args.get(5),"media_capable_request":args.len()==6,"model_file_bytes":std::fs::metadata(&args[2])?.len(),"context_tokens":4096,"parallel_slots":1,"startup_ms":startup_ms,"attempted_requests":outputs.len(),"request_errors":errors,"abstention_mismatches":abstention_mismatches,"grading":"Structural and abstention checks only. Review all answers against their rubrics before claiming factual accuracy.","outputs":outputs});
    std::fs::write(&args[4], serde_json::to_vec_pretty(&report)?)?;
    println!(
        "Captured {} requests: {} errors, {} abstention mismatches. Manual rubric review required.",
        outputs.len(),
        errors,
        abstention_mismatches
    );
    Ok(())
}
