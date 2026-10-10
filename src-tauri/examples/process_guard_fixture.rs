//! A deliberately small executable used by process_guard integration tests.
//! It also implements the production guardian entry point before any fixture
//! behaviour so the test path exercises the same framing and lease protocol.

use cachella_lib::process_guard;
use std::io::Write;
use std::net::TcpListener;
use std::process::{self, Command};
use std::thread;
use std::time::{Duration, Instant};

fn main() {
    if let Some(code) = cachella_lib::pdf::entry() {
        process::exit(code);
    }
    if let Some(code) = process_guard::entry() {
        process::exit(code);
    }
    // The runtime test copies this executable to llama-server[.exe]. Capture
    // its actual arguments without requiring a platform shell or an env var
    // that production intentionally removes from the child environment.
    if std::env::current_exe()
        .ok()
        .and_then(|path| path.file_stem().map(|stem| stem.to_owned()))
        .is_some_and(|stem| stem == "llama-server")
    {
        let args: Vec<_> = std::env::args_os().skip(1).collect();
        let model = args
            .windows(2)
            .find(|pair| pair[0] == "-m")
            .map(|pair| std::path::Path::new(&pair[1]))
            .expect("model argument");
        let capture = model.parent().expect("model directory").join("args.json");
        let args: Vec<_> = args
            .iter()
            .map(|arg| arg.to_str().expect("UTF-8 test argument"))
            .collect();
        std::fs::write(
            capture,
            serde_json::to_vec(&args).expect("serialize arguments"),
        )
        .expect("write arguments");
        if model.file_name().is_some_and(|name| name == "hold.gguf") {
            let port = args
                .windows(2)
                .find(|pair| pair[0] == "--port")
                .expect("runtime port")[1]
                .parse::<u16>()
                .expect("port number");
            let _listener = TcpListener::bind(("127.0.0.1", port)).expect("bind runtime fixture");
            std::fs::write(
                model.parent().unwrap().join("runtime-ready"),
                process::id().to_string(),
            )
            .expect("write readiness marker");
            thread::sleep(Duration::from_secs(30));
        }
        return;
    }
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("emit") => {
            print!("fixture stdout");
            let _ = std::io::stdout().flush();
            eprint!("fixture stderr");
            process::exit(7);
        }
        Some("emit-large") => {
            let bytes = vec![b'x'; 2 * 1024 * 1024];
            std::io::stdout()
                .write_all(&bytes)
                .expect("write large stdout");
            process::exit(0);
        }
        Some("hold") => {
            let marker = args.next().expect("marker path");
            let port = args
                .next()
                .expect("port")
                .parse::<u16>()
                .expect("port number");
            let listener = TcpListener::bind(("127.0.0.1", port)).expect("bind test port");
            std::fs::write(marker, process::id().to_string()).expect("write marker");
            let deadline = Instant::now() + Duration::from_secs(30);
            while Instant::now() < deadline {
                let _ = listener.set_nonblocking(true);
                let _ = listener.accept();
                thread::sleep(Duration::from_millis(25));
            }
        }
        Some("lease-parent") => {
            let marker = args.next().expect("marker path");
            let port = args.next().expect("port");
            let mut child = Command::new(std::env::current_exe().expect("fixture executable"));
            child.args(["hold", &marker, &port]).env_clear();
            let _guard = process_guard::spawn(&child, false).expect("spawn guarded holder");
            let deadline = Instant::now() + Duration::from_secs(3);
            while !std::path::Path::new(&marker).exists() && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(10));
            }
            assert!(
                std::path::Path::new(&marker).exists(),
                "holder did not initialize"
            );
            process::exit(0);
        }
        _ => process::exit(64),
    }
}
