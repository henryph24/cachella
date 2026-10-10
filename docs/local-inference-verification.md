# Local inference verification

Date: 2026-10-09. Development smoke test on an Apple silicon Mac with 36 GiB RAM.

## Configuration

- Model: official Qwen/Qwen3-1.7B-GGUF, Q8_0, 1,834,426,016 bytes.
- SHA256 verified against publisher file metadata: `061b54daade076b5d3362dac252678d17da8c68f07560be70818cace6590cb1a`.
- Engine: llama-server build 10621, loopback HTTP, 4,096-token context, one slot.
- Retrieval: local SQLite FTS5; synthetic launch-decision evidence.
- Command: `cargo run --manifest-path src-tauri/Cargo.toml --example inference_smoke -- http://127.0.0.1:18080`.

## Observations

| Request | Cache requested | Elapsed | Prompt tokens | Cached tokens | Completion tokens |
|---|---|---:|---:|---:|---:|
| First | No | 2,102 ms | 193 | 0 | 112 |
| Repeated | Yes | 1,846 ms | 193 | 192 | 112 |

Both answers identified the unresolved permission bug and cited the supplied document. The command returned `PASS: two real local generations returned valid source references.`

The native application also generated an answer from three synthetic documents, displayed 2.9 seconds elapsed, 487 input tokens and 76 cached tokens. Clicking its citation opened the original document. Cache reuse came from llama.cpp; LMCache was absent.

These observations verify integration and reported prefix reuse. Two requests do not establish a performance benchmark, quality evaluation, electricity savings, or consumer Windows suitability. The integration candidate uses 8-bit weights; 4-bit quality remains to be tested.

## Other verification

- Frontend: 4 tests passed.
- Rust including desktop boundary: 13 tests passed.
- Clippy with warnings denied and Rust format check: passed.
- Fresh macOS development bundle: built, 35.77 MiB. Signing and full packaged runtime validation remain pending.
- Windows CI definition exists; Windows execution remains unverified.

## Remaining product work

Managed model/server lifecycle, multimodal embeddings and media ingestion, resumable indexing, cancellation, and Windows runtime verification remain incomplete. See [the implementation checklist](../tasks/todo.md).

## Managed lifecycle verification

The managed runtime now uses a fresh certificate and API token on every start. Native reqwest clients trust only that certificate for the managed connection. The child receives a restricted environment, an explicit loopback listener and a 4,096-token context.

`cargo run --manifest-path src-tauri/Cargo.toml --example managed_runtime_smoke -- /opt/homebrew/bin/llama-server .local/models/Qwen3-1.7B-Q8_0.gguf` passed five checks: wrong certificate rejection, unauthenticated request rejection, cited generation (1,943 ms in the final run), stop/reap with closed connection, and credential rotation plus drop cleanup.

Current checks: 8 frontend tests and 16 Rust tests passed; Clippy and formatting passed. Two independent review rounds addressed endpoint identity and stale UI responses. Packaged UI controls rendered; the complete native file-picker workflow remains unverified because automation did not complete file selection. Automatic downloads and forced-app-exit cleanup remain pending.
