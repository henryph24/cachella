# Mixed-media evidence presentation

Date: 2026-10-10. This increment changes the production media request builder in
`src-tauri/src/inference.rs`. It adds a modality field derived from validated
payloads and stably presents audio after the other selected evidence. Retrieval
selection, displayed result order, source IDs, timestamps, four-source limit and
payload limits are preserved. Text-only requests retain their existing path.

## Evidence and correction

The original budget-video case omitted its visible red circle in all four
audio-first conditions. A second fixture combines a blue square with the existing
synthetic lunch recording. Its baseline answers incorrectly said the video showed
a lunch in all eight conditions, including frame-first conditions. This rules out
source sorting alone as a sufficient correction for the two tested fixtures.

Both segments previously had `kind: video`, with no explicit modality in their
metadata. Adding `modality: image` or `modality: audio` corrected the four
frame-first lunch answers. The four audio-first lunch answers still described
spoken food as visible content. The final change combines these explicit labels
with stable audio-last presentation. The additional completeness instruction
rejected in [order.md](order.md) remains absent.

The inspected bundled-engine source preserves OpenAI content-array order in
`tools/server/server-common.cpp::oaicompat_content_load_media` and iterates media
parts in `tools/mtmd/mtmd.cpp`. Gemma audio uses explicit begin/end markers.
No image-first requirement was found in those inspected paths. This is evidence
of model/request-presentation sensitivity; no upstream engine defect is claimed.
The change mitigates the demonstrated cases without establishing general immunity
to multimodal hallucination.

## Real-model results

The same production decoder, temporary SQLite store, bounded media loader,
managed TLS runtime and answer validator are exercised by `media_order_probe`.
Each eight-case matrix uses two input orders, two cache flags and two repetitions.
The `order` field records the supplied source order. In final runs the builder
normalizes presentation internally; returned `sources` retain the supplied order.
These are repeated synthetic cases, not 24 independent user questions.

| Run | Structurally valid | Factually complete or correct abstention | Report |
| --- | --- | --- | --- |
| Lunch baseline | 8/8 | 0/8 | [baseline](order-lunch-baseline.json) |
| Lunch, modality labels only | 8/8 | 4/8 | [modality](order-lunch-modality.json) |
| Lunch, labels and audio-last | 8/8 | 8/8 | [lunch final](order-lunch-normalized.json) |
| Budget, labels and audio-last | 8/8 | 8/8 | [budget final](order-budget-normalized.json) |
| Unknown revenue, labels and audio-last | 8/8 | 8/8 | [abstention final](order-unknown-normalized.json) |

Manual review of every recorded answer found that final lunch cases describe the
blue square and list tomato soup, fresh bread and green salad. Final budget cases
describe the red circle and $8/month. Each cites both original source IDs.
All revenue cases set `insufficient_evidence: true` and return zero citations;
the production validator supplies the standard abstention message.

Each run ended with its engine stopped and listener closed. Final process
inspection found zero probe/guardian executables. Other user engines were left
running. Source/fixture fingerprints and the FFmpeg construction command are in
[normalization-provenance.json](normalization-provenance.json).

## Reproduction

The probe now accepts an optional question and imports the actual fixture filename
as its title. The prior budget filename and default question preserve the earlier
baseline input. For the new fixture:

```sh
cargo build --manifest-path src-tauri/Cargo.toml --example media_order_probe
src-tauri/target/debug/examples/media_order_probe \
  src-tauri/resources/engine/llama-server \
  .local/models/gemma-4-E2B-it-Q4_0.gguf \
  .local/models/mmproj-gemma-4-E2B-it-Q8_0.gguf \
  src-tauri/resources/decoder/ffmpeg \
  evaluation/multimodal/fixtures/lunch-video.mp4 \
  /tmp/cachella-lunch-repeat.json \
  'What does the video show, and what food does the recording mention?'
```

Use `budget-video.mp4` and omit the final question for the budget check. Use
`What annual company revenue is stated in the video?` for the abstention check.
Keep a distinct output filename to preserve historical results.

## Tests and boundary impact

The modality assertion failed first (`Null` versus `image`). The mixed-media
presentation regression then failed on unequal content arrays for reversed input
orders. After the correction, it verifies equal presentation plus source IDs,
modality labels, timestamps and image/audio payload positions.

- Focused inference suite: 10 passed, 0 failed in 0.03 s.
- `cargo test --manifest-path src-tauri/Cargo.toml --lib --features desktop`:
  139 passed, 0 failed in 1.81 s; process guardian fixture built first.
- All-target desktop Clippy with `-D warnings`: passed in 13.22 s.
- Rust formatting check: exit 0.
- Production-request integration: 24 final outcomes; manual review above.

The desktop already calls this builder after choosing its top four hits. Sorting
borrowed references affects only request presentation. Media existence, MIME,
size, timestamp and foreign-source validation still apply, and final citation
validation maps IDs to the original selected sources. No schema, persistent
storage, cancellation or process lifecycle changes were introduced.

Frontend tests and packaged UI execution were skipped for this Rust request-only
change. The retained native test app contains the earlier request builder and
must be rebuilt before verifying this correction through its UI. Windows runtime,
broader media and adversarial quality, temporal reasoning and whole-app consumer
hardware qualification remain unfinished.

## Subsequent packaged verification

[Native verification](native-normalized.md) rebuilt the isolated application and
confirmed the corrected combined answer with audio-first retrieval, both citations
and a 455-token reuse counter on a repeat. The earlier package limitation above
is superseded by that report; broader quality and hardware qualification remain
unfinished.
