# Cachella implementation plan

Current requirement-level status: [acceptance audit](../docs/acceptance-audit.md). Historical increment notes below retain their original verification scope.

## Active media-answer feasibility: 2026-10-10

- [x] Trace media retrieval through generation: the original answer path excluded media hits; embeddings provide no transcript/caption.
- [x] Prepare isolated six-case image/audio engine probe; compile/help and exited-engine cleanup checks passed.
- [x] Add optional macOS engine-memory snapshots using the existing accounting helper; validate three counters and failure cleanup.
- [x] Enable validated optional generation projectors at the managed-runtime boundary; regression failed first, then 134 Rust tests passed, all-target desktop Clippy and formatting passed. Subsequent UI selection and payload integration are recorded below.
- [x] Add native media probe using production TLS/authentication and process guardian, plus existing process-group memory collector. Build, Clippy, formatting, Python compilation/help and managed startup-failure cleanup passed; subsequent real-model runs completed.
- [x] Complete pinned Gemma 4 E2B Q4_0 and Q8_0 projector download; verify 3,398,849,248 total bytes and both publisher hashes.
- [x] Run and review image/audio answers and abstention with initial process-memory measurements: direct 5/6 fixture rubrics, structured 6/6 plus 6/6 production validator acceptance. Both owned process groups empty. Consumer default selection remains contingent on broader quality and whole-app measurement.
- [x] Integrate bounded media evidence, citations and model/projector lifecycle with appropriate regression checks.
- Backend integration passes seven real-model requests, including two video-only calls, with source metadata/citations preserved. Optional Gemma setup, model-switch persistence and status-before-catalog cancellation/busy fixes are implemented. Two independent review rounds completed; findings were fixed and verified. Final checks: 138 Rust tests, 57 frontend tests, frontend build, formatting and all-target desktop Clippy passed. Production installer reused and verified both files (3,398,849,248 bytes). Packaged UI validation subsequently passed, including the mixed-media correction; see `evaluation/media-answers/native-normalized.md`. Earlier backend evidence:  `evaluation/media-answers/integration-video.json` and `installer-reuse.json`.
- Evidence and limitations: `evaluation/media-answers/README.md`. Previous question-only turn changed no implementation state; this continuation adds the runnable feasibility probe and starts a live download. Production defaults remain unchanged.

## Objective
Build the agreed local-first enterprise knowledge application for Windows and macOS, using Tauri + TypeScript, a small quantized model through llama.cpp, local storage and built-in prompt caching. Preserve text/image/audio/video search as the intended product scope. LMCache is an optional future enterprise-server integration; do not imply local caching is LMCache.

## Scope assumptions
- “Go” authorizes implementation; an optional clarification is pending.
- No paid services, deployment, publication or external communications.
- Current machine: Apple silicon, 36 GiB RAM; Windows execution requires a Windows runner.
- Cross-platform source and local macOS verification are distinct from verified Windows delivery.

## Work and acceptance
- [x] Inspect workspace, tools and current plan. Only a Fly CPU HTML proposal exists.
- [x] Replace current architecture guidance with the local-first decision and retain the older plan as historical.
- [x] Build Tauri + React/TypeScript application shell and native Rust boundary.
- [x] Implement persistent SQLite document ingestion, chunking, search, deletion and source inspection.
- [x] Implement local inference connection, bounded requests, grounded answers, citation validation and cache metrics.
- [ ] Implement model setup and lifecycle for packaged desktop use.
- [ ] Implement native multimodal embedding integration and text/image/audio/video ingestion, with timestamp/page citations.
- [ ] Implement ingestion resumption, bounded resources and cache invalidation.
- [ ] Verify actual model behavior, local persistence, failure paths and desktop UI.
- [ ] Independently review security/process/concurrency boundaries; confirm findings and resolve in at most two rounds.
- [ ] Package macOS and configure Windows build checks; clearly report any unverified Windows runtime or signing.
- [ ] Audit full scope against actual evidence before marking the goal complete.

## Verification log
- Workspace inspected: no existing application code or project lessons.
- Toolchain available: Node 24.11.1, Rust 1.91.1, cargo 1.91.1, llama-server build 10621.
- Test-first for deterministic data, inference boundary and cache behavior. UI verification after implementation because visual checks require rendered views.

## Known constraints
- EmbeddingGemma 2 now passes native text/image/audio and decomposed-video feasibility on macOS with a pinned newer llama.cpp source build. Desktop text/image/audio/video semantic retrieval is integrated; Windows execution remains unverified.
- Smoke-test timing and cache counters are recorded below; no general performance/quality claims.
- Desktop signing and enterprise distribution are outside a local development smoke test.

## Verification update: 2026-10-09
- Previous discussion turn provided current caching documentation, but changed no implementation state. Resumed from the actual worktree and live download handle.
- `npm test`: 4 passed (search/source bridge, generation failure, stale engine status, deletion confirmation). Corrected test selectors after observed accessible-name whitespace differences.
- `npm run build`: passed; 1,744 modules transformed.
- `npm run test:rust`: 13 passed, 0 failed, including desktop boundary regression. The script now enables the desktop feature so CI executes that regression.
- Final independent review confirmed four prior fixes and found a remaining deletion/generation race. Regression failed before the fix; deletion now acquires the same semaphore held by generation. The regression passes and document retention/deletion are asserted.
- `/opt/homebrew/opt/rust/bin/cargo-clippy clippy --manifest-path src-tauri/Cargo.toml --all-targets --features desktop -- -D warnings`: passed.
- Official Qwen3-1.7B Q8_0 downloaded (1,834,426,016 bytes). SHA256 matches publisher metadata: `061b54daade076b5d3362dac252678d17da8c68f07560be70818cace6590cb1a`.
- Local llama-server listening at 127.0.0.1:18080 with 4,096-token context and one slot. This Q8 model is an integration test candidate; 4-bit quality and Windows behavior remain unverified.
- Previously inspected macOS app supports importing sample documents, searching and opening source text. A fresh package will be required after Rust changes.
- Windows CI workflow exists locally; no remote execution or Windows runtime evidence exists.
- Actual model check: `cargo run --manifest-path src-tauri/Cargo.toml --example inference_smoke -- http://127.0.0.1:18080` returned `PASS: two real local generations returned valid source references.` Cold request: 2,102 ms, 0 cached tokens; warm request: 1,846 ms, 192 cached of 193 prompt tokens; both 112 completion tokens. This is a two-request integration observation, not a benchmark or general savings claim. Answers correctly identified the permission bug in the synthetic evidence.
- Native UI check: saved loopback endpoint, connected to Qwen3-1.7B, generated a grounded answer from three synthetic documents in displayed 2.9 seconds (487 input tokens, 76 cached), and clicked the answer citation to open the original Launch decision document. This ran in the existing development app session.
- Fresh macOS package after the deletion fix: `npm run tauri build -- --debug --features desktop --bundles app` exited 0, `Finished 1 bundle`, 35.77 MiB. New bundle has not yet been relaunched for a full installer/runtime test.
- Rust format check passed with exit 0. No Windows execution, signing, managed runtime, or multimodal integration was verified.

## Managed runtime implementation: 2026-10-09
- Prior goal turn was progress: real local generation and cache reuse verified; deletion race fixed. This turn adds selected-file model lifecycle through native commands and UI.
- Native lifecycle: select trusted executable and GGUF, persist locations, start, await readiness, stop, detect process exit and clean up on normal app exit. Commands use argument arrays and an environment allowlist. Startup and generation share a concurrency guard.
- Security review found a released-port race and stale frontend responses. Managed connections now require per-run certificate-pinned TLS plus a fresh API token. Frontend operation/revision guards discard stale responses. Final bounded review found no additional material issue after private-key cleanup was fixed.
- Initial process regression failed against the stub, then passed. Tests cover invalid configuration/model, duplicate start, child stop/exit, blocked fallback, readiness, and credential-file cleanup.
- Managed real-model smoke passed with Qwen3-1.7B Q8_0: wrong certificate rejected, unauthenticated request rejected, cited answer in 1,826 ms, child reaped, port closed, credentials rotated and owner-drop cleanup. This is a single integration observation, not a performance benchmark.
- Packaged managed controls rendered on macOS. Native picker automation did not complete; canceled without choosing a file. Full select/start/stop UI flow remains unverified. Core managed process flow was verified through the Rust example.
- Still pending: automatic downloads, bundled engine distribution, full native lifecycle UI verification, forced-exit cleanup, 4-bit evaluation, multimodal indexing and Windows execution.
- Final checks: `npm test` reports 8 passed; `npm run test:rust` reports 16 passed; Clippy with warnings denied and format check passed. README/checklist formatting and local links passed deterministic validation. Documentation changes are exempt from test-first.
- Final managed smoke after readiness/cleanup fixes: 5 PASS lines, including mismatched certificate rejection and a cited answer in 1,943 ms. The runtime withholds managed connections until startup authentication checks complete and does not fall back after stop or crash.

## Four-bit evaluation and abstention handling: 2026-10-09
- Prior turn was progress: managed lifecycle implemented and tested. This turn evaluates a pinned 4-bit candidate against the Q8 baseline and fixes a demonstrated abstention presentation defect.
- Downloaded Unsloth Qwen3-1.7B Q4_K_M at revision `d7f544eead698dbd1f15126ef60b45a1e1933222`; 1,107,409,472 bytes, SHA256 `b139949c5bd74937ad8ed8c8cf3d9ffb1e99c866c823204dc42c0d91fa181897` verified. Provenance records the third-party publisher and Apache-2.0 model-card metadata.
- Added eight synthetic cases and a real-model evaluation example using production retrieval/inference. Captured 64 total request records across Q4/Q8 baseline/current reports; each report has 16 records, zero request errors, two abstention mismatches.
- Agent rubric review of current outputs: seven of eight cases met for each variant. Both models followed the embedded instruction in the historical baseline while setting insufficient evidence. Fixed output normalization suppresses those claims and clears answer citations, while preserving source inspection. The injected case still fails to answer the legitimate question.
- Test-first regressions reproduced the false-claim display and blank/oversized/fabricated-citation abstention edge. `npm run test:rust`: 18 passed with desktop feature. Independent review completed in two bounded rounds; core-only review ran 17 tests, matching 18 with the desktop regression included. Clippy and format checks passed.
- Four-bit execution is now verified on the M3 Pro. Peak RAM, Windows, general quality and injection resistance remain unverified. See `evaluation/assessment.md`; neither candidate is qualified for enterprise release by this eight-case sample.
- Final artifact validation: 8 unique cases, 4 reports, 64 records, expected case/cache pairs, derived error/abstention counts and documentation links/formatting passed. Documentation is exempt from test-first. Fresh macOS package built successfully: 1 bundle, 36.70 MiB.

## Multimodal runtime feasibility: 2026-10-09
- Previous turn was progress: Q4/Q8 evaluation captured and abstention handling fixed. This turn verifies an actual multimodal runtime before product integration.
- Google EmbeddingGemma 2 model card publishes Apache-2.0 metadata. Jina Omni ONNX candidate publishes CC BY-NC 4.0 metadata; selected Gemma for this native feasibility path.
- Verified SHA256 and sizes for Q8 text weights (309,855,456 bytes) and Q8 vision/audio projector (554,821,024 bytes), from pinned ggml-org model revision `bfcd298762cc34d0357ece5ebdd31791a3a374d8`.
- Installed llama-server build 10621 failed with `unknown model architecture: gemma-embedding2`. Built the official supporting commit `4fbc76dec51d0add466f0210855c0596589b60d4` in `.local/llama-embedding-build`; `cmake --build ... --target llama-server -j 6` reported `Built target llama-server`. Existing generation engine was not replaced.
- Synthetic fixtures: two PNGs, two speech WAVs, one MP4 with separately extracted frame and audio. Probe invokes the real native model; no mocked embeddings. All 13 embeddings were 768-dimensional normalized finite vectors. Five of five two-candidate retrieval checks passed, covering text/image/audio/video frame/video soundtrack.
- Initial media request exposed a per-process server marker difference from the default C API marker. Corrected the probe to read `/props` and validate advertised modalities plus marker.
- `python3 evaluation/multimodal/probe.py --fixtures evaluation/multimodal/fixtures --output evaluation/multimodal/results.json` exited 0; captured all 5 passing cases. `python3 -m py_compile evaluation/multimodal/probe.py` passed. Report counters and fixture hashes verified. This exploratory runtime probe uses test-after because its purpose is measuring actual model/API compatibility.
- Product remains text/Markdown + keyword retrieval. Authenticated managed embedding runtime, persisted/versioned vectors, multimedia import/navigation, resource bounds/resumption and Windows runtime verification remain required. The video proof uses independent frame and soundtrack embeddings; it does not prove fused temporal understanding.
- Stopped the owned synthetic-fixture server after verification; TCP connection to 127.0.0.1:18081 failed as expected. No embedding service is left listening.

## Native semantic retrieval integration: 2026-10-09
- Previous discussion turn was no implementation progress. Inspected the authoritative workspace and continued from the verified multimodal runtime.
- Added a separately managed, certificate-pinned and authenticated embedding process with GGUF/projector selection, persisted configuration, startup compatibility probe and normal-exit cleanup. Existing generation controls retain their API behavior.
- Added persisted normalized 768-dimensional vectors, content-hashed model/projector profiles, bounded pending batches and per-passage commits. Search and answer generation both use the explicitly selected retrieval mode. An incomplete index produces an actionable error.
- Frontend indexing supports pause after the current batch and restart/resume. Leaving the runtime view stops scheduling new batches. Text and Markdown are connected to the new index; media import and timestamp navigation remain pending.
- Test-first parser regression failed on the unimplemented parser; the new embedding-control test failed before its UI was wired. Store tests were added before store APIs. Exploratory runtime and additional UI checks used test-after to verify final integration.
- Actual-model check: `cargo run --manifest-path src-tauri/Cargo.toml --example semantic_smoke -- .local/llama-embedding-build/bin/llama-server .local/models/embeddinggemma-2-Q8_0.gguf .local/models/mmproj-embeddinggemma-2-Q8_0.gguf` produced five PASS lines: authenticated TLS, 2 vectors persisted across reopen, correct top-1 paraphrase retrieval among 2 sources, deletion cleanup, process/port cleanup. This is a synthetic integration check, not a general quality or performance benchmark.
- Remaining full-scope work: desktop image/audio/video/PDF import and citations, engine/model distribution, forced-exit cleanup, full native picker workflow, Windows execution/signing, broader quality/resource measurements and final requirement audit.
- Native UI exposed a legacy database key mismatch: `(chunk_id, profile)` could not satisfy `ON CONFLICT(chunk_id)`. Added an IMMEDIATE transaction migration preserving one deterministic vector per chunk and source documents. Regression first failed against the old schema; migration, idempotent reopen, replacement write, delete cascade and failure rollback now pass. Native startup upgraded the existing database and retained all 3 synthetic source documents.
- Independent review completed in two bounded rounds, including the migration. No remaining material finding was reported. The added rollback regression confirms a failed foreign-key copy preserves the original table/data and leaves no replacement table.
- Final Rust suite: `npm run test:rust` reports 27 passed, 0 failed. Frontend suite: `npm test` reports 12 passed, 0 failed. `npm run build` passed; Clippy with warnings denied and Rust format check passed. Latest macOS package: `npm run tauri build -- --debug --features desktop --bundles app` reports `Finished 1 bundle`, 37.10 MiB. Windows remains unverified.
- Native packaged UI verification: selected executable/model/projector with native pickers, persisted the selections across restart, started the engine, indexed `3 of 3 passages`, and retrieved Infrastructure budget as the first semantic result. The real existing library migrated without a manual database reset.
- Native semantic answer completed in displayed 5.5 seconds with 486 input tokens and 76 cached tokens; clicking its Infrastructure budget citation opened the stored original text. This single observation includes retrieval/generation overhead and is not a benchmark. The embedding model was stopped through its native control after verification.

## Native media ingestion and retrieval: 2026-10-09
- Prior turn was progress: native semantic retrieval, persistent vectors and source citations passed. Current work preserves text/image/audio/video scope.
- [x] Add bounded offline image/audio/video normalization using selected trusted FFmpeg tools; preserve original files and timestamped segments.
- [x] Persist local media previews and embedding parts atomically with resource limits, deduplication and delete cascades.
- [x] Embed image/audio parts through the owned authenticated runtime, discover its per-process marker and retain model-profile isolation.
- [x] Wire native media imports, media preview/playback, timestamp navigation and semantic retrieval; keep media matches out of text-only generation evidence.
- [x] Verify decoding, DB upgrades/failures, protocol ranges, frontend behavior, actual model retrieval and native packaged flow; independently review boundaries.

- Previous discussion turn changed no implementation state. This continuation verified the pending decoder/store fixes and resumed native integration checks.
- Decoder hardening bounds opened-file reads, FFmpeg outputs/allocations/resolution/time and child lifecycle. Cancellation and normal shutdown kill and reap owned decoder processes; Windows environment essentials are preserved.
- Store boundary rejects unsupported normalized MIME, parts above 2 MiB and invalid/over-duration timestamp intervals before writes. Media storage and deletion share the document transaction/cascade boundary.
- `npm run test:rust`: `45 passed; 0 failed`. `npm test -- --run`: `Tests 17 passed (17)`. `npm run build`: `1748 modules transformed`. Direct Homebrew cargo-clippy with warnings denied and cargo-fmt check exited 0.
- Final independent review (round 2) found no remaining demonstrated material issue in the requested media boundaries. Reviewer reran 45 Rust and 17 frontend tests. Parent ran Clippy through the installed Homebrew binary because the review environment's rustup shim had no default toolchain.
- Production `multimodal_smoke` example with pinned local weights returned 11 PASS lines: five media decodes/range previews, five retrieval comparisons, persistence/timestamps/media-only generation guard/deletion/process cleanup. Report: `evaluation/multimodal/native-results.json`, seven indexed parts. These synthetic checks establish integration only.
- `decode_smoke` returned six PASS lines for JPEG, MP3, FLAC, OGG, WebM and a 12-second temporal MP4. The temporal fixture yields five parts, including a frame and audio segment at 10 seconds.
- `npm run tauri build -- --debug --features desktop --bundles app`: `Finished 1 bundle`, 38.12 MiB. The native picker selected installed FFmpeg and imported the temporal fixture into the existing library, retaining its three text sources.
- Documentation updates are exempt from test-first. Model distribution, Windows execution/signing, consumer resource measurements, broader retrieval/answer qualification, PDF and media-to-text answers remain unfinished.
- Packaged native UI: selected FFmpeg through its native picker; imported MP4, WAV and PNG fixtures; existing three text sources remained. Semantic query `A blue square on a white background` ranked the video frame at 10 seconds first. Clicking it and Play produced `Elapsed: 10 seconds`, `Duration: 12 seconds`, and the blue-square frame. Audio exposed a three-second player and changed Play to Pause with advancing elapsed position. PNG rendered the red circle through the stored-asset protocol.
- Native index initially completed `8 of 8 passages indexed`; adding WAV/PNG correctly changed the count to `8 of 10 passages indexed`. Documentation validation reported `PASS: 3 documentation files, 10 local links, balanced code fences`.
- Final native resume completed `10 of 10 passages indexed`; Stop embedding model returned `Stopped`. Read-only SQLite verification found 6 documents, 10 chunks, 10 embeddings, 3 previews and 7 media parts. Process inspection confirmed the owned embedding executable was no longer running.

## Consumer resource evaluation: 2026-10-09
- Previous turn was progress: media ingestion, native preview/playback and semantic indexing completed with recorded checks.
- [x] Add a repeatable probe using production runtime/inference settings and macOS process accounting.
- [x] Measure Q4 generation, multimodal embedding and both loaded, including prefix reuse and process cleanup.
- [x] Record machine/workload/method limits and update the consumer hardware recommendation from evidence.
- Exploratory measurement code uses test-after: validate accounting/parsing and the actual end-to-end probe before publishing results. No application behavior is changed by the probe.

- Added `resource_probe` example using production managed TLS, authenticated inference, model settings and media embeddings. Added macOS SDK C sampler plus Python collector with an isolated owned process group. Application behavior is unchanged.
- `clang -Wall -Wextra -Werror -O2 evaluation/resources/memory_sample.c -o .local/memory_sample` exited 0. Direct accounting check reported `PASS: SDK sampler observes a 64 MiB touched allocation and lifetime high-water mark`; invalid PID returned exit 2.
- Independent review reproduced a collector cleanup defect when process discovery failed. Cleanup now signals the group established at creation and always reaps the direct child. Final review reported no additional material finding. `python3 -m unittest discover -s evaluation/resources -p 'test_*.py' -v`: `Ran 5 tests`, `OK`, including discovery failure and timeout with a child.
- `cargo build --release --manifest-path src-tauri/Cargo.toml --example resource_probe`: `Finished release profile [optimized]`. Direct Homebrew cargo-clippy all-targets desktop with warnings denied exited 0. Rust formatting applied; full application tests were not repeated because this increment adds only evaluation tools and documentation.
- Two `python3 evaluation/resources/measure.py` executions produced `PASS: 221 memory samples` and `PASS: 127 memory samples`, each with `owned process group empty`. Together: 48 generation requests and 36 measured embedding requests. Artifact and fixture hashes match across runs.
- Sampled simultaneous combined physical footprint: 1.507 and 1.488 GiB. This macOS accounting excludes clean mapped pages and the desktop webview/OS; it cannot establish whole-device RAM requirements. Generation-only 1,365-token median latency across six requests per condition: 1,624.5 ms reuse disabled, 763.0 ms enabled, 1,364 prompt tokens reused. LMCache was absent.
- A 28,919 ms combined-startup outlier justified a second run; the repeat was 2,164 ms. Cause unresolved; both observations remain published. Two incomplete repeat-run shutdown samples (three ESRCH process readings) were excluded from summaries; active-workload samples had no accounting errors.
- `evaluation/resources/README.md` documents reproduction, raw evidence, limitations and the continued 16 GB Windows design target. 8 GB support remains unqualified. Updated the existing HTML plan's hardware section, README and model assessment. Browser inspection confirmed the new HTML text and its evidence link rendered; HTTP returned `200 OK`.
- Deterministic report validation returned `PASS: 2 consistent reports, 48 generations, 36 embeddings, 348 samples; published medians verified`. Documentation validation returned `PASS: 5 Markdown files, 20 local links, HTML local links and code fences`. Documentation is exempt from test-first.
- Still unfinished: automatic model/engine setup, whole-app resource management and profiling, consumer Windows execution/signing, maximum-context and representative retrieval/answer qualification, PDF/page citations, media-to-text answers, forced-app-exit process cleanup and final full-scope audit. The goal remains active.

## Verified model installation: 2026-10-09
- Previous turn was progress: resource probe, cleanup regressions and measured cache reuse were published.
- [x] Add a static pinned Q4 generation and Q8 multimodal model catalog, bounded HTTPS downloads, checksum/header/size verification and atomic publication.
- [x] Add progress, cancellation and explicit external-download controls; preserve manual file selection and engine setup.
- [x] Apply verified paths only to a stopped, unchanged runtime; preserve active and user-selected configurations.
- [x] Verify fixture failure/cancellation/concurrency cases, actual pinned download/reuse and packaged native setup; independently review boundaries in two rounds.
- Interrupted partial files restart on retry; completed verified files are reused. Automatic engine installation remains separate pending work.

- 2026-10-10 continuation: previous discussion answered the local-small-model/LMCache question without implementation changes. Revalidated the pending installer and actual download handle; the download completed successfully.
- Final automated checks: `npm run test:rust` reported `64 passed; 0 failed`; `npm test -- --run` reported `Tests 25 passed (25)` across 6 files. TypeScript/Vite built 1,749 modules. Direct Homebrew Clippy with warnings denied exited 0.
- Independent review round 2 confirmed normal-exit cleanup coordination and late-start prevention. The child-process regression first failed before the fix and now proves partial cleanup before `process::exit` plus rejection of later installs.
- Actual embedding download verified 2 files totaling 864,676,480 bytes in 200,054 ms; reuse verified the same files in 2,078 ms. Reports and reproduction: `evaluation/installation/README.md`.
- Fresh macOS debug package built at 39.33 MiB. Native Quit during a 15,053,780-byte Q4 partial download removed the partial and exited process 12045. Relaunch retained 6 documents and 10 passages; retry and progress restoration after navigation were observed.
- Native Q4 retry completed and selected the 1,107,409,472-byte verified file. The selected installed engine reached Running/Connected and answered the synthetic budget question with its correct citation in a displayed 1.5 seconds (363 input, 0 cached tokens). Separate reuse verified the Q4 file in 2,668 ms.
- Native embedding installation reverified its existing pair, selected both app-data paths and preserved the chosen engine. Read-only settings/database/process assertions returned `PASS: 2 persisted model configurations; 6 documents, 10 chunks, 10 embeddings; 0 partial files; 0 app child processes` after stopping generation. Embedding startup with the identical weights was verified in previous integration work and was not repeated here.
- Installation increment complete. Remaining full-scope work includes engine distribution, forced-exit cleanup/orphan recovery, PDF/page citations, media-to-text answers, whole-app consumer resource bounds, representative quality qualification, Windows execution/signing and the final requirement audit. The goal remains active.

## Bundled inference engine: 2026-10-10
- Previous turn was progress: verified pinned model installation and packaged native lifecycle.
- [x] Prepare checksum-pinned official llama.cpp archives for macOS and Windows with bounded extraction and manifest verification.
- [x] Bundle dependencies/license with the desktop artifact; select included engine for new configurations and preserve manual choices.
- [x] Add explicit included-engine selection for existing workspaces and resolve its path across app moves/upgrades.
- [x] Verify actual generation and embedding compatibility, archive failure paths, desktop setup, and independent security review.
- Official b11531 macOS arm64 archive digest matches GitHub metadata. Production managed-generation smoke returned 5 PASS lines and semantic smoke returned 5 PASS lines. Windows runtime still requires a Windows runner; source checks and archive inspection alone do not qualify it.

- Runtime config adds a backward-compatible bundled-engine flag. New configurations resolve the packaged resource; manual choices remain intact. Explicit included selection shares the existing guarded configuration path and persists models/projectors. A restart resolves included selections against the current app location.
- Test-first default/relocation regression initially failed (`left: None`, expected included executable). Final Rust tests reported `66 passed; 0 failed`; frontend tests reported `Tests 30 passed (30)`. Direct Homebrew Clippy with warnings denied and Rust formatting passed.
- Preparation tests: `python3 -m unittest discover -s scripts -p 'test_*.py' -v` reported `Ran 14 tests`, `OK`. Archive checksum, extracted-payload pin, bounded extraction, Windows path safety, retained notices, symlink expansion and reuse mutation checks are covered.
- Parent reproduced independent review's forged binary-plus-manifest reuse finding. The preparer now pins the canonical extracted-file fingerprint in code and verifies it before fresh publication and reuse. Final review round 2 reproduced rejection and found no remaining material issue.
- Production b11531 managed generation and semantic examples returned 5 PASS lines each; multimodal integration returned 11 PASS lines. Cross-host fresh Windows preparation verified 52 files and Mac preparation verified 61 files. Windows executable behavior remains unverified.
- Final `npm run desktop:build -- --debug --bundles app` reported `Finished 1 bundle`, 108.12 MiB. Packaged engine contents matched all 61 pinned entries, totaling 72,108,556 bytes.
- Native included-engine controls preserved prior manual selections before explicit clicks, retained models, and started both generation and embeddings. Existing index showed `10 of 10 passages indexed`; blue-square search ranked the video at 10 seconds first. Stop controls left 0 app child processes; 6 documents, 10 chunks and 10 vectors remain. Evidence: `evaluation/engines/README.md` and JSON reports.
- Remaining full goal: PDF/page citations, media-to-text answers, decoder distribution, forced-exit recovery, whole-app consumer resource bounds, representative retrieval/answer quality, Windows runtime/signing and full requirement audit. No deployment or publication occurred.
- Final documentation validation: `PASS: 4 Markdown files, 16 local links, balanced fences; 4 JSON reports parse; final packaged payload matches trusted pin`. Documentation is exempt from test-first. Removed two unused extraction locals; Python compilation passed.

## Forced application-exit cleanup: 2026-10-10
- Previous goal turn was progress: included inference-engine distribution and native setup passed.
- [x] Add a private-pipe guardian that owns and reaps engine/decoder children after app termination.
- [x] Route generation, embedding and media subprocesses through the guardian while preserving bounded output, exit status and normal stop semantics.
- [x] Verify abrupt parent exit, normal child exit, output forwarding, cancellation and real packaged engine/decoder behavior; independently review process boundaries.
- Scope: forced termination of the application. Independent termination of the guardian and an OS crash require separate guarantees and must remain explicitly qualified.

- 2026-10-10 continuation: the preceding question clarified the small-model direction without changing implementation. Revalidated the pending guardian work against the current tree. The prior packaged app left one owned llama-server alive for 5,078 ms after SIGKILL; the probe cleaned it up and a separate process check confirmed both recorded PIDs gone. Baseline: `evaluation/lifecycle/baseline-forced-exit.json`.
- Initial integration checks: `npm run test:rust` returned `73 passed; 0 failed`; lifecycle probe tests returned `Ran 3 tests`, `OK`. Startup error-path inspection identified possible child leaks and a blocked output-relay join. Those corrections and independent review are in progress; this increment is not yet verified complete.

- Final lifecycle verification: `npm run test:rust` reported `77 passed; 0 failed` (including 11 guardian tests); probe tests reported `Ran 3 tests`, `OK`. All-target desktop Clippy with warnings denied and Rust formatting passed. The final guardian uses a pre-spawn commit gate, error-path ownership guard, and lease-aware output completion.
- Independent review completed two rounds. Final review verified guarded failure paths plus media output-cap and aborted-task cleanup. Fault tests now wait for child readiness and are restricted to the fixture executable. Initial test failures and the edition-2021 syntax issue were corrected before final validation.
- Actual Q4 generation integration passed 5 checks; actual FFmpeg/multimodal integration passed 11 checks. `npm run desktop:build -- --debug --bundles app` produced 1 bundle (108.64 MiB), with TypeScript/Vite building 1,749 modules.
- Native packaged app started generation and embeddings, showing Running and 10/10 indexed passages. After SIGKILL of root PID 32920, its 2 guardians and 2 engines were all absent within the probe's 66 ms observation window; 0 survivors. Relaunch preserved 6 documents, 10 chunks and 10 embeddings. Evidence: `evaluation/lifecycle/README.md`.
- This lifecycle increment is complete for the tested macOS/direct-child scope. Remaining full goal: PDF/page citations, media-to-text answers, decoder distribution, leftover temporary/download-file recovery, whole-app consumer resource bounds, representative quality qualification, Windows execution/signing and final full-scope audit. Frontend tests were not repeated because UI code was unchanged; Windows execution remains unavailable locally. No deployment or publication occurred.

## PDF import and page citations: 2026-10-10
- Previous goal turn was progress: guarded local runtimes and decoder cleanup verified in the packaged app.
- [x] Extract selectable PDF text locally in a guarded helper with bounded source, stream expansion, page count, text output and execution time.
- [x] Preserve physical page numbers across blank pages, chunk/search/index/store/restart and citation source navigation; preserve existing library data.
- [x] Wire native PDF selection and grounded answers, explain OCR/layout limitations, and verify the packaged import/search/citation flow.
- [x] Test failure and migration paths, run relevant checks and independent review; record actual evidence and remaining full scope.
- Initial scope supports embedded selectable text. Scanned-page OCR and original-layout rendering remain separate capabilities. Original PDF bytes and extracted page text will be stored locally.

- PDF tests were introduced before extraction existed. A protected-PDF regression then failed because lopdf auto-decrypted empty-password files; checking retained encryption state fixed it. The final extraction suite has 8 tests.
- Review identified unbounded ToUnicode reverse-map expansion. Parent reproduced with 65,537 mappings; vendored lopdf 0.45.0 now checks mapping/payload expansion before allocation and propagates the error through font fallback. Only two upstream source files differ; MIT license retained. Round 2 passed review with per-CMap limits explicitly distinguished from a process-wide RSS quota.
- Final automated checks: `npm run test:rust` returned `90 passed; 0 failed`; frontend tests returned `Tests 33 passed (33)`; Clippy with warnings denied and Rust formatting passed. Real PDF integration returned 5 PASS lines, including actual semantic retrieval and a Q4 answer citing page 3. Final debug package: 1 bundle, 115.06 MiB, 1,750 Vite modules.

- Continuation: the preceding question answered the small-model/LMCache architecture without implementation progress. Revalidated the live packaged app, completed indexing (12/12), stopped both models and restarted it.
- Native persistence assertions returned `PASS: 7 documents, 12 chunks, 12 vectors; 3 PDF pages, 2 PDF vectors, identical 2599-byte source; 0 app descendants after restart`. The PDF reopened at page 1 of 3. Prior native answer/citation verification is recorded in `evaluation/pdf/native-results.json`; the answer cited and opened page 3.
- Selectable-text PDF increment complete for the tested macOS scope. Remaining full goal: media-to-text answers, decoder distribution, orphan temporary/download-file recovery, whole-app consumer resource bounds, representative quality qualification, Windows runtime/signing and final full-scope audit. No deployment or publication occurred.
- Final documentation-only verification (test-first exempt): deterministic validation returned `PASS: 5 Markdown files, 24 local links, balanced fences, HTML links, JSON reports, fixture hash, and 90/33/5 recorded checks`; direct Homebrew `cargo-fmt fmt --manifest-path src-tauri/Cargo.toml -- --check` exited 0. No executable code changed during this continuation.

## Interrupted model download recovery: 2026-10-10
- Previous goal turn was progress: PDF indexing and persistence were verified and documented.
- [x] Reclaim owned abandoned download partials on startup and installation retry; preserve verified models and unrelated paths.
- [x] Hold a per-revision OS file lock throughout installation/recovery; skip active instances during startup and reject conflicting installs.
- [x] Verify abrupt process termination, active-owner preservation, non-regular files, retry and packaged startup; independently review concurrency/security boundaries.
- Scope: model partial files in pinned catalog directories. Decoder/PDF scratch and runtime TLS orphan files remain separate unfinished recovery work.

- Regression first failed because retry retained an abandoned partial. Review then found a pathname-swap race; parent reproduced deletion of an outside disposable fixture before replacing operations with anchored no-follow cap-std/cap-fs-ext handles. Final review found no remaining material issue within the trusted app-data-parent boundary.
- Recovery recognizes legacy six-character and current UUID partial names, holds a stable per-revision OS lease, skips active revisions at startup, rejects conflicting installs and preserves unrelated paths. Download creation/drop, verification and hard-link no-clobber publication use the same directory handle. A final directory identity check rejects moved/replaced selections. Hard-link support is required for publication; older builds must be closed before upgrading because they do not participate in leases.
- `npm run test:rust`: `97 passed; 0 failed`; focused model tests: `23 passed; 0 failed`. Direct Homebrew all-target desktop Clippy with `-D warnings` exited 0; formatting passed. New standard-library file locks require declared Rust 1.89+. UI behavior was unchanged, so frontend tests were skipped; TypeScript/Vite passed during package build.
- `npm run desktop:build -- --debug --bundles app`: `Finished 1 bundle`, 115.00 MiB. Production model reuse returned `PASS: 1 verified model file(s), 1107409472 bytes, 35833 ms`. This duration occurred during concurrent builds and is not a performance qualification.
- Native startup assertions returned `PASS: startup reclaimed 1 partial / 4096 bytes; preserved 3 model hashes and 7 documents / 12 passages / 12 vectors; 0 app descendants`. Report: `evaluation/recovery/README.md`.
- Remaining full scope includes decoder/PDF/runtime scratch orphan recovery, immediate inference cancellation, media-to-text answers, decoder distribution, whole-app resource qualification, representative quality evaluation, Windows runtime/signing and final product audit. No deployment or publication occurred.

## In-flight answer cancellation: 2026-10-10
- Previous goal turn was progress: interrupted model-download recovery verified in tests and packaged startup.
- [x] Add request-scoped cancellation covering answer retrieval and generation; release operation guards and preserve other/new requests.
- [x] Wire visible cancellation, navigation/unmount handling, preserved evidence and retry in the desktop UI.
- [x] Verify pending/running/late cancellation, HTTP disconnect, real bundled engine behavior and native UI; independently review concurrency.
- This increment covers answers and their retrieval. Standalone search/index cancellation and other scratch-file recovery remain outstanding.

- Initial cancellation registry regressions failed 3/3 before implementation. Eight final backend cases cover pending/active/stale requests, abort/shutdown cleanup, result races and actual HTTP disconnection while waiting for headers or response body.
- UI review identified lost results with delayed cancellation acknowledgement and execution after a failed cancel during registration. Four ordering regressions failed before fixes. Final independent review round 2 found no remaining material issue; the exit-wait concern was retracted because process exit closes sockets and the answer has no durable temporary artifact.
- `npm run test:rust` returned `105 passed; 0 failed`; `npm test` returned `Tests 44 passed (44)` across 6 files. All-target desktop Clippy with warnings denied, Rust formatting and TypeScript/Vite passed. One initial full-suite decoder-abortion PID-marker failure is retained in `evaluation/cancellation/rust-initial-failure.txt`; the isolated test and full rerun passed without changing decoder code. Its cause remains unproven.
- Actual managed Q4 engine smoke returned 5 PASS lines: processing observed before cancellation, request/permit release measured 0 integer ms, engine idle observed after 1,308 ms, cited budget retry succeeded and engine stopped. These are workload observations; compute cancellation timing is engine-dependent.
- Packaged debug build produced 1 bundle (115.09 MiB). Native search-view and Documents-view cancellation both showed the completion notice; 6 evidence hits remained. Retry displayed a cited answer in 4.7 seconds with 609 input and 608 cached tokens. Final assertions: `PASS: 2 native cancellations, 6 retained hits, cited retry; 7 documents / 12 passages / 12 vectors; 0 app descendants`.
- Answer cancellation increment complete for tested macOS scope. Remaining: standalone search/index cancellation, decoder/PDF/runtime scratch recovery, media-to-text answers, decoder distribution, whole-app resource qualification, representative quality evaluation, Windows execution/signing and final full product audit. No deployment or publication occurred.

## Search and indexing cancellation: 2026-10-10
- Prior implementation turn was progress: generic backend registry, batch persistence regression and frontend cancellation were added. The intervening architecture answer changed no implementation state; this continuation revalidated source and finished the incomplete tests.
- [x] Generalize the existing cancellation registry and route search, answers and index batches through the same request identity and operation guard.
- [x] Cancel active indexing immediately, keep committed vectors and resume pending passages; cancel semantic search while preserving previous results.
- [x] Reuse frontend cancellation ordering rules, verify failure/navigation/unmount races, run actual embedding and packaged checks, and independently review concurrency.

- Shared generic operation registration now covers answer, search and each index batch. Per-vector SQLite commits survive cancellation; completed passages are skipped on resume. Frontend index pause intent survives false/rejected cancellation acknowledgements and stops later batches. Search preserves previous query/results after accepted cancellation.
- Independent review identified stale-ID permit acquisition before validation. Parent reproduced a stale command returning the busy error; identity validation and non-blocking permit acquisition now share one registry critical section. Round 2 found no remaining material concurrency issue.
- Final automated checks: `npm run test:rust` returned `107 passed; 0 failed`; `npm test` returned `Tests 51 passed (51)` across six files; all-target desktop Clippy with warnings denied and formatting passed. Four additional index ordering tests cover pending registration, false/rejected acknowledgement and unmount.
- Actual embedding engine probe produced six PASS lines: active index cancellation retained one of ten vectors, two batches resumed text indexing, four image/audio/video parts indexed, active query cancellation released the permit, semantic retry returned budget evidence and runtime stopped. Client cancellation measured below one integer millisecond; this does not measure compute stop latency. The initial probe failed because identical document bodies deduplicated; unique synthetic bodies corrected the fixture.
- Native packaged index cancellation stopped at 29/51 and resumed 51/51. All 29 retained vector hashes were unchanged. The synthetic 39-passage fixture remains, giving eight demo documents. Semantic retry returned eight hits with PDF page 3 first. Native search completed before cancellation landed; accepted native search cancellation remains unobserved, with production engine cancellation and UI ordering tested separately.
- Final native check: `python3 evaluation/search-index-cancellation/verify_native.py` returned `PASS: native index cancellation retained 29/51, resumed 51/51; 29 hashes unchanged; 8 documents; 0 app descendants`. Details: `evaluation/search-index-cancellation/README.md`.
- Remaining full goal: decoder/PDF/runtime scratch recovery, media-to-text answers, decoder distribution, whole-app consumer resource and representative quality qualification, Windows runtime/signing and final acceptance audit. No deployment or publication occurred.
- Final package refresh after removing an unused frontend cancellation flag: `Finished 1 bundle`, 115.12 MiB; frontend tests remain 51/51. Relaunch preserved eight documents and 51 indexed passages with zero app descendants. Documentation-only updates are test-first exempt; validation returned `PASS: 4 Markdown files, 29 local links, balanced fences, 2 JSON reports; 107 Rust tests, 51 frontend tests, 6 engine checks, 1 package recorded`.

## Owned scratch recovery: 2026-10-10
- Previous goal turn was progress: generic cancellation, committed-vector resume, real embedding checks and native index cancellation were completed.
- [x] Replace anonymous production media/PDF/TLS workspaces with private application-owned session directories.
- [ ] Recover abandoned sessions using anchored directory handles and OS leases; preserve live parents and subprocess guardians, unrelated files and symlink targets.
- [ ] Verify crash/active-owner/path-swap behavior, normal media/PDF/runtime paths and packaged startup; independently review concurrency and deletion boundaries.
- Legacy anonymous temporary directories have no reliable ownership marker and will be preserved. The new recovery contract applies to the owned namespace created by this build.

- The new namespace uses shared parent/guardian session leases and an exclusive recovery check under a stable coordinator with a five-second wait limit. Unix deletion uses retained open directories. Ten focused scratch regressions cover orphan/live-owner cleanup, symlink preservation, real-directory replacement, partial creation and bounded lock contention.
- Parent runtime regression first failed on anonymous TLS files. Worker reproduced deletion of a real-directory replacement marker, then switched Drop and recovery to retained handles. Independent review found one remaining name-based allocation-failure cleanup; that branch now consumes the retained directory too. Its added test directly exercises the cleanup helper; its initial red was a missing-helper compile failure and does not reproduce a forced allocation-lock failure.
- Concrete unfinished Windows issue: cap-primitives 3.4.6 implements remove_open_dir_all by closing the handle and calling std::fs::remove_dir_all(path). Unix rename-safety verification does not establish that guarantee on Windows. Replace that fallback with a Windows-safe deletion strategy and verify it on Windows before declaring this recovery increment cross-platform complete.
- Final macOS checks: `npm run test:rust` returned `117 passed; 0 failed`; ten scratch regressions passed, all-target desktop Clippy with warnings denied and formatting passed. Actual media decoding ran with installed FFmpeg. Frontend unit tests were skipped because UI behavior was unchanged; TypeScript/Vite passed in the package build.
- Production guardian crash probe returned four PASS lines; actual PDF/embedding/Q4 integration returned five PASS lines, including grounded page-3 answer and both engines stopped. New macOS debug package: one bundle, 115.23 MiB.
- Native startup after preparing three abandoned workspaces returned `PASS: native startup reclaimed 3 workspaces / 3 files; unrelated file preserved; 8 documents / 51 passages / 51 vectors unchanged; 0 app descendants`. Report: `evaluation/scratch-recovery/README.md`.
- Unix recovery and native startup are verified. Cross-platform recovery remains incomplete because of the concrete Windows deletion fallback above. Remaining full goal also includes settings-publication orphan handling, media-to-text answers, decoder distribution, whole-app resource/quality qualification, Windows runtime/signing and final product acceptance. No deployment or publication occurred.

## Windows test portability: 2026-10-10

- [x] Replace the unconditionally imported Unix shell fixture in the embedding runtime argument test with the existing compiled helper, preserving projector/context/pooling and generation checks.
- [x] Enable the platform-independent bundled configuration test on Windows.
- [x] Verify the changed helper against all Rust library tests: 117 passed, 0 failed in 1.58 seconds on macOS. The focused test first failed because the original helper did not produce the requested JSON capture.
- [ ] Execute the Windows build and tests; this host has only an Apple ARM Rust target. Source portability alone does not qualify Windows execution.
- Windows scratch deletion still requires implementation and platform verification. This increment changes test infrastructure only. Evidence: `evaluation/windows-portability/README.md`.
- All-target desktop Clippy passed with warnings denied (12.42 seconds); Rust formatting passed. Frontend tests and package rebuild were skipped because changes are confined to test infrastructure.
- Read-only dependency audit confirms Windows directory handles omit `FILE_SHARE_DELETE`; three portable rename/replacement tests therefore need equivalent Windows-specific tests together with the future deletion backend. Do not simply disable coverage and claim Windows recovery complete. A Windows backend must also preserve independent parent/guardian session access while maintaining handle-based deletion.

## Windows scratch deletion backend: 2026-10-10

- [x] Implement handle-relative Windows directory operations and delete-by-handle cleanup, preserving parent/guardian session leases and reparse targets.
- [x] Compile the exact Windows backend and its tests using an isolated Windows-target check harness; keep actual Windows runtime verification separate.
- [x] Run Unix integration regressions and independent review of unsafe API boundaries, replacement behavior, sharing modes and cleanup failure paths.
- [ ] Verify the backend on Windows before marking cross-platform recovery complete.
- Implemented native Windows relative opens and delete-by-handle cleanup in `scratch_windows.rs`; platform-neutral binary parsing lives in `scratch_windows_records.rs`. The same traversal budget spans query pages and recursive children (16,384 records, 4 MiB filename bytes, 64 directory levels).
- Windows production scratch source and tests pass cross-target check and Clippy with warnings denied via `scripts/scratch-check`. This does not link or run the Windows desktop. Isolated toolchain: `.local/rustup`, Rust 1.91.1, x86_64-pc-windows-msvc standard library; global toolchain selection unchanged.
- Final macOS checks: `cargo test --manifest-path src-tauri/Cargo.toml --lib --features desktop` reports 123 passed, 0 failed in 1.57 seconds; all-target desktop Clippy passed (1.93 seconds), and formatting passed. Six Windows record/budget regressions execute portably. A continuation-record test first failed before the boundary correction.
- Two review rounds complete. The first found a per-query-batch cap that allowed unbounded total enumeration; final code shares one checked budget throughout traversal, and cumulative-page tests pass. No remaining material finding in the second review.
- Windows reparse regression is explicitly invoked by the local CI definition, including its symbolic-link privilege requirement. Workflow YAML parses; no remote workflow was run. Native Windows cleanup, packaging/signing and process behavior remain unverified; frontend, model inference and macOS package checks were skipped because their behavior was unchanged.
- Report: `evaluation/windows-scratch/README.md`. Full goal remains active: Windows qualification, settings-publication recovery, decoder distribution, media answers, consumer resource/quality qualification and final acceptance remain unfinished.

## Recoverable settings publication: 2026-10-10

- [x] Stage runtime, embedding, endpoint and media JSON in same-filesystem leased workspaces; preserve committed data through preparation and failed publication.
- [x] Recover abandoned settings staging at desktop startup and save retry, preserving active writers and unrelated legacy temporary files.
- [x] Verify forced writer termination, publication/error paths, desktop state ordering and Windows cross-compilation; independently review shared lease/publication boundaries.
- New publication lifecycle is exploratory implementation with test-after verification; existing process/recovery tests provide regression coverage. Reproducible state-ordering defects receive failing regression tests before their fixes.
- Added `settings::save_json` and same-filesystem publication workspaces using existing scratch session leases. Startup and save retry reclaim abandoned staging; all four JSON settings files keep their existing schemas and paths. Unattributed legacy `.tmp` files are preserved.
- Forced-process test verifies active staging survives recovery, killed-writer staging is reclaimed and committed JSON remains unchanged. Additional tests cover overlapping writers, encoding failure, failed publication and normal cleanup.
- Reproduced endpoint bug before fixing: failed persistence cleared managed mode. The regression failed at the managed-mode assertion; holding the runtime guard and committing before changing mode now preserves state on failure and updates it after success.
- Final full Rust suite: 130 passed, 0 failed in 1.33 seconds. All-target desktop Clippy passed in 2.83 seconds; formatting passed. Windows cross-target source/tests check and Clippy passed for the exact scratch/settings modules. Independent review found no material issue.
- Verification report: `evaluation/settings-recovery/README.md`. Native Windows execution, native startup/UI checks for this increment and repackaging remain unverified; frontend/model checks were skipped because their behavior was unchanged. The full goal remains active with decoder distribution, media answers, consumer qualification and final acceptance still unfinished.


## Included media decoder: 2026-10-10

- [x] Resolve included decoder from current application resources; preserve legacy manual settings and publish selection before changing memory.
- [x] Verify pinned FFmpeg source signature and build static x264 dependency on macOS ARM64.
- [x] Build and exercise FFmpeg/ffprobe through production media preparation: 11 fixture imports passed on macOS ARM64.
- [ ] Add verified source preparation, complete rebuild materials, decoder resources and package/CI integration.
- [ ] Verify packaged fresh-profile media import and Windows build/runtime behavior.
- [x] Complete independent decoder-selection review and relevant checks: no selection defect; 134 Rust tests, 53 frontend tests, TypeScript/Vite build, formatting and Clippy passed.
- Source build is exploratory work with test-after validation against the existing production decoder smoke example. No decoder package has been published.

- Decoder evidence: `evaluation/decoders/README.md`. Prototype build corrected PNG configure naming and x264 assembler deployment flags. Final executables total 18,021,376 bytes, use system-only dylibs and file/pipe protocols. Packaging and Windows execution remain unverified.


## Decoder source preparation and macOS packaging: 2026-10-10

- [x] Implement bounded, hash-pinned source preparation with regular-file extraction and fresh builds; bundle source archives, licenses and standalone rebuild scripts.
- [x] Add failure regressions for corrupt source, traversal/links/aliases, extraction caps, timeout process cleanup and publication rollback.
- [x] Run fresh preparation and replacement preparation using the verified local archives; verify 15 prepared files against the diagnostic receipt.
- [x] Wire `desktop:build:mac` and the macOS CI matrix to both resource overlays; preserve the Windows engine-only build pending its decoder recipe.
- [x] Inspect the packaged macOS app and exercise its included binaries through production import: 15 payload files matched, 11 fixture imports passed, one 144.99 MiB debug bundle built.
- [ ] Complete Windows source preparation, native runtime and installer verification.
- Source integrity/publication tests were written first; each new API initially failed before implementation. The initial build recipe had already passed exploratory production smoke testing.

- Final preparation verification: 26 Python tests passed; two source builds completed (fresh and replacement); independent review resolved the missing packaging command and found no further issue. See `evaluation/decoders/README.md`. Native fresh-profile UI import, Windows decoder preparation/runtime, media answers and consumer qualification remain unfinished.


## Windows decoder source build: 2026-10-10

- [x] Pin and verify zlib 1.3.2 source against the upstream published checksum for the Windows static dependency.
- [x] Add Windows x86-64 target flags, static dependency linking, PE architecture/import checks and source/license materials.
- [x] Wire MSYS2 UCRT64 source preparation, native decoder smoke and both Tauri overlays into Windows CI configuration.
- [x] Complete the local MinGW cross-build and inspect its Windows binaries: two x86-64 PE executables, 19 payload files verified, Windows system/UCRT imports only.
- [x] Finish independent review and macOS regression checks: two review rounds, 31 Python tests passed, Mac source build plus 11 production imports passed.
- [ ] Execute Windows CI/native imports and qualify the installer on Windows.
- First cross-build reached FFmpeg configuration after building zlib and x264; FFmpeg required explicit `--pkg-config=pkg-config` because it otherwise prepends the cross prefix. A failing regression captured that setting before correction.

- Windows decoder report: `evaluation/decoders/windows.md`. Native Windows/MSYS2 execution and installer qualification remain unverified. CI is wired to all 11 fixture imports and both resource overlays. Cross-compilation and import inspection do not qualify native execution.


## Fresh-profile native decoder verification: 2026-10-10

- [x] Build a separate `com.cachella.decodercheck` app/profile without changing the existing library.
- [x] Verify automatic included-decoder selection, native file-picker import and video preview playback without a manual FFmpeg setup.
- [x] Quit/relaunch and verify persistence: 1 document, 2 chunks, 1 video asset and 2 media parts retained.
- [x] Quit the isolated app and verify no remaining test executable.
- Evidence: `evaluation/decoders/native.md`. One synthetic video fixture was used; no model was started and no embeddings were generated. The test profile and bundle are retained.
- Executable source was unchanged, so unit/lint suites were not repeated. Frontend build, desktop compilation, native UI observations and read-only database/process checks passed.
- Native Windows/MSYS2 execution, installers/signing, media answers, consumer quality/resource qualification and final acceptance remain unfinished.

## Packaged media answers: 2026-10-10

- [x] Integrate bounded stored media evidence into managed generation and offer the matching Gemma model/projector bundle.
- [x] Verify backend retrieval, structured answers, citations, persistence and shutdown on the synthetic fixtures (138 Rust tests and seven integrated answer requests recorded in the media-answer report).
- [x] Build isolated macOS package; verify included engine/decoder, cached-model installation, navigation/cancellation, indexing, citations, preview playback and restart persistence.
- [x] Fix terminal installer selection after remount/cancel; 58 frontend tests passed and rebuilt native UI exposed the cancellation message.
- [ ] Resolve/evaluate incomplete answers to compound media questions across retrieval orders and runtime contexts.
- [ ] Qualify whole-app resources and quality on target consumer hardware; execute Windows installer/runtime checks and final acceptance.

Native observations: combined video question answered $8/month but omitted the red circle (4.0 s, 450 input tokens, zero cached tokens). Visual-only question answered red circle (2.6 s, 445 input tokens, 67 cached tokens). Two distinct requests do not establish cache savings. Citation playback exposed the correct video and 0–3 second interval. Restart preserved 1 document, 2 chunks, 2 embedding rows, 1 asset and 2 media parts; final test-bundle process count was zero. See `evaluation/media-answers/native-ui.md` and `native-ui-persistence.json`. Fresh network download, Windows execution and consumer hardware qualification remain unverified. Full goal stays active.

## Controlled compound-media reproduction: 2026-10-10

- [x] Record eight production-request outcomes using the same imported video: frame/audio order, prompt-cache flag and two repetitions per condition.
- [x] Review factual completeness and citations manually; distinguish request validation from answer quality.
- [ ] Change production behavior only after evidence identifies a justified correction; retain the baseline artifact.
- [x] Validate the evaluation harness with build/lint and actual-engine execution, including shutdown.

The new evaluation harness is exploratory, with test-after verification using the existing production decoder, store, media loader and generation request. No product prompt or runtime behavior has changed.

Controlled result: 8/8 baseline requests passed structural validation; manual review found complete answers in 4/4 frame-first cases and visual omissions in 4/4 audio-first cases, independent of cache flag. An added completeness instruction caused 4/4 audio-first format failures and was rejected. Original production prompt restored; focused inference tests report 9 passed, probe build/lint/format passed, both engines stopped, and zero probe/guardian executables remained. Evidence: `evaluation/media-answers/order.md`. Remaining work is broader paired-media diagnosis and a validated general correction; no product fix is claimed.

## Mixed-media presentation correction: 2026-10-10

- [x] Extend reproduction to a second visual/audio pair and inspect the engine input path.
- [x] Add validated modality labels and stable audio-last request presentation; preserve selected hits, source binding and timestamps.
- [x] Capture failing unit regressions before each change and run final Rust checks: 139 passed, all-target desktop Clippy and formatting passed.
- [x] Manually review 24 final real-model outcomes: 16 complete answers over two synthetic videos, 8 correct abstentions; engines stopped and zero probe/guardian executables remained.
- [x] Rebuild and verify the corrected request through the native UI.
- [ ] Expand quality coverage and qualify whole-app consumer resources, Windows execution and installers, then complete the full acceptance audit.

Evidence: `evaluation/media-answers/normalization.md`. The preceding compound-question limitation is mitigated on the two controlled fixtures; broad reliability remains unproven. The retained macOS native test package predates this correction. No release was published.

Native correction verified: rebuilt 145.09 MiB debug package returned red circle and $8/month in 3.8 seconds, with both citations despite audio-first retrieval. Repeated answer was identical (3.2 seconds, 455 cached tokens of 460 input tokens). Citation preview preserved the 0–3 second interval; normal quit left zero test-bundle executables and persisted 1 document, 2 chunks, 2 embeddings, 1 asset and 2 media parts. See `evaluation/media-answers/native-normalized.md`. This completes the bounded native correction check; broader quality, consumer qualification, Windows and final acceptance remain unfinished.

## Combined Gemma resource qualification: 2026-10-10

- [x] Reuse the existing release resource probe and process-group sampler; add an optional generation projector and image/audio/combined-media answer phases.
- [x] Build and lint the extended probe; validate collector aggregation/cleanup and CLI behavior.
- [x] Run the actual Gemma + EmbeddingGemma workload with both engines resident and inspect recorded answers, accounting errors and shutdown.
- [x] Document measurements with explicit exclusions for desktop WebKit, decoder, OS and untested 16 GB/Windows hardware.

This evaluation extension uses test-after verification against the real inference engines. Product executable behavior is unchanged. The existing text-only invocation remains supported; the optional projector adds 12 media requests to the existing 24 text-generation and 18 embedding requests.

Combined-resource result: 54 measured requests, 218 valid process samples, no accounting errors, all owned processes stopped. Both-engine media phase peaked at 2,414,460,928 resident bytes and 1,698,055,232 footprint bytes. All 36 generated answers met the repeated synthetic fact rubrics by manual review. Release build, example Clippy, formatting, Python compilation/help, C compilation and five collector tests passed. Evidence: `evaluation/resources/gemma-media.md`. The measurement excludes UI/decoder/OS and does not qualify 16 GB or Windows. Windows runner availability was requested asynchronously; no answer has been assumed.

## Acceptance audit and Gemma text coverage: 2026-10-10

- [x] Map the product contract to current code, reports and missing evidence; correct stale README media-answer status.
- [x] Extend the existing text evaluator with an optional projector to follow the managed Gemma request path, retaining the original text-only invocation.
- [x] Run all eight unchanged text fixtures with both cache settings: 16 application calls, 14 model generations, zero errors/abstention mismatches; all eight rubrics met by manual review in both settings.
- [x] Validate release build (4.15 s), example Clippy (1.21 s), formatting, report structure and documentation links. Product/frontend behavior unchanged; those tests were not repeated.
- [ ] Implement the shared memory-budget policy described in the HTML plan and verify whole-app behavior.
- [ ] Complete representative quality/hardware qualification and native Windows delivery, then repeat the acceptance audit.

Evidence: `docs/acceptance-audit.md` and `evaluation/gemma4-e2b-text-assessment.md`. The audit found an actual implementation gap: serialized model starts currently do not enforce a shared memory budget. That can be progressed locally while Windows runner availability remains unanswered. Full goal stays active.

## Shared-resource policy feasibility: 2026-10-10

- [x] Inspect startup, retrieval, indexing, cancellation and engine fit behavior; distinguish model residency limits from unverifiable byte estimates.
- [x] Extend the existing multimodal smoke with a single-resident sequence; verify five retrieval checks, seven answers, stopped listeners and index retention after restarting embeddings.
- [x] Validate final release build (3.30 s), example Clippy (1.14 s), formatting and zero remaining probe/guardian executables.
- [ ] Add a persisted one-model mode and coordinate desktop search/index/answer switching with cancellation-safe startup.
- [ ] Test switch failure/cancellation/shutdown, independently review concurrency changes and verify native UI behavior.

Evidence: `evaluation/media-answers/single-resident.md`. The probe precomputes retrievals and proves one backend switch cycle; it is not an implemented desktop memory policy or a byte-level RAM cap. Default preference was requested asynchronously; no reply has been assumed and current two-model behavior is unchanged. Windows availability also remains unanswered.

## Cancellation-safe startup prerequisite: 2026-10-10

- [x] Capture failing startup-cancellation regression; guard owned startup through readiness and cleanup before permit release.
- [x] Verify cancellation for both roles, connection ownership, poisoned-lock cleanup and shutdown during readiness; independent review found no concrete defect.
- [x] Verify real generation and embedding startup in the rebuilt isolated Mac app, retained 2/2 indexed passages, and zero owned executables after quit.
- [ ] Implement persisted one-model mode and desktop search/index/answer switching; verify target-hardware resources and Windows delivery.

Evidence: [startup cleanup](../evaluation/media-answers/startup-cleanup.md). Earlier full suite: 142 passed; added shutdown test: 1 passed (142 filtered), 0.64 s. Test Clippy passed (11.68 s), formatting passed, isolated app build passed (16.45 s Rust). Automatic residency policy remains unfinished; full goal stays active.

## Optional one-model residency: 2026-10-10

- [x] Reproduce both engines remaining resident; implement persisted Both/Single policy and a shared admitted startup path.
- [x] Wire semantic retrieval, indexing and answers to resume explicitly started models within the existing operation permit; explicit Stop disarms automatic resume.
- [x] Verify persistence, failure/cancellation and external-connection boundaries; independent concurrency review.
- [x] Finish policy and suspended-runtime controls with frontend regression coverage.
- [x] Build and exercise a native search/answer switch cycle, cancellation, stop and restart persistence.

Existing two-model behavior remains the default. One-model mode applies to managed
children. Start each role once per session to enable automatic resumption; Stop
disables that role until its next explicit Start. Switching retains stored
embeddings, releases the stopped engine's prompt cache and incurs startup/hash
latency. This is a residency limit with no fixed byte-level memory guarantee.

One-model verification complete within the isolated Mac scope: [report](../evaluation/residency/README.md). Full Rust suite 148 passed; final focused desktop tests 20 passed after the model-specific error correction; frontend 67 passed; final Clippy, formatting and packaged build passed. Native successful switch/answer, cancellation during file verification, explicit Stop and policy/session restart behavior verified. 590 process samples showed at most one managed engine; final zero owned bundle processes. Whole-app consumer hardware, Windows execution and installer qualification remain unfinished. Full goal stays active.

## Release desktop resource accounting: 2026-10-10

- [x] Build the isolated release app and identify its own WebKit content/network/GPU coalition helpers.
- [x] Add read-only attach collector reusing process-rusage sampling and simultaneous-sum aggregation; test membership, errors and reparenting, independently review attribution limits.
- [x] Measure release UI, model starts, semantic search, answer and switching; inspect sampled members, failures and shutdown.
- [x] Report numerical scope and remaining consumer/Windows qualification without inferring minimum RAM from this Mac.

The prior engine-only resource reports exclude UI. This run includes attributed
app/XPC processes and descendant engines. Early app launch, system-wide driver/OS
allocations and other users remain excluded. It uses an evaluation-only private
macOS coalition ABI; unsupported layouts fail closed. The collector never stops
or modifies a measured process.

Evidence: [release desktop accounting](../evaluation/resources/release-desktop.md). Three runs: 899 + 511 + 755 samples, all final coalitions empty; eleven collector tests passed (0.758 s), Python/C checks and release build passed. Matched Single/Both answer footprint peaks: 1.165/2.071 GiB. Two short imports missed FFmpeg despite successful UI imports; a longer 60-second loop captured six valid FFmpeg samples. No production behavior changed. The isolated profile retains the new synthetic document (20 unindexed passages); original profile untouched. Early-launch/system-wide accounting, maximum workloads, target consumer hardware, Windows delivery and representative enterprise quality remain unfinished. Full goal stays active.

## Cancel embedding verification work: 2026-10-10

- [x] Reproduce cancellation leaving the already-running blocking hash unsignalled.
- [x] Propagate future cancellation into bounded hash reads without changing embedding profile identifiers.
- [x] Run focused and full Rust checks, independently review the cancellation boundary, and record remaining limits.

Evidence: [hash cancellation](../evaluation/residency/hash-cancellation/README.md). Regression failed before the guard; 152 Rust tests passed (3.54 s), Clippy passed (11.05 s), formatting passed. Independent concurrency review found no correctness issue. Worker signalling and hash checkpoints have separate deterministic tests; an end-to-end real-file cancellation observation, a rebuilt native package, Windows and consumer-device qualification remain open. Full goal stays active.

## Packaged cancellation recovery follow-up: 2026-10-10

- [x] Rebuild the isolated release app with the hash cancellation change.
- [x] Exercise native cancellation, retry, semantic search and normal shutdown.

Evidence: [packaged follow-up](../evaluation/residency/hash-cancellation/README.md#packaged-mac-follow-up). Release build 13.73 s; 114.85 MiB bundle. Native cancellation retained 2/22 passages, retry completed 22/22, semantic search returned eight matches, quit left zero test-bundle executables. The release had completed embedding startup before cancellation, so the hash-phase observation remains open. The isolated profile now has all 22 embeddings. Windows, consumer-hardware and final enterprise acceptance remain unfinished; full goal stays active.

## Maximum-duration media qualification: 2026-10-10

- [x] Generate and probe exact 120-second and over-limit video fixtures.
- [x] Exercise production decoding at the duration boundary and verify rejection above it.
- [x] Observe the accepted import in the release desktop with existing coalition accounting; report scope and remaining limits.

Evidence: [duration boundary](../evaluation/resources/duration-boundary.md). Production decoding and native UI accepted 120 seconds (36 passages), rejected 121 seconds. 1,294 process samples, twelve valid FFmpeg observations, final coalition empty. Isolated profile: 3 documents, 58 chunks, 22 embeddings (36 new passages unindexed). Maximum file size/resolution, model pressure, consumer hardware and Windows acceptance remain open.

## Full-duration indexing and answer: 2026-10-10

- [x] Index all 36 passages from the accepted 120-second fixture in the release app.
- [x] Verify retrieval, cited generation and source navigation with one-model switching.
- [x] Record process measurements and final persistent state without inferring target-hardware acceptance.

Evidence: [full-duration index and answer](../evaluation/resources/duration-index-answer.md). 36 new embeddings, 58/58 total; eight retrieved passages; correct $8/month answer with four citations; full-duration citation opened at 1:00–1:10. 1,019 process samples, maximum one engine, final coalition empty. Limits on maximum bytes/resolution/context, enterprise quality, lower-memory hardware and Windows remain open.

## Media byte and geometry boundaries: 2026-10-10

- [x] Exercise byte/pixel/side limits through the actual decoder and retain failing evidence.
- [x] Fix zero-sized scaling and yuv420p odd-dimension failures across image and video preparation; bump normalization fingerprint.
- [x] Run real-decoder regression, full Rust suite, Clippy, formatting and rebuilt release probe.

Evidence: [boundary report](../evaluation/resources/boundaries/README.md). Ten cases: seven accepted, three rejected; five previously failing geometry cases now pass. 153 Rust tests passed (3.04 s), Clippy passed (3.24 s), release probe built (3.31 s). v2 normalization preserves old entries; reimport can create a new entry. Packaged/native verification of this change, complex maximum-size workloads, context pressure, target hardware and Windows delivery remain open.

## Packaged geometry correction: 2026-10-10

- [x] Rebuild the isolated release package and import previously failing image/video geometry through native controls.
- [x] Verify stored dimensions, v2 fingerprints, retained index and shutdown.

Evidence: [native package follow-up](../evaluation/resources/boundaries/README.md#packaged-mac-follow-up). Build 27.38 s, 114.85 MiB; native imports 2; stored dimensions 512 × 1 and 512 × 256; both v2 identities match; zero test-bundle processes after quit. Isolated profile now has 5 documents, 60 chunks, 58 embeddings. The two new passages are unindexed; consumer hardware, Windows and enterprise acceptance remain open.

## Corrected geometry embedding/retrieval: 2026-10-10

- [x] Embed both corrected geometry imports through the release app.
- [x] Verify semantic retrieval and source opening; preserve all previous vectors.

Evidence: [native embedding follow-up](../evaluation/resources/boundaries/README.md#native-embedding-and-retrieval-follow-up). 60/60 passages indexed; two new finite normalized 768-D vectors; previous 58 profiles/vector hashes unchanged. Each corrected source ranked first for its respective synthetic query; the image source opened; zero test-bundle processes after quit. No executable changes. Context pressure, representative enterprise evaluation, consumer hardware and Windows acceptance remain open.

## Context-limit errors and recovery: 2026-10-10

- [x] Exercise ordinary/dense/recovery evidence through the real managed model.
- [x] Reproduce generic HTTP400 feedback; map the exact structured context error through bounded parsing without exposing engine details.
- [x] Verify malformed/oversized/truncated/stalled and other-status cases; independently review; repeat live recovery after rebuild.

Evidence: [context-limit report](../evaluation/context/README.md). Eight final calls: four intentional context-limit rejections, four correct ordinary/recovery answers. 155 full Rust tests passed; final twelve inference tests passed; Clippy/formatting passed. No probe/engine processes remained. The package has not been rebuilt for this change. Native feedback, token-pressure memory, enterprise quality, consumer hardware and Windows remain open.


## Packaged context feedback and recovery: 2026-10-10

- [x] Rebuild the isolated desktop package with structured context-limit feedback.
- [x] Verify native rejection retains results and a subsequent answer succeeds on the same engine.
- [x] Record coalition measurements, persistent counts and normal shutdown.

Evidence: [native context follow-up](../evaluation/context/README.md#packaged-native-feedback-and-recovery). Build 27.26 s; 114.87 MiB bundle. Five dense matches produced the actionable error; recovery answered eight dollars with one citation, 196 input tokens, 2.2 s. Engine PID unchanged. 1,336 samples; final coalition empty. Test profile: 7 documents, 66 chunks, 60 embeddings. Six new text passages remain unembedded. Successful near-full-context memory, consumer hardware, representative enterprise quality and Windows acceptance remain open.


## Successful context-pressure qualification: 2026-10-10

- [x] Locate a deterministic dense-evidence workload that successfully approaches the 4,096-token context through the production answer path.
- [x] Measure the successful workload in the isolated release desktop and verify recovery/shutdown.
- [x] Record model quality, actual token counts and process-accounting limits without inferring consumer-device acceptance.

Four fixture sizes (900, 1,100, 1,250 and 1,400 characters per source) use four sources each and a fixed budget rubric. Fixture-only work is exempt from test-first; the existing production probe provides execution evidence.

Evidence: [pressure fixture selection](../evaluation/context/pressure.md). Twelve calls: eight correct four-citation answers, two structure errors, two explicit context rejections. Selected fixture: 3,648 prompt + 301 completion tokens; desktop import files prepared. No probe/engine processes remained. Native process measurement and structure-failure diagnosis remain open.

Native follow-up: [pressure accounting](../evaluation/context/pressure.md#native-desktop-pressure-measurement). Two correct four-citation answers, 3,646 input tokens; first 8.5 s/zero cached, repeat 5.2 s/3,645 cached. Short recovery passed. 882 samples, 880 valid, final coalition empty. Test profile 11 documents/70 chunks/60 embeddings. Structure-failure diagnosis, Windows, target-hardware and enterprise-quality qualification remain open.


## Generation-limit feedback: 2026-10-10

- [x] Capture the synthetic failing response using an isolated copy of the production inference path.
- [x] Reproduce generic structure feedback in a failing regression, then recognize the exact structured `finish_reason: length` before parsing answer content.
- [x] Run focused/full Rust tests, all-target desktop Clippy and formatting.
- [x] Recheck the real-model failure followed by successful near-limit recovery; record artifacts and remaining native-package work.

Captured response: 3,783 prompt + 313 completion tokens = 4,096, finish reason `length`, JSON cut inside a source ID. The temporary diagnostic example was removed from the Cargo examples directory; production never writes raw responses. Source patch adds seven production lines and one focused HTTP integration regression. 156 full Rust tests passed (3.05 s); thirteen focused tests passed (2.01 s); Clippy passed (2.52 s).

Evidence: [generation-limit report](../evaluation/context/length-limit.md). Four final calls: two expected limit errors and two correct four-citation recovery answers on the same engine; zero remaining probe/engine processes. The native package still needs rebuilding and message verification. Windows, consumer-hardware and enterprise-quality qualification remain open.


## Packaged generation-limit feedback: 2026-10-10

- [x] Rebuild the isolated release package with exact finish-reason handling.
- [x] Exercise native generation-limit feedback and a subsequent successful short answer.
- [x] Verify retained results, engine continuity, stored fixture contents and shutdown; record evidence.

The native fixture has four 1,300-character sources under the unique `LimitBudget` keyword, avoiding overlap with existing pressure-test documents. No product settings or default models change.

Evidence: [native limit-message follow-up](../evaluation/context/length-limit.md#packaged-native-follow-up). Build 27.68 s, 114.87 MiB. Four results retained after exact limit message; recovery answered eight dollars with one citation in displayed 2.2 s. Engine PID unchanged; zero test-bundle executables after quit. Test profile 15 documents/74 chunks/60 embeddings. No executable changes in this follow-up. Windows, target hardware, enterprise quality and remaining resource qualification stay open.


## Maximum-pixel detailed image: 2026-10-10

- [x] Prepare a deterministic 20-megapixel grayscale noise PNG within the encoded byte limit.
- [x] Measure native release import with the existing coalition collector.
- [x] Verify normalized stored dimensions, decoder observations and shutdown; report scope limits.

This extends the earlier solid-image boundary checks with incompressible pixel data. It remains a synthetic decoder workload, without enterprise retrieval-quality claims.

Evidence: [detailed maximum-pixel image](../evaluation/resources/complex-image/README.md). 20,010,168-byte / 20MP PNG imported and preview opened; stored 512 × 410 / 143,181 bytes. 706 samples, 702 valid, no sampled decoder, final coalition empty. Separate direct FFmpeg run: 80,248,832-byte maximum RSS, 62,702,976-byte peak footprint, output hash identical to native preview. These peaks are separate measurements. Windows, target hardware, enterprise quality and remaining resource gates stay open.


## Delivery environment audit: 2026-10-10

- [x] Reconcile the available Mac package and bounded evidence into a current delivery summary.
- [x] Check the local environment for an executable Windows qualification path.
- [ ] Receive an available authorized Windows machine or existing repository/runner, then execute native Windows delivery checks.

[Delivery status](../docs/delivery-status.md) links completed behavior evidence and open gates. [Environment check](../evaluation/delivery/environment.json): macOS, 36 GiB, Git exit 128, named VM apps absent and QEMU/Wine unavailable on PATH. Remote hosts were not inventoried. Windows runner details were requested; no response or authorization is assumed. No deployment, repository creation or paid setup was attempted. This is the first current availability audit; full objective remains active and incomplete.


## Delivery environment blocked audit: 2026-10-10

The same Windows execution-environment requirement remains unresolved for three
consecutive goal turns. [Third check](../evaluation/delivery/availability-audit-3.json)
confirms the current host and named local tool availability; no Windows target or
authorized repository/runner has been supplied. The preceding turn was no progress.
Native Windows delivery and consumer-device acceptance remain unverified. Await
the requested environment details before running those checks. Existing Mac
evidence is retained; no repeated tests or unrelated implementation changes were
made. Full product completion is unproven.
