# One-model residency feasibility

Date: 2026-10-10. The release `multimodal_smoke` example now accepts an optional
`--single-resident` argument after the generation model/projector. This is an
evaluation-only extension. Desktop model switching and a resource-policy control
are still unimplemented.

## Reason for this check

The HTML product plan calls for shared resource budgeting. Current desktop starts
are serialized, but both models can remain loaded. Model-file sizes and macOS
footprint observations cannot support an exact cross-platform RAM limit. A
one-managed-engine residency policy provides a directly testable resource bound,
with model fit and whole-device RAM qualification treated separately.

The bundled engine's `--help` reports `--fit` enabled by default. It adjusts unset
arguments to fit device memory; this is an engine-local feature and does not
coordinate the app's two runtimes. The app explicitly fixes context and slot
counts. Existing process guards, operation permits and persisted vectors can be
reused for a one-model mode. Source inspection also found that moving startup
inside a cancellable answer requires cleanup when its readiness future is dropped;
the current manual startup path has no such scoped cancellation guard.

## Actual sequence and result

1. Start and authenticate EmbeddingGemma; import five media fixtures and one text
   document; index seven passages; close/reopen SQLite and verify persistence.
2. Compute five retrieval results while embeddings are loaded. Stop that engine
   and verify its listener is closed.
3. Start Gemma generation and verify the embedding runtime is stopped. Complete
   five retrieved-evidence answers and two video-only answers with source IDs and
   timestamps. The selected evidence remains available after embedding shutdown.
4. Stop generation and verify its listener is closed before restarting embeddings.
   Recompute the embedding profile, verify all seven saved vectors remain indexed,
   and run a budget query whose first result is the budget audio or video.
5. Verify deletion behavior, stop the final engine and verify its listener closed.
   Read-only process inspection found zero probe/guardian executables afterward.

The run printed:

```text
PASS: single-resident switch and embedding restart preserve the semantic index.
PASS: persistence, timestamp metadata, text-only generation guard, deletion and process cleanup verified.
```

[Raw report](single-resident.json) records five passing retrieval rankings and
seven structured answer results. Manual review found red-circle descriptions,
$8/month, Priya's ownership and the expected unsupported-revenue abstention.
The image/video-frame and audio/video-audio queries repeat questions and share
standalone media twins. The explicit video-only answer cites both video segments.
This is bounded synthetic evidence, not a general quality score.

This probe precomputes the five retrieval results before switching once to
answering; it does not exercise per-question automatic switching through the UI.
It confirms process/listener sequencing and persistence. It does not measure the
moment GPU/OS allocations are reclaimed or establish a minimum RAM requirement.
Stopping an engine loses its in-memory prompt cache, so automatic switching has
a startup and cache-reuse tradeoff. Persistent document embeddings remain intact.

## Reproduce and checks

```sh
cargo build --release --manifest-path src-tauri/Cargo.toml --example multimodal_smoke
src-tauri/target/release/examples/multimodal_smoke \
  src-tauri/resources/engine/llama-server \
  .local/models/embeddinggemma-2-Q8_0.gguf \
  .local/models/mmproj-embeddinggemma-2-Q8_0.gguf \
  src-tauri/resources/decoder/ffmpeg evaluation/multimodal/fixtures \
  /tmp/cachella-single-resident-repeat.json \
  .local/models/gemma-4-E2B-it-Q4_0.gguf \
  .local/models/mmproj-gemma-4-E2B-it-Q8_0.gguf --single-resident
```

An initial missing closing brace prevented formatting/compilation and was fixed.
The final release build passed in 3.30 seconds; example Clippy with warnings
denied passed in 1.14 seconds; formatting check exited zero. This exploratory
harness extension uses test-after verification with the real engines. No product
behavior changed, so product/frontend suites and repackaging were not repeated.

Unfinished: persisted policy selection, desktop coordination across search,
indexing and answering, cancellation during a model switch, failure recovery,
independent review of the changed concurrency paths, native UI verification and
Windows/target-hardware qualification. The user preference question about the
default remains unanswered; existing two-model behavior remains in place.
