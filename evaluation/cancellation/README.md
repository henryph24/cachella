# In-flight answer cancellation

## Behavior

The desktop registers an answer request before submitting it. Cancellation uses that request's identifier and covers semantic retrieval plus generation. A pending registration holds no operation permit; it can be cancelled or superseded before work starts. An active request owns the existing model-operation semaphore until its future is dropped or completed. Stale identifiers cannot cancel a newer request.

The answer registry uses a watched cancellation value and serializes completion with cancellation. An accepted cancellation overrides a simultaneously ready result. Aborting the command drops its guard, releasing both registration and operation permit. Shutdown closes registration and signals running work. Normal process exit also closes client sockets through the operating system; the desktop synchronously stops its managed engines.

The UI exposes cancellation in the global footer, including after navigating to another view. Retrieved evidence remains available for inspection and retry. Cancellation during request registration prevents submission when the identifier arrives. Unmount requests cleanup; cancellation and completion are scoped to the originating UI operation.

## Actual engine measurement

The [model smoke output](model-smoke.txt) records five passing checks using the production answer client, private TLS/authentication, the included llama.cpp b11531 engine and Qwen3-1.7B Q4_K_M. The probe first observes `is_processing=true` from the authenticated `/slots` endpoint, then cancels through the production registry.

In this synthetic run, the app request settled in **0 ms at integer-millisecond resolution** and the engine slot returned to idle after **1,308 ms**. A retry produced the expected nine-dollar budget answer with a valid source identifier. The runtime then stopped. See [raw results](model-results.json). These observations do not establish a hardware latency guarantee or instantaneous termination of an engine's current compute batch. Independently managed server cancellation behavior remains engine-dependent.

```sh
cargo run --manifest-path src-tauri/Cargo.toml --example cancel_answer_smoke -- \
  /path/to/llama-server /path/to/model.gguf evaluation/cancellation/model-results.json
```

## Verification

The initial registry tests failed before cancellation was implemented; see [red output](regression-red.txt). Tests exercise pending and active cancellation, stale identifiers, cancellation concurrent with ready output, command abortion, shutdown notification and permit release. A loopback integration fixture verifies that cancelling the actual inference client closes its HTTP connection while awaiting headers and while reading an incomplete response body.

One full-suite execution had a missing PID marker in the pre-existing decoder-abortion test. Its isolated rerun passed. The [initial failure](rust-initial-failure.txt) is retained; no causal claim is made from that single observation. The final full suite returned **105 passed, 0 failed** ([Rust output](rust-tests.txt)). The frontend suite returned **44 passed** across six files ([frontend output](frontend-tests.txt)); all-target desktop Clippy with warnings denied passed ([output](clippy.txt)). Rust formatting and TypeScript/Vite checks passed.

Independent review completed two rounds. It identified delayed cancellation acknowledgement losing a completed answer, plus cancellation-before-submission falling through to generation when cleanup failed. Four UI ordering regressions reproduced these cases before fixes. Final review found no remaining material issue. Additional UI tests cover accepted cancellation, retry, late success suppression, begin failure, cancellation failure, navigation and unmount while registration is pending.

The final `npm run desktop:build -- --debug --bundles app` produced one **115.09 MiB** macOS debug bundle ([build output](package-build.txt)). Native verification cancelled an active answer from the search view and another after navigating to Documents. Both showed “Answer generation cancelled.” and restored the controls. Six retrieved passages remained available. A retry returned an answer with four source citations, displaying **4.7 seconds**, **609 input tokens** and **608 cached tokens**. Citation membership does not prove semantic entailment or representative quality.

Both runtimes were left stopped. Read-only SQLite and process checks verified **7 documents, 12 passages, 12 vectors and zero app descendants**. See [native results](native-results.json). Windows runtime execution remains unverified.

## Remaining scope

Standalone search/index cancellation, decoder/PDF/runtime scratch recovery, media-to-text answers, decoder distribution, whole-app memory qualification, representative answer quality, Windows runtime/signing and the full product audit remain unfinished. This feature does not add LMCache.
