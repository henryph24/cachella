# Cancellation during embedding file verification

The desktop startup path previously awaited one `spawn_blocking` task that hashed
both embedding files. Dropping the await during an operation cancellation left the
worker reading and hashing until completion. The operation permit could already
be available for a retry during that work.

`embedding::profile_cancellable` now owns a drop guard that signals the blocking
worker. The shared hash implementation checks that signal before each file,
before and after each 64 KiB read, and before returning the completed identifier.
`start_runtime_admitted` awaits this function. Synchronous evaluation callers
continue to use `profile`; both paths share the same hash implementation.

Cancellation is cooperative. An in-progress filesystem call can complete before
the worker observes the signal, and a chunk already being hashed can finish.
There is no deadline for an operating-system read. The request does not await
worker termination after cancellation, so brief overlap with a retry is possible.
Explicit startup commands retain their existing shutdown behavior; this change
covers cancellation by dropping the verification future, including registered
operations cancelled at shutdown. It adds no new Stop command for explicit startup.

## Verification

- Test-first regression: `cargo test --manifest-path src-tauri/Cargo.toml --lib
  --features desktop cancelled_profile_task_signals_already_running_worker`
  failed with `cancelled startup left verification work active` before adding the
  guard. [Raw failure](red.txt).
- The worker regression waits for a real blocking task to start, aborts its
  awaiting task, then releases the worker and checks the observed signal. The
  hash regression cancels between chunks before the missing projector is opened;
  it also checks cancellation before validating any model path.
- Successful asynchronous and synchronous profiles match; invalid selections
  retain their errors. The existing content-change test now includes a fixed
  pre-change SHA-256 identifier for the two fixture files.
- Focused embedding tests: 7 passed, 0 failed, 0.03 s.
  [Output](focused.txt).
- Final `cargo test --manifest-path src-tauri/Cargo.toml --lib --features desktop`:
  152 passed, 0 failed, 3.54 s. This includes existing desktop switch cancellation,
  retry and startup cleanup tests. [Output](full.txt).

All-target desktop Clippy with `-D warnings` passed (11.05 s), and
`cargo-fmt fmt --manifest-path src-tauri/Cargo.toml --check` passed.
[Clippy output](clippy.txt).

Independent concurrency review found no correctness issue. Tests cover worker
signalling and hash checkpoints separately; a single end-to-end test cancelling
a registered operation during a real-file hash remains unimplemented.

## Packaged Mac follow-up

Rebuilt with `npx tauri build --bundles app --features desktop` and the engine,
decoder and isolated `com.cachella.mediacheck` configuration overrides. Optimized
Rust build: 13.73 s; app bundle: 114.85 MiB. [Build output](release-build.txt) and
[source/executable hashes](release-artifacts.json) identify this artifact.

Native CUA checks retained Single policy and both selected model pairs. Starting
embeddings preserved 2/22 indexed passages. Starting generation suspended the
embedding engine; Index library automatically resumed it. Cancel indexing
returned idle controls and preserved 2/22 progress. Embedding startup had already
completed before cancellation, so this run provides no direct observation of
cancellation during hashing or worker termination.

Retry completed 22/22 passages. Semantic search for “What monthly cloud budget
does the video mention?” returned eight matches, with original budget-video audio
first. Normal quit left zero executables from the test bundle. The isolated
profile now contains two documents, 22 chunks, 22 embeddings, two assets and
22 media parts. The original user profile was untouched.

[Native observations](native-observations.json) and [process checks](native-process-checks.json)
record the scope. This updates the previous resource run's isolated profile from
2 to 22 stored embeddings. Existing resource measurements describe the preceding
release artifact; no memory benchmark was repeated here. Windows/consumer-device
qualification and full product acceptance remain open.
