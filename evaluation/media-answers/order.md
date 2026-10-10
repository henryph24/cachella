# Compound media answer: controlled reproduction

Date: 2026-10-10. Apple M3 Pro, 36 GiB RAM. Gemma 4 E2B Q4_0 with its
matching Q8_0 projector, bundled llama.cpp b11531. Model pins and hashes are in
[download-verification.json](download-verification.json). One synthetic three-second
video is used. This evaluation establishes a reproducible source-order effect for
this fixture; it does not establish a general model or engine root cause.

## Method

The evaluation-only `media_order_probe` imports the video through production
FFmpeg preparation and SQLite storage, loads its exact frame/audio payloads with
the production bounded loader, and calls `answer_with_media_authenticated`.
It uses the managed TLS/authenticated runtime, 4,096-token context, one slot,
temperature zero and disabled thinking. It deliberately controls source order;
no embedding engine or retrieval ranking is exercised in this probe.

Question: “What does the video show, and what monthly cloud budget does it mention?”

Each run executes cache disabled then enabled; within each block it executes
audio-first then frame-first, twice per condition. One engine instance serves
each eight-request run. This sequence is fixed and filesystem caches/background
workload are uncontrolled. The two repetitions expose repeatability within this
run; they are not independent hardware samples. No model-answer substring score
was used: full responses and citations were manually reviewed.

```sh
cargo build --manifest-path src-tauri/Cargo.toml --example media_order_probe
src-tauri/target/debug/examples/media_order_probe \
  src-tauri/resources/engine/llama-server \
  .local/models/gemma-4-E2B-it-Q4_0.gguf \
  .local/models/mmproj-gemma-4-E2B-it-Q8_0.gguf \
  src-tauri/resources/decoder/ffmpeg \
  evaluation/multimodal/fixtures/budget-video.mp4 \
  evaluation/media-answers/order-baseline.json
```

Use a new report filename when rerunning to retain historical evidence.
The probe reports request outcomes including errors and writes after every case.
Its final PASS confirms recording and shutdown; factual correctness requires the
separate review below.

## Baseline results

Raw report: [order-baseline.json](order-baseline.json).

| Source order | Cache flag | Complete answers | Elapsed milliseconds |
| --- | --- | --- | --- |
| Audio, frame | Disabled | 0/2 | 3916, 3622 |
| Frame, audio | Disabled | 2/2 | 3556, 3548 |
| Audio, frame | Enabled | 0/2 | 2973, 2961 |
| Frame, audio | Enabled | 2/2 | 2837, 2862 |

All eight responses passed the production answer-structure and source-membership
validator. Every audio-first answer was: `The video is titled "budget-video.mp4"
and mentions that the project cloud budget is eight dollars per month.`
It cites both source IDs but omits the visible red circle. Every frame-first
answer was: `The video shows a red circle. The project cloud budget is $8 per month.`
It also cites both source IDs, covering the visible and spoken evidence.

Each baseline request reports 450 prompt tokens. Cache-disabled cases report zero
cached tokens; cache-enabled cases report 445. The incomplete answer persists in
both cache conditions. Recorded elapsed times are lower in the enabled block;
the fixed sequence and small sample do not support a general savings estimate.

## Rejected instruction experiment

Raw report: [order-completeness-instruction.json](order-completeness-instruction.json).
The experiment appended the following to the final media question text:

> Address every part of the question using the supplied evidence. Use visible and audible content to describe media. State any detail the evidence leaves unknown.

Four audio-first cases failed production answer-structure parsing, returning
`The model did not return the required answer structure. Your sources remain available.`
All four frame-first cases retained the complete answer. Raw invalid model output
is not exposed by this production API, so the exact syntax/length failure remains
unidentified. It would be unsupported to attribute it to token truncation.

This candidate was rejected and the original production prompt was restored.
The final source and rebuilt probe use the baseline prompt. Sorting production
media to fit this single fixture was not adopted. Additional paired fixtures,
paraphrases, abstention cases and inspection of invalid raw responses are needed
to select a general correction.

## Verification

- Probe build passed before the runs and after restoring the prompt (final 3.25 s).
- `cargo test --manifest-path src-tauri/Cargo.toml --lib inference::tests`:
  `9 passed; 0 failed` in 0.03 s, including the exact original question payload.
- `cargo-clippy clippy --manifest-path src-tauri/Cargo.toml --example media_order_probe -- -D warnings`:
  passed after restoration in 1.09 s.
- `cargo-fmt fmt --manifest-path src-tauri/Cargo.toml -- --check`: exit 0.
- Both real-engine reports contain eight cases and `engine_stopped: true`;
  each probe also checked its generation listener was closed.
- Read-only process inspection found zero remaining `examples/media_order_probe`
  executables, including guardians. Unrelated engines were left running.

The new evaluation harness uses exploratory test-after verification against the
actual engine. The reproducible quality failure was captured before the candidate
prompt edit; that edit was rejected. No production behavior change remains.
Frontend tests and repackaging were skipped because frontend/package behavior is
unchanged. Windows, broader quality, whole-app consumer resource qualification
and the incomplete compound answer remain unfinished.
