# Cachella

A local-first knowledge application for Windows and macOS. The current development build uses Tauri 2, React/TypeScript, SQLite FTS5, persisted semantic vectors and local llama.cpp engines.

## Current capabilities

- Import UTF-8 `.txt` and `.md` documents through a native file picker, up to 2 MB per file.
- Import selectable-text PDFs with physical-page citations and extracted-page navigation, up to 16 MiB and 200 pages. [PDF verification and limitations](evaluation/pdf/README.md).
- Import PNG/JPEG, WAV/MP3/FLAC/OGG and MP4/WebM through included or manually selected local FFmpeg tools; inspect stored previews and timestamped media matches.
- Persist and deduplicate documents, search passages, inspect original text and delete library entries.
- Download and select pinned Qwen3-1.7B Q4_K_M, Gemma 4 E2B Q4_0 with its projector, or EmbeddingGemma 2 Q8 model files with byte limits, SHA-256 verification and cancellation.
- Package a pinned llama.cpp engine for Apple silicon macOS and x86-64 Windows CPU inference, with an explicit included-engine selection.
- Start and stop a selected llama-server and GGUF model with a private authenticated TLS connection; or connect an independently managed loopback server.
- Generate answers, validate citation identifiers and display reported token/cache metrics.
- Cancel search, answers and active index batches; keep committed vectors for resume.
- Run a separate EmbeddingGemma 2 engine, resume text and media indexing, and use semantic retrieval for search and answer evidence.
- Optionally keep one managed engine loaded and switch on demand between search, indexing and answers.
- Keep application data in the platform application-data directory (`com.cachella.desktop`).

Managed Gemma media answers are implemented and verified on the synthetic macOS cases linked below. Native Windows runtime and installer verification remain unfinished. PDF OCR and original-layout rendering are outside the current text extraction path. LMCache is a future enterprise-server option. Local prompt reuse is llama.cpp caching.

## Run locally

Install Node.js 22+, Rust 1.89+ and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS.

```sh
npm ci
npm run desktop
```

`npm run dev` opens an interface preview. File import and inference require the desktop process; the browser preview does not emulate native behavior.

## Managed inference

Packaged builds select the included llama.cpp engine for new configurations. In Local runtime, existing workspaces can select **Use included engine** or choose a trusted `llama-server` executable. Select **Download and select model** for the tested Qwen3-1.7B Q4_K_M candidate, or choose your own compatible GGUF, then select **Start model**. The engine must support `--ssl-cert-file`, `--ssl-key-file`, `--no-webui`, and `LLAMA_API_KEY` (verified with build 10621 on macOS). The included b11531 engine is prepared from checksum-pinned official release archives. Review the model license before use.

For manually selected files, Cachella saves their locations without copying them. Each start creates an ephemeral loopback HTTPS endpoint, a new certificate trusted only by the native client, and a new API token. Credentials are excluded from the webview and saved configuration. Engine readiness has a 60-second limit after embedding file verification. **Stop model** and normal application exit close a private guardian lease that terminates and reaps the owned process. A packaged macOS force-kill check verified cleanup of both engines and their guardians; see the [process lifecycle report](evaluation/lifecycle/README.md). Windows execution, independent guardian termination and OS crashes remain unverified. The [included-engine preparation and compatibility report](evaluation/engines/README.md) records package provenance, platform coverage and remaining release checks. Saved included-engine selections resolve the current app resource path after a move or upgrade; manual engine choices remain unchanged.

In **Local runtime → Managed model memory**, enable **Keep one managed model loaded**
with both models stopped. Start each role once per session to allow automatic
switching during search, indexing and answers. A suspended role displays **On
demand**; its **Stop** button disables further automatic starts. The policy
persists across restart; session startup intent does not. Switching retains the
stored index, adds file-verification/startup time and discards the stopped engine's
prompt cache. The default permits both models to stay loaded. External servers
are excluded, and the residency limit gives no fixed RAM guarantee. See the
[one-model verification report](evaluation/residency/README.md).

## Verified model downloads

The generation card defaults to the pinned Unsloth Qwen3-1.7B Q4_K_M conversion (1,107,409,472 bytes). Its **Answer model** selector also offers ggml-org Gemma 4 E2B Q4_0 with a matching Q8 projector (3,398,849,248 bytes combined) for media answers. The semantic-search card offers ggml-org EmbeddingGemma 2 Q8 with its projector (864,676,480 bytes combined). The catalog identifies conversion publishers and Apache-2.0 model metadata. See [Qwen provenance](evaluation/model-provenance.json), [Gemma media-answer evidence](evaluation/media-answers/README.md) and [embedding provenance](evaluation/multimodal/provenance.json). These remain development candidates. Selecting a different generation model manually clears the previous projector; selecting a downloaded bundle installs its matching pair.

**Download and select model** explicitly starts external HTTPS requests to Hugging Face and its delivery network. Download requests contain no library documents or prompts. Only the built-in pinned catalog is accepted. The native client disables proxies and restricts redirects to the approved HTTPS hosts. Files must match the expected byte count, GGUF header and SHA-256 digest before publication in the application-data `models` directory. Existing files are reverified before reuse.

One installation runs at a time. **Cancel download** interrupts transfer or verification. Leaving Local runtime lets the download continue; reopening that page restores its status. Interrupted partial files restart on retry, and completed verified files are reused. Normal quit requests cancellation and waits for active-file cleanup. Startup and installation retry reclaim abandoned model partials in the pinned catalog directories. A per-revision OS lock preserves active downloads in other instances; directory handles keep recovery anchored if a pathname changes. Close older Cachella builds before upgrading because they do not participate in these locks. Recovery errors stop startup before desktop commands become available. See [download recovery verification](evaluation/recovery/README.md). Media/PDF scratch and runtime TLS files now use leased sessions below the OS temporary directory; macOS recovery preserves live parents and guardians and reclaims abandoned sessions on startup. Legacy anonymous temporary files are preserved. Windows cleanup now uses native handle-relative operations; its runtime qualification remains unfinished. Settings saves use leased staging beside the committed files, with atomic replacement and startup recovery. See [settings recovery verification](evaluation/settings-recovery/README.md). See the [Windows cleanup check report](evaluation/windows-scratch/README.md). See [scratch recovery verification](evaluation/scratch-recovery/README.md).

The downloaded files are selected only if the target runtime is stopped and its configuration still matches the configuration at download start. A newer manual selection is preserved; the download remains available for a retry. Model installation does not start an engine. The executable can come from the included engine or your trusted local installation. Download storage is additional to copies you previously obtained elsewhere; this feature does not delete those files.

## Local semantic search

In **Local runtime → Semantic search model**, choose an EmbeddingGemma 2 compatible llama-server, then use **Download and select model** to install its matching model/projector pair or choose those files manually. The included b11531 engine supports the pinned model. The older installed build 10621 cannot load it; the [model provenance and original source-build checks](evaluation/multimodal/README.md) record that compatibility boundary. Select **Start embedding model**, then **Index library**. Generation and embedding have separate owned processes and credentials.

Indexing saves each completed passage in SQLite. **Cancel indexing** cancels the active batch and stops scheduling further batches; each batch contains up to eight requests, each with a 120-second limit. Leaving the runtime page also requests cancellation. Completed passage vectors remain saved. Select **Index library** again to resume. Model and projector content hashes plus a versioned input format identify the embedding space. Changing weights at the same path causes reindexing on the next start. A single vector per passage bounds storage across profile changes; old-profile vectors are replaced as new indexing completes.

Enable **Use semantic search** beside the search box. Both search and answer generation use that retrieval mode. Semantic search requires every current passage to be indexed and the embedding engine to be running or armed for automatic startup in one-model mode. A newly imported document requires another indexing pass. Deleting a document also deletes its vectors. The index embeds up to 200 UTF-8 title bytes and the first 1,500 UTF-8 bytes of each passage; semantic queries accept up to 1,800 UTF-8 bytes. Full source text remains available for inspection. Search ranks similarity and does not establish that every returned passage answers the question.

Text, images, audio and video parts share the semantic index. The managed Gemma 4 E2B bundle can answer from up to four retrieved text passages, images, audio segments or sampled video parts, preserving source IDs and timestamps for inspection. Media segments are limited to 2 MiB each; requests contain at most 8 MiB of raw media. Qwen and external text-only connections continue to use text/Markdown/PDF evidence and show an inspection message for media-only results. Sampled video evidence does not establish continuous temporal understanding. Peak RAM and latency on consumer Windows devices have not been measured; [synthetic integration results](evaluation/media-answers/README.md) describe the current evidence scope.

## Local media import

The macOS decoder bundle selects its included FFmpeg/FFprobe by default. In **Local runtime → Media import**, you can select a trusted manual FFmpeg installation with FFprobe alongside, or return to **Use included decoder**. Existing manual settings are preserved. Import files from **Documents**. Each media file is limited to 32 MiB and audio/video to two minutes; a batch accepts up to five media files. SQLite stores normalized previews and embedding parts atomically, with a 512 MiB total media limit. Original imported files remain unchanged.

Images are resized to at most 512 pixels per side. Audio becomes 16 kHz mono PCM; indexing splits it into ten-second segments. Video previews are H.264/AAC MP4 at up to 720 pixels per side and 30 fps. Search samples a frame every five seconds and the soundtrack every ten seconds. Selecting a timed match seeks the player to that segment without autoplay. Short events between sampled frames may be missed; this checks independent visual/audio similarity and does not establish fused temporal understanding or provide a transcript.

Media decoding runs locally with bounded inputs, outputs and command deadlines. Each completed embedding is persisted; indexing resumes through **Index library**. Removing a library item removes its stored previews, parts and vectors. The supported decoder environment has been exercised on macOS; Windows execution remains a release gate.

Verify the production runtime, embedding client and persistent index against actual local weights:

```sh
cargo run --manifest-path src-tauri/Cargo.toml --example semantic_smoke -- /path/to/llama-server /path/to/embeddinggemma-2.gguf /path/to/mmproj.gguf
```

## Connect an independently managed server

Use an installed [llama.cpp server](https://github.com/ggml-org/llama.cpp/tree/master/tools/server) and a compatible GGUF model:

```sh
llama-server -m /path/to/model.gguf --host 127.0.0.1 --port 8080 -c 4096 -np 1
```

In Local runtime, save and check `http://127.0.0.1:8080`. The native client accepts direct loopback HTTP addresses, disables proxies and redirects, and limits each generation to 120 seconds. This alternate connection expects an already-running server. Stop the managed model before switching with Save & check. Model license and hardware suitability must be checked before distribution.

Use “Explore sample workspace” to add three explicitly synthetic documents. Search, then select “Generate answer”. Citation validation checks source membership; semantic accuracy still requires evidence review.

Select **Cancel answer** in the footer to cancel an active answer, including its semantic retrieval. The control remains available after changing views. Retrieved evidence stays available for inspection and retry. The app drops the request promptly; the engine may need additional time to stop its current compute batch. The bundled-engine probe observed its slot returning to idle after 1,308 ms. See [answer cancellation verification](evaluation/cancellation/README.md).

Select **Cancel search** in the footer to retain the previous search results and query. In Local runtime, **Cancel indexing** interrupts the active batch and refreshes its committed progress. **Index library** resumes pending passages. Leaving Local runtime also cancels indexing. A cancellation that reaches an already completed request preserves its result and reports that ordering. See [search and indexing cancellation verification](evaluation/search-index-cancellation/README.md).

## Checks

```sh
npm test
npm run build
npm run test:rust
npm run check:rust
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
```

`npm run test:rust` builds the process-guardian fixture before running library tests. Direct `cargo test` needs `cargo build --manifest-path src-tauri/Cargo.toml --example process_guard_fixture` first.

Run the actual-model integration check against a local server:

```sh
cargo run --manifest-path src-tauri/Cargo.toml --example inference_smoke -- http://127.0.0.1:8080
```

Verify managed TLS, authentication, real generation and process cleanup:

```sh
cargo run --manifest-path src-tauri/Cargo.toml --example managed_runtime_smoke -- /path/to/llama-server /path/to/model.gguf
```

Build an Apple silicon macOS development bundle with its included engine and media decoder (Python 3.11+, Xcode Command Line Tools, make and pkg-config are required):

```sh
npm run desktop:build:mac -- --debug --bundles app
```

This command verifies the engine archive, rebuilds FFmpeg and x264 from pinned source, and applies both engine and decoder resource overlays. Decoder resources include source archives, licenses and standalone rebuild scripts. Engine preparation verifies existing outputs; decoder preparation always builds fresh source. The decoder receipt records the local build and does not authenticate cached binaries. For offline source preparation, run `python3 scripts/prepare_decoder.py --sources /path/to/archives --replace`. See [decoder verification](evaluation/decoders/README.md).

`npm run desktop:build` retains the engine-only packaging path, including Windows. Windows decoder preparation uses MSYS2 UCRT64 as described below. Native Windows execution remains a release gate. The raw Tauri CLI with the base configuration builds a manual-engine development app.

Prepare the Windows decoder from the repository root in an MSYS2 UCRT64 shell:

```sh
pacman -S --needed python make pkgconf diffutils mingw-w64-ucrt-x86_64-gcc mingw-w64-ucrt-x86_64-nasm
/usr/bin/python3 scripts/prepare_decoder.py --platform windows-x86_64 --replace
```

Then build from the repository root in a Windows terminal with Node.js and the Rust/Tauri prerequisites:

```sh
npm run desktop:bundle:windows -- --debug --bundles nsis
```

The Windows source recipe includes pinned zlib and links its decoder dependencies statically. CI is configured to execute the production decoder smoke before bundling. Cross-compiling on macOS/Linux with MinGW-w64 and NASM uses `--platform windows-x86_64 --output /path/to/windows-decoder`; use a separate output from the macOS resources. Cross-compilation does not establish Windows runtime behavior.

Windows installers must be built and tested on Windows. Signing, installer validation and enterprise release are separate gates.

## Design and implementation status

- [Current local-first plan](docs/local-first-plan.html)
- [Implementation checklist and verification](tasks/todo.md)
- [Current acceptance audit and remaining requirements](docs/acceptance-audit.md)
- [Gemma text assessment](evaluation/gemma4-e2b-text-assessment.md)
- [Observed local inference results](docs/local-inference-verification.md)
- [Q4/Q8 model assessment and captured outputs](evaluation/assessment.md)
- [Small-model resource measurements and prompt reuse](evaluation/resources/README.md)
- [Release desktop memory comparison, including attributed UI helpers](evaluation/resources/release-desktop.md)
- [Multimodal runtime feasibility](evaluation/multimodal/README.md)
- [Historical Fly CPU proposal](docs/cachella-demo-plan.html)

No cloud resources are provisioned. Core local inference has no recurring hosted inference bill; device electricity, hardware and distribution costs remain separate.
