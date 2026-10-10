# Packaged macOS media-answer verification

Date: 2026-10-10. App: `Cachella Media Check.app`, identifier
`com.cachella.mediacheck`. Hardware: Apple M3 Pro, 36 GiB physical RAM.
This report transcribes native CUA observations; it is not an automated UI-test
transcript. The original `com.cachella.desktop` library was not used.

## Preparation and scope

The isolated profile started with fresh settings/library and four preseeded,
hard-linked model files. The UI installer verified the cached Gemma 4 E2B Q4_0
and EmbeddingGemma 2 Q8_0 model/projector pairs. This checks existing-file
verification and selection; it does not verify a fresh installer network transfer.
The included engine and decoder were selected automatically. The synthetic
`evaluation/multimodal/fixtures/budget-video.mp4` was imported using the native
file picker, producing one document and two media passages. The embedding engine
started and the UI reported `2 of 2 passages indexed`.

Build command:

```sh
npx tauri build --debug --bundles app --features desktop \
  --config src-tauri/tauri.engine.conf.json \
  --config src-tauri/tauri.decoder.conf.json \
  --config '{"identifier":"com.cachella.mediacheck","productName":"Cachella Media Check"}'
```

Final build log: [native-ui-build-final.txt](native-ui-build-final.txt).
The debug application occupied 145.04 MiB. This is an isolated test package;
no release was signed, published or deployed.

## Installer navigation regression

The first packaged check found that cancelling Gemma verification after leaving
and returning to settings restored the Qwen catalog default and hid the terminal
message. `ModelDownload` now retains the installer bundle through terminal status
until an explicit new selection. The rebuilt app preserved Gemma selection and
showed `Download cancelled.` after the same navigation/cancel sequence. Existing
model/projector settings remained intact and Start model became available.
Focused regressions and the full frontend suite passed: 58 tests across six files.
The capability label now states `Text and retrieved media with compatible local models`.

## Answer and citation observations

Both generation and embedding engines were running. Semantic search returned the
stored video audio and frame, each with 0–3,042 ms metadata.

| Question | Exact answer | UI metrics | Assessment |
| --- | --- | --- | --- |
| What does the video show, and what monthly cloud budget does it mention? | The video is titled "budget-video.mp4" and mentions that the project cloud budget is eight dollars per month. | 4.0 s; 450 input tokens; 0 cached tokens; 2 citation buttons | Partial: correct budget, omitted visual description. The frame was retrieved, so retrieval absence does not explain the omission. |
| What shape and color are visible in the video? | A red circle is visible in the video. | 2.6 s; 445 input tokens; 67 cached tokens; 1 citation button | Correct visual description. |

Opening the combined answer's second citation displayed `budget-video.mp4`,
`Matched interval: 0:00–0:03`, and a local media URL. Pressing Play showed the red
circle, a Pause control and a three-second duration; a subsequent observation
showed elapsed time of three seconds and the Play control again. Audio content
was not independently assessed by listening during this UI check.

The two requests differ, so their timings do not measure cache savings. The
reported 67 cached tokens establish an engine counter observation only. The
combined-answer omission remains unresolved. Earlier backend combined-answer
success cannot establish repeatable completeness across retrieval orders and
runtime contexts. Further controlled quality evaluation is required.

## Persistence and shutdown

After normal quit, read-only SQLite queries counted:

- 1 document, 2 chunks and 2 embedding rows.
- 1 preview asset and 2 stored media parts.

Relaunch showed the document and two passages. Settings displayed the persisted
Gemma generation model/projector and EmbeddingGemma model/projector, with both
runtimes stopped. The included decoder remained selected. Normal quit after that
restart left zero processes whose executable path belonged to the test bundle.
Raw read-only evidence: [native-ui-persistence.json](native-ui-persistence.json).
The isolated test profile and package remain available.

## Verification limits

This continuation changed documentation only; test-first is exempt. Native
interaction, read-only database/settings inspection and scoped process checks
provided new evidence. No executable code changed during this continuation, so
Rust, frontend and lint suites were not repeated. The preceding build includes
the tested terminal-selection correction and capability text update.

Pending: combined-question answer completeness, broader media/retrieval quality,
whole-application memory and latency on target consumer hardware, native Windows
execution and installer qualification, release signing and final acceptance.
