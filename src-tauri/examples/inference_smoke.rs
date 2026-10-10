use cachella_lib::{inference, store::Store};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let endpoint = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "http://127.0.0.1:18080".into());
    let status = inference::status(&endpoint).await?;
    if !status.connected {
        return Err(status.message.into());
    }
    println!("Connected model: {}", status.model.unwrap_or_default());
    let mut db = Store::open(std::path::Path::new(":memory:"))?;
    db.import_text("Launch decision", "md", "The Atlas launch moved to Friday because the security review found an unresolved permission bug. Priya owns the fix. The launch is approved once the permission regression tests pass.")?;
    let question = "Why did the Atlas launch move to Friday?";
    for cache in [false, true] {
        let result = inference::answer(&endpoint, question, db.search(question)?, cache).await?;
        if result.answer.insufficient_evidence || result.answer.source_ids.is_empty() {
            return Err("Expected a cited answer from the actual model".into());
        }
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({"cache_enabled":cache,"result":result}))?
        );
    }
    println!("PASS: two real local generations returned valid source references.");
    Ok(())
}
