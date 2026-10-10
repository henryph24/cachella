//! Subprocess fixture for scratch recovery, including a live production guardian.
use cachella_lib::{process_guard, scratch};
use serde_json::json;
use std::{
    io::Read,
    path::PathBuf,
    process::Command,
    time::{Duration, Instant},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(code) = process_guard::entry() {
        std::process::exit(code);
    }
    let args: Vec<_> = std::env::args_os().collect();
    match args.get(1).and_then(|arg| arg.to_str()) {
        Some("owner") => {
            let manifest = PathBuf::from(args.get(2).ok_or("Missing manifest")?);
            let marker = manifest.with_extension("tool.pid");
            let mut workspaces = Vec::new();
            for (name, bytes) in [
                ("input.pdf", b"%PDF-synthetic".as_slice()),
                ("input.wav", b"RIFFsynthetic".as_slice()),
                ("key.pem", b"synthetic-private-key".as_slice()),
            ] {
                let workspace = scratch::Workspace::new()?;
                std::fs::write(workspace.path().join(name), bytes)?;
                workspaces.push(workspace);
            }
            let mut tool = Command::new(std::env::current_exe()?);
            tool.env_clear().arg("tool").arg(&marker);
            for key in ["SYSTEMROOT", "WINDIR"] {
                if let Some(value) = std::env::var_os(key) {
                    tool.env(key, value);
                }
            }
            let _guard = process_guard::spawn(&tool, false)?;
            let deadline = Instant::now() + Duration::from_secs(5);
            while !marker.exists() {
                if Instant::now() > deadline {
                    return Err("Tool did not initialize".into());
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            let tool_pid: u32 = std::fs::read_to_string(&marker)?.parse()?;
            std::fs::write(
                manifest,
                serde_json::to_vec(&json!({
                    "owner_pid":std::process::id(), "guardian_pid":_guard.id(), "tool_pid":tool_pid,
                    "workspaces": workspaces.iter().map(|workspace| workspace.path()).collect::<Vec<_>>()
                }))?,
            )?;
            let mut input = Vec::new();
            std::io::stdin().read_to_end(&mut input)?;
        }
        Some("tool") => {
            std::fs::write(
                PathBuf::from(args.get(2).ok_or("Missing tool marker")?),
                std::process::id().to_string(),
            )?;
            std::thread::sleep(Duration::from_secs(60));
        }
        Some("recover") => {
            scratch::initialize()?;
        }
        Some("normal") => {
            let workspace = scratch::Workspace::new()?;
            let path = workspace.path().to_owned();
            std::fs::write(path.join("normal.txt"), b"synthetic")?;
            drop(workspace);
            assert!(!path.exists(), "Normal workspace drop retained files");
        }
        _ => return Err(
            "Usage: scratch_recovery_probe owner manifest.json | tool marker | recover | normal"
                .into(),
        ),
    }
    Ok(())
}
