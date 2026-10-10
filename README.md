# Cachella

A local-first knowledge workspace with search across documents, images, audio and video, and cited answers from quantized models running on your computer.

Cachella is a development-stage desktop application built with Tauri 2, React, Rust, SQLite and llama.cpp. The current packaged targets are Apple silicon macOS and x86-64 Windows. Enterprise deployment and consumer-hardware qualification are ongoing.

## What works today

- **Bring local files:** import Markdown, text, selectable-text PDFs, PNG/JPEG images, WAV/MP3/FLAC/OGG audio and MP4/WebM video.
- **Search your library:** keyword search with SQLite FTS5 and persistent multimodal semantic embeddings.
- **Ask with evidence:** generate answers with source references, PDF page references and media timestamps.
- **Run local models:** download checksum-pinned models, start and stop the included llama.cpp engine, and inspect reported token and prompt-cache usage.
- **Control resource use:** optional one-model-at-a-time operation, cancellable downloads/indexing/answers, and resumable committed indexing progress.

The default answer candidate is Qwen3-1.7B Q4_K_M. Optional Gemma 4 E2B provides media answers; EmbeddingGemma 2 supplies multimodal embeddings. Model weights download separately and retain their publisher terms. The [development guide](docs/development.md) explains setup, download sizes and model compatibility.

Local prompt reuse uses llama.cpp caching. LMCache integration remains proposed work for a shared inference server.

## Start developing

Install Node.js 24 (the CI version), Rust 1.89 or later, Python 3.11 or later, and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your operating system. CI uses stable Rust with Clippy and rustfmt.

```sh
git clone https://github.com/henryph24/cachella.git
cd cachella
npm ci
npm run desktop
```

The repository is currently private; cloning requires collaborator access until publication. This development command opens the desktop app. Included engine and decoder resources require the packaging preparation described below; a development app can also use trusted manually selected local binaries.

For a browser-only interface preview:

```sh
npm run dev
```

Native import and inference require the desktop app. In a prepared build, open **Local runtime**, select the included engine, download a model, and start it. Import files in **Documents**; start the embedding model and select **Index library** for semantic search. **Explore sample workspace** adds explicitly synthetic text documents.

## Build and check

```sh
npm test
npm run build
npm run test:rust
npm run check:rust
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
python3 -m unittest discover -s scripts -p 'test_*.py'
```

The Rust test command first builds its process-guardian fixture. Real-model checks require separate model downloads.

For an Apple silicon development bundle, install Xcode Command Line Tools, make and pkg-config, then run:

```sh
npm run desktop:build:mac -- --debug --bundles app
```

Windows packaging requires MSYS2 UCRT64 decoder preparation followed by the NSIS build. Follow the [platform build instructions](docs/development.md#checks); the [CI workflow](.github/workflows/desktop-checks.yml) records the exact Windows toolchain steps. Packages are development artifacts; signing and public releases remain pending.

## Data and privacy

Imported content, normalized media previews and embeddings are stored locally. Managed inference runs through an authenticated loopback connection. Model downloads make explicit HTTPS requests to Hugging Face and its delivery network; these download requests contain no library documents or prompts.

Notion, Google Drive and watched Obsidian vaults are planned. Composio is being evaluated for cloud account connections. Cloud connectors would introduce external data processing and require separate permission, deletion and retention controls. See the [source integration plan](docs/data-sources.md).

## Current limits

- PDFs require selectable text; OCR and original-layout rendering are outside the current implementation.
- Media files are limited to 32 MiB; audio/video to two minutes. Video retrieval uses sampled frames and audio segments.
- The library currently caps at 500 documents, 10,000 passages and 512 MiB of stored PDF/media assets.
- Citation membership checks do not establish answer correctness. Existing model evaluations use synthetic fixtures and small rubrics.
- Memory and latency depend on hardware, model and workload. The intended 16 GB consumer-device workload remains unqualified.

On 10 October 2026, [Windows CI](https://github.com/henryph24/cachella/actions/runs/38028684256) passed 151 Rust tests, one separately invoked reparse test, frontend checks, lint, decoder smoke and NSIS packaging. The subsequent installer/runtime qualification failed at `WaitForInputIdle`; Windows desktop launch and real-model acceptance remain open. macOS execution evidence is available in the [evaluation index](evaluation/README.md).

## Contributing and project status

Read [CONTRIBUTING.md](CONTRIBUTING.md) for development checks and contribution scope. The [HTML technical plan](docs/local-first-plan.html), [acceptance audit](docs/acceptance-audit.md) and [open-source readiness checklist](docs/open-source-readiness.md) describe intended behavior and remaining gates.

## Licensing

A license for Cachella's own source is awaiting the maintainer's selection. Public open-source release remains pending. Third-party code, bundled executables and downloaded models retain their own licenses; see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md). The decoder build enables GPL components and requires its accompanying licensing and source materials when distributed.
