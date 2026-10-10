# Pinned model installation verification

The production installer accepts two fixed model bundles. It verifies exact byte counts, GGUF headers and pinned SHA-256 hashes before publishing files. Model selection and engine startup are separate steps.

## Reproduce download and reuse

```sh
cargo run --release --manifest-path src-tauri/Cargo.toml --example install_models_smoke -- /path/to/model-directory embeddinggemma2-q8 embedding-download.json
cargo run --release --manifest-path src-tauri/Cargo.toml --example install_models_smoke -- /path/to/model-directory embeddinggemma2-q8 embedding-reuse.json
```

The first invocation downloads missing files from the pinned Hugging Face revision. The second verifies the existing files. Use `qwen3-1.7b-q4` for the generation bundle. Each report records observed phases, file sizes, elapsed time and terminal progress. The example calls the application's production installer; it leaves verified files in the requested directory.

## Recorded evidence: 2026-10-10

- [Embedding download](embedding-download.json): 2 files, 864,676,480 bytes, 200,054 ms. Both downloading and verification phases observed.
- [Embedding reuse](embedding-reuse.json): the same 2 files verified in 2,078 ms; only verification phases observed. These are individual integration observations on the development Mac, with no general throughput claim.
- `npm run test:rust`: 64 passed, 0 failed, including checksum/header/size rejection, partial cleanup, cancellation/retry, existing-file preservation, symlink rejection, destination publication collision, selection conflict, failed settings save and shutdown coordination.
- `npm test -- --run`: 25 passed across 6 files. Download UI covers progress, cancellation, selection callbacks and selection errors after native file verification.
- `npm run build`: TypeScript and Vite succeeded, 1,749 modules transformed. Direct Homebrew Clippy with `--all-targets --features desktop -- -D warnings` exited 0.
- Independent review completed in 2 rounds. The first identified exit preceding asynchronous partial cleanup. The regression first failed; the implementation now blocks normal exit until active installer cleanup completes and permanently rejects new jobs after shutdown begins. The final reviewer found no remaining demonstrated material defect.
- `npm run tauri build -- --debug --features desktop --bundles app`: `Finished 1 bundle`, 39.33 MiB.
- Packaged macOS quit check: started Q4 download, observed a 15,053,780-byte `.part` file and application PID 12045, then invoked normal Quit. A read-only filesystem/process assertion returned `PASS: native quit removed active partial and process 12045 exited`.
- Relaunch preserved all 6 documents and 10 passages. A new Q4 download started successfully; leaving and reopening Local runtime showed continuing progress.

- [Q4 reuse](qwen-reuse.json): the native download produced a verified 1,107,409,472-byte file; a separate production-installer reuse check verified it in 2,668 ms. The initial sampled `downloading` phase in this reuse report precedes existing-file inspection and does not establish a network transfer.
- [Native results](native-results.json): the app selected the downloaded Q4 file, started the chosen engine and displayed `Running` and `Connected`. The synthetic budget question returned the correct answer and its Infrastructure budget citation in a displayed 1.5 seconds, with 363 input tokens and 0 cached tokens. This single observation establishes integration only.
- The native semantic-model button reverified and selected both previously downloaded embedding files while preserving the existing engine executable. Read-only settings checks confirmed both configurations were saved. Final inspection found 6 documents, 10 chunks, 10 embeddings, 0 partial files and 0 app child processes after Stop model.

## Verification limits

Embedding execution with the identical pinned weights was verified in the preceding integration work; this increment checks the downloaded pair selection and preserves its engine. The embedding engine was not restarted for this installer check. Windows runtime/signing, automatic engine installation and forced-kill orphan recovery remain unverified or unfinished. Hash verification establishes downloaded artifact identity; model behavior and licensing qualification require their separate assessments. No private library data is submitted by this downloader.
