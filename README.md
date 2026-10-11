# Cachella

**Source alpha candidate:** experimental software for developers and invited testers.
The repository is currently private; public source licensing and signed installer
publication are still being prepared. See the [alpha release notes](docs/alpha-release.md)
and [changelog](CHANGELOG.md).

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

## Install on macOS

**Release status:** there is currently no published end-user installer. The repository is private, and existing app bundles are development builds. Signing, notarization and clean-Mac installation testing remain release requirements. Obtain a test build directly from the maintainer if you are participating in testing. The steps below describe a prepared macOS build with its engine and decoder included.

### Before you start

- **Mac:** Apple silicon (M-series). An Intel Mac package is not currently provided.
- **macOS:** the current local test environment is macOS 14.4. The bundled engine declares macOS 13.3 as its minimum; the full application's supported OS range still needs release qualification.
- **Storage:** allow space for the app, your imported library and model downloads. The default answer model downloads about 1.11 GB; semantic search adds about 0.86 GB. Optional media-answer weights add about 3.40 GB. These are decimal download sizes; leave additional space for temporary downloads and your files.
- **Memory:** no consumer minimum is certified yet. Recorded Mac testing used 36 GiB RAM; 16 GB qualification is pending. See the memory-saving option below.
- **Internet:** needed to obtain the app and download models. Once the required models are installed, local import, search and inference can operate offline.

A prepared app includes its inference engine and media decoder. End users do not need Node.js, Rust, Python, Homebrew or a separately installed FFmpeg to run that app.

### Install and open

1. Obtain the Apple silicon macOS package from the maintainer. For a future public release, use only the project's [GitHub Releases](https://github.com/henryph24/cachella/releases) and check the release notes for supported macOS versions and published verification instructions.
2. If supplied a ZIP, extract it in Finder. If supplied a disk image, open it. Drag **Cachella.app** into **Applications**. Development test builds may have a different name specified by the maintainer.
3. Open the app from **Applications**. Confirm macOS's normal first-open prompt after checking its source.
4. If macOS reports an unidentified developer, a damaged app or a malware warning, stop and contact the maintainer. Public distribution must pass the signing and notarization gates; do not disable Gatekeeper or remove quarantine attributes to work around an unexpected warning. See [Apple's app-opening guidance](https://support.apple.com/en-us/102445).

No Cachella account or cloud-provider API key is required for the current local features. The app should not require Full Disk Access, Accessibility, microphone or camera permission for importing existing files. Select only the files you intend to import through the file picker.

### First run: get a text answer

1. Open **Local runtime**. In the answer-model section, confirm that the included engine is selected. **Use included engine** appears when an alternative engine is selected and a bundled engine is available.
2. Keep the default **Qwen3-1.7B Q4_K_M** answer model. Select **Download and select model** and wait for download and verification to finish. This retrieves weights from Hugging Face and its delivery network.
3. Select **Start model** and wait for it to become ready. Downloading a model does not start it.
4. Open **Documents → Import files** and select a small `.txt`, `.md` or selectable-text PDF. Alternatively, **Explore sample workspace** adds three synthetic demonstration documents.
5. Open **Ask your library**, leave **Use semantic search** unchecked, enter a word or phrase from your document, and run the search. Select **Generate answer** after results appear.
6. Open the cited sources to check the answer against the imported content. Model answers can be incorrect even when the citation identifier is valid.

Keyword search works without a running model. Generating answers requires the answer model.

### Enable semantic and media search

1. Open **Local runtime → Semantic search model**. Use the included engine and select **Download and select model** to install EmbeddingGemma 2 and its matching projector.
2. Select **Start embedding model**, then **Index library**. Wait for all current passages to finish indexing. Completed passages persist if indexing is cancelled or interrupted; select **Index library** again to resume.
3. In **Ask your library**, enable **Use semantic search**, then search in natural language. Run **Index library** again after importing new files.
4. For images, audio or video, confirm the included decoder under **Local runtime → Media import**, then use **Documents → Import files**. Supported formats and input limits are listed below.

The default Qwen model answers from text/PDF evidence. For answers using retrieved images, audio or sampled video, stop the answer model, select the optional **Gemma 4 E2B** bundle in **Answer model**, download it and start it. Its matching projector is selected with the bundle. Media retrieval and media-answer generation are separate capabilities.

### Reduce memory use

Stop both managed models, then enable **Local runtime → Managed model memory → Keep one managed model loaded**. Start each role once during the session to allow automatic switching. An **On demand** role starts when needed; **Stop** disables that role's automatic startup. Switching adds model-loading time and clears the stopped model's prompt cache. This setting does not guarantee that a particular Mac has enough memory.

### Troubleshooting

| What you see | What to do |
| --- | --- |
| Browser preview banner; imports unavailable | Open the installed desktop app. The browser preview is for interface development. |
| No included engine or decoder | The package is incomplete or was built without bundled resources. Obtain a prepared build from the maintainer. |
| Download fails or is cancelled | Check your connection and available disk space, then retry. Completed verified files are reused; an incomplete file restarts. |
| Start model is disabled | Finish any download and select the required model files. Semantic search also requires its projector. |
| Model fails to start | Check the displayed error, use the included engine and catalog model pair, and stop other model roles to reduce memory pressure. Report your macOS version, chip and model name. |
| Semantic search says indexing is incomplete | Start the embedding model and run **Index library** after all imports complete. |
| No PDF text appears | The PDF may be scanned. OCR is not currently included; use a document containing selectable text. |
| Context or generation limit error | Ask a shorter, more specific question. Existing search results remain available for inspection. |
| App-data access-policy error | Ask your administrator or the maintainer to review the app-data folder permissions. Custom macOS ACLs are rejected before the library is opened; keep a backup and avoid blanket permission changes. |
| File rejected | Check the format, size, duration and library limits below. A cloud-only file may need downloading to your Mac first. |

For support, provide the app version, macOS version, chip, model name and a reproducible example using non-sensitive data. Remove document content, usernames and private paths from shared logs.

### Your data, backups and uninstalling

The normal app stores its library, settings and downloaded models under:

```text
~/Library/Application Support/com.cachella.desktop/
```

Development test builds may use a separate identifier and folder. Imported content and stored previews are copies; deleting a library entry leaves the original file in its original location. Manually selected model files also remain in their original locations.

To back up, quit Cachella completely and copy the entire app-data folder to a protected backup location. A backup can include sensitive source content, embeddings and large model files. Backup/restore across versions has not been qualified.

To uninstall, quit Cachella and move the app from **Applications** to Trash. This preserves the app-data folder. To erase the library and downloaded models as well, use Finder's **Go → Go to Folder** with the path above, confirm the correct app identifier, and remove that folder only after saving anything you need. This removes the app's stored copies and settings; backups and manually selected external files require separate handling. File deletion does not promise forensic secure erasure.

## For developers

Source builds require Node.js 24, Rust 1.89 or later, Python 3.11 or later, and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/). Clone access currently requires permission to the private repository.

```sh
git clone https://github.com/henryph24/cachella.git
cd cachella
npm ci
npm run desktop
```

This starts a development app; bundled resources require separate preparation. See the [development guide](docs/development.md) for engine setup, macOS/Windows packaging, tests and real-model checks.

## Data and privacy

Imported content, normalized media previews and embeddings are stored locally. The database has no application-level encryption; protect your Mac account and backups, and enable FileVault according to your organization's policy. Managed inference runs through an authenticated loopback connection. Model downloads make explicit HTTPS requests to Hugging Face and its delivery network; these download requests contain no library documents or prompts.

Notion, Google Drive and watched Obsidian vaults are planned. Composio is being evaluated for cloud account connections. Cloud connectors would introduce external data processing and require separate permission, deletion and retention controls. See the [source integration plan](docs/data-sources.md) and [security policy](SECURITY.md).

## Current limits

- PDFs require selectable text; OCR and original-layout rendering are outside the current implementation.
- Media files are limited to 32 MiB; audio/video to two minutes. Video retrieval uses sampled frames and audio segments.
- The library currently caps at 500 documents, 10,000 passages and 512 MiB of stored PDF/media assets. Search passage previews are capped at 8 KiB and mark truncation; open the source for its full text.
- Citation membership checks do not establish answer correctness. Existing model evaluations use synthetic fixtures and small rubrics.
- Memory and latency depend on hardware, model and workload. The intended 16 GB consumer-device workload remains unqualified.

On 11 October 2026, [Windows qualification](docs/windows-runtime-qualification.md) passed release NSIS installation, desktop readiness and clean close, installed PDF/guardian helpers, Qwen3 inference, and five synthetic text/image/audio/video retrieval checks on Windows Server 2025. The run also passed 154 Rust tests plus one separately invoked reparse test, 67 frontend tests, lint and 11 decoder fixtures. Windows 11 standard-user installation, interactive import flows, GPU behavior and consumer-hardware performance remain unqualified. macOS execution evidence is available in the [evaluation index](evaluation/README.md).

## Contributing and project status

Read [CONTRIBUTING.md](CONTRIBUTING.md) for development checks and contribution scope. The [HTML technical plan](docs/local-first-plan.html), [acceptance audit](docs/acceptance-audit.md) and [open-source readiness checklist](docs/open-source-readiness.md) describe intended behavior and remaining gates.

## Licensing

A license for Cachella's own source is awaiting the maintainer's selection. Public open-source release remains pending. Third-party code, bundled executables and downloaded models retain their own licenses; see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md). The decoder build enables GPL components and requires its accompanying licensing and source materials when distributed.
