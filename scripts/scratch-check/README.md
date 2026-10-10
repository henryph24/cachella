# Scratch backend check harness

This crate compiles the production `src-tauri/src/scratch.rs` and `src-tauri/src/settings.rs` sources and their tests directly. It allows Windows type checking without the desktop SDK and bundled SQLite C compiler. It does not replace the desktop build or Windows execution checks.

On a host with the Windows Rust standard library installed:

```sh
CARGO_TARGET_DIR=.local/scratch-check-target cargo check --manifest-path scripts/scratch-check/Cargo.toml --target x86_64-pc-windows-msvc --tests
```

On a Windows machine, run the source tests:

```powershell
cargo test --manifest-path scripts/scratch-check/Cargo.toml
```

Run the full desktop checks separately using the root project's npm scripts. The fixture crate has its own lockfile so a Windows check can be reproduced without desktop dependency resolution.
