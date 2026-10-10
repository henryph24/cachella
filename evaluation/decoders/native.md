# Fresh-profile native decoder verification, 2026-10-10

Verified on macOS ARM64 using an isolated application identifier. No executable source changes were required in this increment.

## Build and isolation

Built one 144.99 MiB debug bundle with:

```sh
npx tauri build --debug --bundles app --features desktop --config src-tauri/tauri.engine.conf.json --config src-tauri/tauri.decoder.conf.json --config '{"identifier":"com.cachella.decodercheck","productName":"Cachella Decoder Check"}'
```

[Build output](native-check-build.txt) records the bundle path. The profile `~/Library/Application Support/com.cachella.decodercheck` did not exist before launch. The native runtime page confirmed that profile path and an included decoder inside `Cachella Decoder Check.app/Contents/Resources/decoder/`. The existing `com.cachella.desktop` app and profile were not selected or modified during this test workflow.

## Native workflow

CUA operated the packaged application and native file picker. The fresh library showed zero documents. Without choosing or installing FFmpeg, importing the synthetic `budget-video.mp4` created one document, two passages and a stored video preview. The preview displayed its frame/audio timestamps and rendered the red-circle fixture. Playback advanced from zero seconds to its three-second end.

After a normal quit, process inspection confirmed the test executable had exited. Relaunch showed the same one document and two passages; the runtime page still reported the included decoder selected. The test app was quit again afterward. [Transcribed accessibility observations](native-ui-observations.txt) record the relevant states; they are manual native-UI evidence.

A read-only SQLite check after the final quit reported:

- 1 document and 2 chunks.
- 1 `video/mp4` asset, 38,715 bytes.
- 1 `audio/wav` part, 97,418 bytes, covering 0–3,042 ms.
- 1 `image/png` part, 14,322 bytes, starting at 0 ms.
- 0 embeddings, since no embedding model was started.
- No `media-tools.json` override, consistent with fresh included-decoder selection.
- No remaining executable from the isolated test app in `ps -axo pid=,comm=`.

[Database/process check output](native-profile-check.txt) records these assertions. The initial process check matched its own shell command text; checking executable names through `comm` corrected that observation error. No application defect was demonstrated.

## Scope and remaining work

This establishes fresh-profile native video import, local preview playback and persistence through restart on macOS. The previously recorded CLI fixtures cover the other supported extension families; this UI run used one video fixture. It does not establish media answer generation, auditory content accuracy, native Windows behavior or consumer-hardware performance.

The isolated app bundle and its one-fixture profile are retained for further verification; the test app is stopped. Windows/MSYS2 execution, installer/signing checks, media-to-text answers, broader quality/resource qualification and final application acceptance remain unfinished. Unit and lint suites were not repeated because executable source was unchanged; the bundle's frontend build and native desktop compilation passed. No publication or deployment occurred.
