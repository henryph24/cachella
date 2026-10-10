# Search and indexing cancellation verification

Date: 10 October 2026. Scope: local macOS development build, actual managed EmbeddingGemma 2 engine and synthetic fixtures. Windows execution remains unverified.

## Behavior

Search, answers and index batches use one request-scoped operation registry. Registration precedes submission, cancellation is scoped to the registered ID, and dropping the active future releases its shared operation permit. The desktop index command uses the production `embedding::index_batch` function with a maximum of eight passages. Every completed vector commits individually; cancelled work remains pending.

Search keeps its prior results and query when cancellation succeeds. The frontend waits for an outstanding cancellation acknowledgement before deciding whether to publish a completed result. Index cancellation also records a session pause, which stops subsequent batches even when the active batch finishes first or the cancellation bridge rejects. Leaving the runtime view cancels active indexing. This releases the client request; engine compute may take additional time to finish its current batch.

## Automated evidence

Documentation-only updates are exempt from test-first. Local links, Markdown fences, JSON reports and recorded test counts were validated after the updates.

- `npm run test:rust`: **107 passed; 0 failed**. [Full output](rust-tests.txt).
- `npm test`: **51 passed across 6 files**. [Full output](frontend-tests.txt).
- All-target desktop Clippy with `-D warnings`: passed. [Output](clippy.txt).
- Rust format check: exited 0. [Output](format.txt).
- TypeScript and Vite: passed during package build. [Output](package-build.txt).

The deterministic index regression cancels while the second embedding is pending, reopens SQLite with one of ten vectors, then resumes eight and one pending passages. It asserts the committed passage is never submitted again. Existing cancellation tests cover HTTP disconnection, shutdown, abort, stale IDs and completion races. Frontend tests cover pending registration, delayed false/error acknowledgement, retained results, index progress refresh, resume and unmount.

The shared index helper was extracted from the existing desktop loop before adding its focused regression. This was a behavior-preserving extraction followed by cancellation coverage; the generic registry already had eight tests. Frontend tests were migrated with the IPC changes. Independent review found a stale-request ordering issue: the registry originally acquired the shared permit before validating the ID. A regression first returned the busy error for a stale request. [Recorded failure](stale-registration-red.txt). Identity validation and non-blocking permit acquisition now occur in one registry critical section. Round 2 found no remaining material concurrency issue within the reviewed paths.

## Actual engine evidence

Run from the repository root with the locally verified engine, models and FFmpeg:

```sh
cargo run --manifest-path src-tauri/Cargo.toml --example cancel_index_smoke -- \
  .local/engine-candidate/llama-b11531/llama-server \
  .local/models/embeddinggemma-2-Q8_0.gguf \
  .local/models/mmproj-embeddinggemma-2-Q8_0.gguf \
  /opt/homebrew/Cellar/ffmpeg/8.0_2/bin/ffmpeg \
  evaluation/multimodal/fixtures \
  evaluation/search-index-cancellation/model-results.json
```

[Structured result](model-results.json) and [six PASS lines](model-smoke.txt) establish:

1. An actual active embedding slot was observed after a text vector committed. Cancellation returned in 0 integer milliseconds, and the reopened database retained one of ten vectors.
2. Two resumed batches completed the text index; the original vector bytes were unchanged.
3. The production batch path indexed four image/audio/video parts.
4. An actual active query embedding was cancelled in 0 integer milliseconds, releasing its operation permit.
5. Semantic retry ranked a synthetic budget document first; all 14 vectors persisted.
6. The managed embedding runtime stopped.

Zero integer milliseconds means below the measurement's one-millisecond resolution. This measures client completion and permit release, with engine activity observed separately before cancellation. It does not establish engine compute stop latency, consumer hardware limits or general retrieval quality. The real-engine run preceded the final registry ordering fix; the final registry is covered by the full Rust suite and packaged checks.

The initial probe used identical document bodies, so content deduplication left one document and prevented observation of a second active embedding after a commit. The probe now includes a unique passage number in every body. [Initial fixture failure](model-initial-fixture-failure.txt). No production change was needed for that fixture error.

## Packaged native evidence

`npm run desktop:build -- --debug --bundles app` produced one 115.12 MiB macOS debug bundle. Native UI verification used the included engine and the locally installed pinned embedding model. Importing the 67,957-byte synthetic `Index cancellation demo.txt` added 39 passages to the existing 12.

The Cancel indexing button stopped work at **29 of 51 passages**. Resuming reached **51 of 51**. All 29 vector hashes captured after cancellation remained identical after resume. The demo fixture remains in the library, which now contains eight documents.

The Cancel search button appeared during an actual semantic query, but completion won before the native click reached it; the footer then opened Runtime & privacy. This is not evidence of accepted native search cancellation. The real-engine probe verifies active query cancellation, and frontend tests verify its UI ordering. A subsequent native semantic search returned eight hits with the budget PDF at page 3 ranked first.

Both managed models were stopped after verification. `python3 evaluation/search-index-cancellation/verify_native.py` returned `PASS: native index cancellation retained 29/51, resumed 51/51; 29 hashes unchanged; 8 documents; 0 app descendants`. [Structured native observations](native-results.json), [assertion output](native-check.txt) and [read-only verification script](verify_native.py). The script assumes this recorded fixture state and will deliberately fail after the library changes.

## Remaining scope

Accepted in-flight native search cancellation was not observed in this session. Decoder/PDF/runtime scratch recovery, media-to-text answers, decoder distribution, whole-application resource and quality qualification, Windows runtime/signing and the full product acceptance audit remain unfinished. No deployment or publication occurred.
