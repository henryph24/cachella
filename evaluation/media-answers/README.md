# Small multimodal generation feasibility

Status: verified downloads, native feasibility and backend media-answer
integration are complete for the documented synthetic checks. Packaged macOS UI
verification is recorded in [native-ui.md](native-ui.md): retrieval, citations,
playback, persistence and shutdown worked; a combined question omitted the visual
description. Broader quality and consumer hardware qualification remain pending.
[Controlled order evaluation](order.md) reproduces the omission in all four
audio-first baseline requests, with caching enabled and disabled. A prompt
experiment was rejected after four answer-format failures; production behavior
remains unchanged in that historical experiment.

[Subsequent evidence-presentation correction](normalization.md) adds validated
modality labels and stable audio-last presentation. It produced 16/16 complete
answers across two synthetic videos and 8/8 abstentions for an unsupported
question. [Rebuilt native UI verification](native-normalized.md) now confirms
the corrected combined answer, both citations and the llama.cpp reuse counter.
Broader quality and consumer hardware qualification remain pending.

## Integrated answer path

`ask_library` now loads stored media evidence when its managed generation model
has a projector. It uses the same operation permit, TLS credentials, cancellation
and post-generation source-existence checks as text answers. The loader reads
only the first four retrieved candidates and bounds each SQLite payload read to
2 MiB before materializing it. Request construction validates PNG/WAV signatures,
source modality and timestamps, and binds each payload to JSON source metadata.
Structural media labels are excluded from factual evidence. Qwen and external
text-only connections preserve the original text filtering and media abstention.

`multimodal_smoke` now optionally accepts generation-model and projector paths
after its original arguments. Run:

```sh
cargo build --manifest-path src-tauri/Cargo.toml --example multimodal_smoke
src-tauri/target/debug/examples/multimodal_smoke \
  src-tauri/resources/engine/llama-server \
  .local/models/embeddinggemma-2-Q8_0.gguf \
  .local/models/mmproj-embeddinggemma-2-Q8_0.gguf \
  src-tauri/resources/decoder/ffmpeg evaluation/multimodal/fixtures \
  evaluation/media-answers/integration-video.json \
  .local/models/gemma-4-E2B-it-Q4_0.gguf \
  .local/models/mmproj-gemma-4-E2B-it-Q8_0.gguf
```

The report records five import preparations, persisted/reopened embeddings, five
retrieval ranking checks and seven completed answer requests. The initial five
answer requests cover image, audio, video-frame, video-audio and text rankings;
the image/video-frame and audio/video-audio queries repeat the same questions.
Mixed-source answers correctly describe the red circle, $8/month budget and
Priya's ownership. Their citations can select standalone media twins, so two
additional requests explicitly use the stored video's frame and audio chunks.
The combined question answers “red circle” and “$8 per month” and cites both
video chunk IDs with 0–3,042 ms metadata. The unknown-revenue question abstains.
This establishes decomposed-video evidence handling for this fixture; it does
not establish long-video temporal reasoning or broad retrieval quality.

The generation catalog now offers Gemma 4 E2B as an optional 3.40GB download with
its matching projector; Qwen remains the initial choice. The generation card
displays an installed projector read-only. A manually chosen generation model
clears a prior projector after settings persistence succeeds. A settings-remount
regression ensures an active Gemma download remains visible and cancellable when
status arrives before the catalog.

Boundary tests initially failed before media request construction existed; the
manual-model-switch regression also failed before its fix. The integrated Rust
suite reported `138 passed; 0 failed` in 1.22 seconds. All-target desktop Clippy
passed in 2.22 seconds. Native smoke checks confirmed persistence, timestamps,
the text-only guard, source deletion and both engine listeners closed on stop.
These checks exercise production backend components without the Tauri UI.

Installer reuse: two already-downloaded files were hard-linked into the isolated
`.local/gemma4-install-check` root and reverified through the production catalog
installer. `install_models_smoke` reported `PASS: 2 verified model file(s),
3398849248 bytes, 101065 ms`; `installer-reuse.json` records the ready state.
This checks catalog-to-installer wiring and existing-file validation. It does not
repeat a network transfer through the desktop installer.

Frontend checks before packaged verification: `npm test` reported `57 passed` across six files;
`npm run build` completed TypeScript and Vite compilation. Two independent review
rounds identified the stale projector on manual model switching and catalog-gated
download cancellation/busy controls. The latter fix now renders generic progress
and Cancel before catalog resolution, including catalog failure, and reports
global installer busy state to each parent runtime card. Regressions assert these
controls before resolving the catalog. Both findings were fixed and verified by
the applicable tests; no third independent review was performed. Native packaged
interaction, installer network transfer, Windows execution and whole-app resource
qualification remain outside this increment's verification.

## Observed results

Both pinned files passed exact-size and SHA256 checks; see
`download-verification.json`. The native runs used an Apple M3 Pro with
38,654,705,664 bytes of physical memory. Startup, private TLS/authentication and
owned-process shutdown completed in both runs.

| Check | Direct requests | Structured answers |
| --- | ---: | ---: |
| Questions completed | 6 | 6 |
| Fixture content rubrics met by manual review | 5 | 6 |
| Production answer validation passed | Not requested | 6 |
| Request elapsed range | 305–981 ms | 850–1,520 ms |
| Startup | 16,679 ms | 1,454 ms |
| Memory samples | 72 | 32 |
| Sampled simultaneous resident sum peak | 1,064,566,784 bytes | 1,135,738,880 bytes |
| Sampled simultaneous footprint sum peak | 818,881,088 bytes | 833,070,464 bytes |
| Owned process group empty after completion | Yes | Yes |

The direct lunch transcript says “Launches tomato soup…”; the fixture says
“Lunch is tomato soup…”. The structured run transcribes that phrase correctly.
Both image shape/color descriptions, the budget transcript, the $8/month answer
and the revenue abstention meet their fixture rubrics. This is manual review of
six synthetic questions per run, with four unique fixtures. The second prompt
and response constraint changed, so the differing transcript does not establish
general transcription reliability or a reproducible fix.

The structured run uses the production JSON schema and system instruction and
calls `inference::validate_answer` with the supplied fixture source ID. All six
objects passed validation; the insufficient-evidence response was normalized by
the production validator. This checks source membership and structure. It does
not automatically establish factual support, which was inspected separately.

Raw evidence: `native/answers.json`, `native/resources.json`,
`native-grounded/answers.json`, `native-grounded/resources.json`. The latter run
adds `--grounded` to the command below and uses a separate output directory.
The initial direct-run probe and the later grounded-capable probe have different
binary hashes, preserved in their resource reports. Filesystem caches and
background workload were uncontrolled; startup differences are not a cold/warm
benchmark. Resource accounting excludes some clean mapped pages and does not
measure the full desktop with concurrent embeddings. The 3.40GB download and
these process counters cannot establish total RAM requirements.

## Managed-runtime probe

The native `media_answer_probe` example is the preferred next check. It uses
`Runtime::start`, the production TLS/authentication check, projector validation
and process guardian. It sends six direct multimodal requests with thinking
disabled and bounded fixture/response sizes. These requests do not yet traverse
the desktop retrieval/citation path.

```sh
cargo build --manifest-path src-tauri/Cargo.toml --example media_answer_probe
python3 evaluation/media-answers/measure_native.py \
  --probe src-tauri/target/debug/examples/media_answer_probe \
  --engine src-tauri/resources/engine/llama-server \
  --model .local/models/gemma-4-E2B-it-Q4_0.gguf \
  --projector .local/models/mmproj-gemma-4-E2B-it-Q8_0.gguf \
  --fixtures evaluation/multimodal/fixtures \
  --sampler .local/media-memory-sample \
  --output evaluation/media-answers/native
```

`measure_native.py` reuses the existing process-group collector from
`evaluation/resources/measure.py`. It records the native harness, guardian and
engine, verifies the owned group is empty on successful completion, and saves
artifact hashes and hardware metadata. `answers.json` contains raw answers;
`resources.json` contains events, samples and phase summaries. Phase names can
cover multiple questions about the same fixture. Latency is full-request elapsed
time. The collector has a 600-second overall timeout.

Verified preparation: native example built in 5.36 seconds; all-target desktop
Clippy passed in 1.75 seconds; formatting and Python compilation/help passed.
A deliberately exited engine caused the native startup error, left its owned
process group empty and produced no answer artifact. The first fixture setup
attempt failed while copying macOS protected file flags; copying file bytes and
setting executable permissions allowed the actual failure-path check to run.
Subsequent real candidate answers and resource samples are summarized above.

## Candidate and direct-engine diagnostic

The current desktop uses multimodal embeddings for retrieval, but its Qwen answer
path retains only text/Markdown/PDF hits. Media chunk labels describe timestamps
and content types; they contain no transcript or caption. Embedding vectors alone
cannot provide factual media evidence to the text generation model.

Candidate: `ggml-org/gemma-4-E2B-it-GGUF`, pinned revision
`b4243c156154b6dca9324415f8c7ccc098b4aed1`. The model API metadata reports
4,647,450,147 parameters and Apache-2.0 licensing. Q4_0 model weights plus the
Q8_0 projector total 3,398,849,248 bytes. Runtime memory remains unmeasured.
The source metadata is preserved in `model-metadata.json` and `tree-metadata.json`.
Matching size/SHA256 evidence is recorded in `download-verification.json`.

This is exploratory feasibility work with test-after validation. The probe is
separate from the production runtime and leaves desktop settings unchanged.
It binds an authenticated loopback server, disables thinking, limits context to
4,096 tokens, uses one slot, and submits six bounded image/audio questions.
The fixtures are synthetic; their expected content is documented in
`../multimodal/README.md`. Responses require manual content review. No substring
scoring establishes semantic correctness.

```sh
python3 scripts/probe_media_answers.py \
  --engine src-tauri/resources/engine/llama-server \
  --model .local/models/gemma-4-E2B-it-Q4_0.gguf \
  --projector .local/models/mmproj-gemma-4-E2B-it-Q8_0.gguf \
  --fixtures evaluation/multimodal/fixtures \
  --output evaluation/media-answers/probe \
  --sampler .local/media-memory-sample
```

Build the optional macOS helper with
`cc -Wall -Wextra -Werror evaluation/resources/memory_sample.c -o .local/media-memory-sample`.
This reuses the existing SDK-based process accounting helper. Snapshots after
startup and each answer include resident bytes, physical footprint and the
kernel's lifetime peak footprint for the owned engine PID. These observations
exclude the desktop, embedding engine, decoder, Python harness and operating
system. Clean mapped pages can be excluded from footprint; the lifetime peak
footprint is not total working-set memory. See `../resources/README.md` for the
accounting limitations. These measurements cannot certify a 16GB device.

Checks completed: Python compilation and command help passed. An exited-engine
failure probe returned an error, recorded zero answer cases and confirmed the
owned engine stopped. The script records full responses and request elapsed
time. Elapsed time includes prompt processing and generation; it is not time to
first token.

Outstanding: packaged UI verification, native Windows checks and qualification
of whole-app resources and quality. The isolated feasibility probe does
not verify production retrieval-to-answer wiring, media citations, video temporal
reasoning, process cleanup after parent termination, native Windows behavior or
the 16GB consumer hardware target. The desktop default remains unchanged.

## Runtime preparation

The managed runtime now validates and passes an optional generation projector.
Embedding mode still requires its projector and retains the embedding-only flags;
generation keeps the 4,096-token context and one slot. The argument-capture
regression failed before implementation because `--mmproj` was absent, then
passed with the change. It also checks invalid-projector rejection and generation
without a projector. The test uses the existing process guardian fixture.

Verification: `cargo test --manifest-path src-tauri/Cargo.toml --lib --features desktop`
reported `134 passed; 0 failed` in 1.41 seconds. All-target desktop Clippy with
`-D warnings` completed successfully in 9.99 seconds; formatting check passed.
The macOS memory helper compiled with `-Wall -Wextra -Werror` and returned three
positive counters for an explicitly requested live PID. The probe's exited-engine
cleanup check still passed with memory sampling configured.

The grounded-probe extension built in 4.05 seconds; all-target desktop Clippy
passed in 0.77 seconds, and Python compilation passed. The real run completed
with `PASS: 32 samples; owned process group empty`. This exploratory evaluation
uses test-after verification through the actual engine.

The earlier runtime-only increment was preparatory. The integrated path and
optional catalog bundle are now described above. Manual generation-projector
selection is not exposed; bundled model installation selects its matching pair.
Subsequent packaged macOS verification and its quality limitation are recorded
in [native-ui.md](native-ui.md).
