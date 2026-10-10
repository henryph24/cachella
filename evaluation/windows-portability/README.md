# Windows test portability

The embedding runtime argument test imported `std::os::unix::fs::PermissionsExt` unconditionally and launched a shell script. This prevented the Windows test build from compiling that module.

The test now copies the existing compiled process-guardian fixture to `llama-server` with the platform executable suffix. The fixture records the actual child arguments as JSON beside the synthetic model. The test still runs the production runtime start path, requires the embedding projector, checks context/pooling/projector arguments, then checks generation arguments independently. Both child exits must succeed, and the first capture is removed before the second launch. The platform-independent bundled-configuration test also runs on Windows now.

Test-first evidence: after switching the caller to the compiled fixture, the focused test failed with `No such file or directory` when reading `args.json` (the old fixture did not implement capture). Adding the fixture capture made all five runtime tests pass. This regression exercises the new test helper; it does not establish a Windows compiler result.

Verification on macOS:

- `cargo build --manifest-path src-tauri/Cargo.toml --features desktop --example process_guard_fixture`: exit 0.
- `cargo test --manifest-path src-tauri/Cargo.toml --lib --features desktop`: 117 passed, 0 failed, 1.58 seconds. See [test output](rust-tests.txt).
- `/opt/homebrew/opt/rust/bin/cargo-clippy clippy --manifest-path src-tauri/Cargo.toml --all-targets --features desktop -- -D warnings`: exit 0, finished in 12.42 seconds. See [Clippy output](clippy.txt).
- `/opt/homebrew/opt/rust/bin/cargo-fmt fmt --manifest-path src-tauri/Cargo.toml -- --check`: exit 0.

Only the `aarch64-apple-darwin` Rust target is installed on this host. Windows compilation and runtime remain unverified. No production runtime behavior changed, no package was rebuilt, and frontend tests were skipped because frontend code is unchanged. The separate Windows scratch-deletion issue remains open.
