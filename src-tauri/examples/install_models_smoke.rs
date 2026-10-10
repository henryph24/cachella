//! Exercise the production pinned downloader and retain its verified files for desktop use.
use cachella_lib::models::ModelInstaller;
use serde_json::json;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("Usage: install_models_smoke model-directory bundle-id report.json".into());
    }
    let installer = ModelInstaller::new(PathBuf::from(&args[1]));
    let started = Instant::now();
    let install = installer.install(&args[2]);
    tokio::pin!(install);
    let mut tick = tokio::time::interval(Duration::from_millis(500));
    let mut previous = None;
    let mut phases = Vec::new();
    let installed = loop {
        tokio::select! {
            result = &mut install => break result?,
            _ = tick.tick() => {
                let progress = installer.status()?;
                let key = (progress.phase.clone(), progress.downloaded_bytes / (64 * 1024 * 1024));
                if previous.as_ref() != Some(&key) {
                    println!("{}: {} of {} bytes", progress.phase, progress.downloaded_bytes, progress.total_bytes);
                    previous = Some(key);
                }
                if phases.last() != Some(&progress.phase) { phases.push(progress.phase); }
            }
        }
    };
    let progress = installer.status()?;
    if progress.phase != "ready" || progress.downloaded_bytes != progress.total_bytes {
        return Err("Installer did not report a verified complete bundle".into());
    }
    let mut files = Vec::new();
    for path in [Some(&installed.model), installed.projector.as_ref()]
        .into_iter()
        .flatten()
    {
        files.push(json!({"name":path.file_name().ok_or("Missing file name")?.to_string_lossy(),"bytes":std::fs::metadata(path)?.len()}));
    }
    let report = json!({"bundle_id":args[2],"elapsed_ms":started.elapsed().as_millis(),"observed_phases":phases,"progress":progress,"files":files,"verified":true,"scope":"Production installer; SHA256, GGUF header and exact size enforced before publication. Engine selection/start is a separate native UI check."});
    std::fs::write(&args[3], serde_json::to_vec_pretty(&report)?)?;
    println!(
        "PASS: {} verified model file(s), {} bytes, {} ms",
        files.len(),
        progress.total_bytes,
        started.elapsed().as_millis()
    );
    Ok(())
}
