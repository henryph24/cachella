# Recoverable settings publication

Runtime, embedding-runtime, endpoint and media-tool settings now share one atomic JSON writer. It creates a leased workspace inside the destination's application-data directory, writes and syncs a temporary file, then replaces the committed file by rename. Keeping staging under that parent preserves same-filesystem publication. Normal success and failure remove the workspace; a killed writer leaves an owned session that startup or the next save can reclaim.

The existing scratch session/coordinator mechanism protects active writers in other processes. Concurrent instances retain the existing last-commit-wins settings behavior. Recovery only visits its owned UUID namespace, preserves unrecognized legacy temporary files and uses the platform-specific retained-handle cleanup backend. The application-data parent is the ambient trust boundary. Helpers never receive these publication workspaces, so they require no guardian lease. There is no settings schema migration: existing JSON names, representations and loaders remain compatible.

Desktop startup recovers settings staging before opening the library or loading settings. All settings writers use `settings::save_json`. The endpoint path also holds the runtime and endpoint state guards while publishing. It changes inference mode and the in-memory selection only after a successful file commit; a failed save preserves the managed mode and previous endpoint.

## Verified on macOS

- `cargo test --manifest-path src-tauri/Cargo.toml --lib --features desktop`: **130 passed, 0 failed**, 1.33 seconds. [Full output](rust-tests.txt).
- Six publication tests cover: old contents visible until commit, destination preservation on failure, JSON round-trip with unrelated temporary-file preservation, overlapping live writers, serialization failure and forced writer termination.
- The forced-exit case starts a separate test process using the production staging path, waits for its atomic readiness marker, recovers while its lease is live, kills/reaps it, then recovers its staging. The previously committed JSON remains unchanged. A fixture guard also reaps the process if the parent test fails.
- The endpoint-mode regression first failed because a failed save had already selected external inference. Moving that change after publication made it pass. [Failure](endpoint-red.txt), [passing regression](endpoint-green.txt).
- All-target desktop Clippy with warnings denied passed in 2.83 seconds, and Rust formatting passed. [Clippy output](clippy.txt).

Independent review found no material issue in the publication, lease, state-ordering or wiring boundaries.

The new lifecycle used test-after verification because it was an exploratory integration of existing session leases. The reproduced endpoint ordering defect used a failing regression before its fix.

## Windows checks and limits

The [isolated check harness](../../scripts/scratch-check/README.md) now includes the exact production scratch and settings modules, including their tests:

```sh
cargo check --manifest-path scripts/scratch-check/Cargo.toml --target x86_64-pc-windows-msvc --tests
cargo clippy --manifest-path scripts/scratch-check/Cargo.toml --target x86_64-pc-windows-msvc --tests -- -D warnings
```

Cross-target compilation and Clippy passed. [Check output](windows-check.txt), [Clippy output](windows-clippy.txt). These checks cover the writer/recovery modules; they do not link or execute the Windows desktop.

Native Windows rename, file-sharing and process-crash behavior remains unverified. This contract covers process interruption; it does not establish filesystem or power-loss durability of the renamed parent directory entry. Frontend tests, model inference, native startup/UI checks and package rebuilds were skipped because this increment changes persistence internals. The previous packaged app has not been rebuilt with this increment.

The full goal remains active: native Windows qualification/signing, decoder distribution, media-to-text answers, consumer resource/quality qualification and final product acceptance remain unfinished. No deployment, publication or user-library mutation occurred during these isolated tests.
