# Cancellation-safe managed startup

Date: 2026-10-10. Scope: managed generation and embedding startup, as a prerequisite
for the planned one-model residency policy. Automatic switching remains unfinished.

## Behavior and regression evidence

The startup future now owns a cleanup guard until readiness commits. Dropping an
interrupted startup stops its own child before releasing the global operation
permit. Cleanup compares endpoint and private token, preserves another connection,
and recovers a poisoned runtime mutex for shutdown. Credentials stay private.

The original cancellation regression failed with `cancelled startup left its
model running`. After the change it passed for both runtime roles. The preceding
full Rust suite passed 142 tests, and an independent fresh-context review found
no concrete correctness defect. Its two evidence gaps were successful real-engine
startup and shutdown while readiness was pending.

The added `shutdown_during_startup_releases_permit_without_publishing_readiness`
test follows the exit callback's closing barrier, operation shutdown and both
runtime stops while the fixture listener is alive. Both generation and embedding
cases assert a closing error, no running/ready model, no embedding profile,
closed listener and reusable operation permit.

```sh
cargo test --manifest-path src-tauri/Cargo.toml --lib --features desktop shutdown_during_startup
```

Result: `1 passed; 0 failed; 142 filtered out`, 0.64 seconds. This additional test
covers existing behavior after the cancellation fix; no production change was
needed. The earlier full suite was not repeated for a test-only addition.

```sh
/opt/homebrew/opt/rust/bin/cargo-clippy clippy --manifest-path src-tauri/Cargo.toml --lib --features desktop --tests -- -D warnings
/opt/homebrew/opt/rust/bin/cargo-fmt fmt --manifest-path src-tauri/Cargo.toml --check
```

Clippy passed in 11.68 seconds; formatting passed. The initial ordinary cargo-fmt
invocation found an unconfigured rustup shim; the installed Homebrew formatter
completed successfully.

## Packaged check

The isolated `com.cachella.mediacheck` debug app rebuilt successfully with the
included engine and decoder. Build output reports 16.45 seconds for Rust and one
145.09 MiB app bundle. See [build log](native-startup-build.txt) and
[source/binary fingerprints](native-startup-artifact.json).

Manual CUA observations: retained Gemma generation selection; Start model
transitioned through loading to Running, Connected and Local model connected.
This exercises successful TLS/authentication readiness through the desktop command.
Embedding startup then displayed Running and `2 of 2 passages indexed`, exercising
the authenticated compatibility query and embedding profile publication. Normal
quit left zero executables under the isolated app bundle, verified with
`ps -axo pid=,comm=` and an exact bundle-path filter. No original user app or
external server was stopped.

## Limits

This increment adds no memory-policy setting or automatic model switching. The
holding fixture proves lifecycle behavior and never claims successful inference.
Native Windows, complete consumer-hardware qualification and final acceptance
remain unfinished. The native check uses the existing synthetic isolated profile.
