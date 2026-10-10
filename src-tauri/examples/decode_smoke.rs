use cachella_lib::media;
use std::path::Path;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(code) = cachella_lib::process_guard::entry() {
        std::process::exit(code);
    }
    run()
}

#[tokio::main]
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 3 {
        return Err("Usage: decode_smoke /path/to/ffmpeg media-files...".into());
    }
    for path in &args[2..] {
        let prepared = media::prepare(Path::new(&args[1]), Path::new(path)).await?;
        if prepared.parts.is_empty() {
            return Err("No media parts prepared".into());
        }
        println!(
            "PASS: {} -> {} preview, {} indexed parts",
            Path::new(path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy(),
            prepared.mime,
            prepared.parts.len()
        );
    }
    Ok(())
}
