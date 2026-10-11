# Cachella acceptance audit

Date: 2026-10-10. Overall result: incomplete. This audit preserves the local-first
Windows/macOS knowledge-app objective in [the implementation plan](../tasks/todo.md)
and [HTML product contract](local-first-plan.html). It separates demonstrated
macOS and hosted Windows behavior from remaining qualification and delivery work. Historical reports
retain their original scope; their passing counts are not universal guarantees.

Current handoff: [delivery status](delivery-status.md) identifies the available
Mac test package, evidence limits and Windows environment requirements.

## Requirement-to-evidence map

| Requirement | Current evidence inspected | Assessment |
| --- | --- | --- |
| Detailed HTML architecture plan | [Current plan](local-first-plan.html), [historical Fly proposal](cachella-demo-plan.html) | Present. Tauri/Rust/SQLite/llama.cpp and optional enterprise LMCache are explicit. |
| Cross-platform local desktop | Tauri source; [Mac package run](../evaluation/media-answers/native-normalized.md); [Windows workflow](../.github/workflows/desktop-checks.yml); [Windows qualification](windows-runtime-qualification.md) | Mac execution and Windows Server 2025 release installation, GUI readiness/close, installed helpers, Qwen3 inference and five retrieval cases verified. Windows 11 interactive acceptance and consumer hardware remain open. |
| Persistent text/document library and citations | Store implementation; [PDF report](../evaluation/pdf/README.md); [native persistence](../evaluation/media-answers/native-ui-persistence.json) | Implemented and exercised within those fixture scopes. PDF extraction is selectable text; OCR and original-layout rendering are outside this path. |
| Text/image/audio/video search | [Multimodal results](../evaluation/multimodal/README.md); [integration report](../evaluation/media-answers/integration-video.json) | Local indexed retrieval demonstrated on synthetic fixtures. Video is decomposed into timed frames/audio. Continuous temporal understanding is unproven. |
| Small quantized local generation | [Gemma text assessment](../evaluation/gemma4-e2b-text-assessment.md), [media correction](../evaluation/media-answers/normalization.md) | Grounded generation demonstrated. Eight text rubrics and two media fixtures are bounded development evidence; representative enterprise quality remains open. |
| Citation navigation and absent-evidence behavior | Production `ask_library` path; [native citations](../evaluation/media-answers/native-normalized.md); text/media abstention records | Source IDs and timestamps remain connected to selected evidence. Structural citation validation does not prove factual support. |
| Local caching and cost demonstration | [Combined resource run](../evaluation/resources/gemma-media.md) | Built-in llama.cpp reuse is measured. LMCache is optional future server work. No cloud resources or hosting-cost savings are claimed. |
| Packaged model, engine and decoder setup | [Engine preparation](../evaluation/engines/README.md), [decoder verification](../evaluation/decoders/README.md), [native setup](../evaluation/media-answers/native-ui.md) | Mac bundled runtime and cached-model verification/selection exercised. Hosted Windows release installation and pinned model downloads passed. Windows 11 end-user setup remains unverified. The cited Gemma UI check preseeded model files. |
| Private runtime, cancellation and recovery | [Lifecycle](../evaluation/lifecycle/README.md), [model recovery](../evaluation/recovery/README.md), [settings recovery](../evaluation/settings-recovery/README.md), [answer cancellation](../evaluation/cancellation/README.md) | Reviewed/tested boundaries and macOS execution recorded. Hosted Windows authentication, stop/restart, credential rotation and owner-drop checks passed; full Windows desktop recovery remains open; no universal crash/security guarantee is inferred. |
| Bounded ingestion and resumable indexing | Media/store limits; [search/index cancellation](../evaluation/search-index-cancellation/README.md); native indexed-row persistence | Per-file, payload, storage and job bounds are implemented. These bounds do not constitute a whole-application memory budget. |
| Consumer hardware/resource budget | `start_runtime`/`Runtime::start_mode` inspection; [component measurements](../evaluation/resources/gemma-media.md) | Incomplete. An optional one-managed-model residency policy is implemented; native switching, cancellation, explicit Stop and policy/session restart behavior are verified in the [residency report](../evaluation/residency/README.md). It enforces process residency without promising a byte-level RAM cap. The [release desktop run](../evaluation/resources/release-desktop.md) measures the app, attributed WebKit/Metal helpers, engines and a separate decoder workload on the 36 GiB Mac. Early launch, system-wide allocations, maximum workloads and the intended 16 GB Windows target remain unverified. |
| Required checks and independent reviews | Recorded Rust/frontend/package results and bounded review reports above | Relevant completed increments have evidence. Full cross-platform acceptance is incomplete, so the goal cannot be marked achieved. |

## Concrete remaining work

1. Qualify the shared residency policy under representative workloads and resource
   pressure on target hardware. Native switching and bounded failure/cancellation
   tests pass; GGUF file size or macOS footprint alone cannot establish RAM requirements.
2. Extend the available release desktop process measurements to early launch,
   resource pressure and intended consumer hardware. The UI/Metal helper process
   run on the 36 GiB Mac remains separate from the intended 16 GB Windows target. Maximum supported context/media workloads and
   representative enterprise documents remain outside current qualification.
3. Complete Windows 11 standard-user install/uninstall, interactive imports, GPU
   behavior and consumer-hardware acceptance. Hosted Windows release installation,
   startup, helper modes and synthetic real-model retrieval passed on 11 October
   2026; see the [qualification report](windows-runtime-qualification.md).
4. Recheck acceptance against those results and document the delivered package's
   limits. Signing/publishing requires the corresponding distribution setup; no
   deployment or publication has been authorized or performed.

Git and the private GitHub remote are initialized. Hosted Windows run 38131003959
completed successfully; the retained report records its bounded acceptance scope.
The existing default model choice remains unchanged. Gemma's new eight-case
result improves evidence for its selectable desktop configuration without
resolving the historical Qwen attack-case utility failure or proving general
injection resistance.

## This audit's verification scope

The audit inspected the current README/product contract, desktop answer/start
paths, runtime configuration, Windows workflow and linked reports. It found and
corrected the README's stale statement that media answers were unimplemented.
The current Gemma text configuration was then exercised in sixteen application
calls (fourteen real generations), with manual review against all eight unchanged
rubrics. Release probe build, example Clippy and formatting passed. No product
executable behavior changed in this increment. The audit itself is documentation
and is exempt from test-first; links and fixture/report structure were validated.

## Subsequent resource-policy investigation

[One-model backend feasibility](../evaluation/media-answers/single-resident.md)
verified retrieval evidence survives embedding shutdown, seven answers complete,
and restarting embeddings preserves the seven-vector index. A configurable
one-managed-model residency limit is the proposed enforceable control. It carries
startup/cache-reuse costs and provides no byte-level memory guarantee. Desktop
coordination, cancellation-safe startup and policy UI are implemented. The [native switch cycle](../evaluation/residency/README.md), pre-spawn cancellation,
explicit Stop and restart policy persistence are verified on the development Mac.
Whole-app hardware qualification remains unfinished.

## Release desktop resource evidence

[Three release desktop runs](../evaluation/resources/release-desktop.md) add 2,165
samples with final coalitions empty. The matched answer phases peaked at 1.165 GiB
and 2.071 GiB of summed process footprint in Single and Both modes. The decoder
follow-up captured FFmpeg in six valid samples on a 60-second synthetic loop.
These bounded process-accounting results preserve exclusions for early launch,
system-wide memory, maximum context/media and Windows; they do not establish
minimum device RAM. The isolated profile now has two documents and 22 passages,
At that run, two vectors were retained and 20 passages were unindexed; the
subsequent packaged recovery run completed all 22 embeddings.

## Hash cancellation and packaged recovery follow-up

The embedding startup hash now receives a cooperative cancellation signal when
its awaiting future is dropped. The shared hash checks between 64 KiB reads;
152 Rust tests, Clippy and independent review passed. A rebuilt isolated release
app verified cancellation recovery (2/22 passages retained), successful retry
(22/22), semantic search (eight matches) and quit (zero test-bundle executables).
The native cancellation arrived after startup; it does not verify interruption
during hashing. Filesystem reads retain their operating-system duration limits.
See [evidence and artifact hashes](../evaluation/residency/hash-cancellation/README.md).
Windows, lower-memory hardware and full product acceptance remain open.

## Maximum-duration import follow-up

[Duration-boundary qualification](../evaluation/resources/duration-boundary.md)
verified a 120-second, 384 × 384 video imports as 36 passages and a 121-second
video is rejected through production decoding and native UI. Existing process
accounting recorded 1,294 samples, including twelve valid FFmpeg observations,
and an empty coalition after quit. This covers the duration boundary on the
36 GiB Mac. Maximum file size/resolution, embedding the new 36 passages, model
context pressure, lower-memory hardware and Windows remain unverified. The
isolated profile now holds three documents, 58 chunks and 22 embeddings.

## Full-duration indexing and answer follow-up

[The completed two-minute fixture run](../evaluation/resources/duration-index-answer.md)
indexed all 36 new parts (24 frames, 12 audio segments), reaching 58/58 stored
embeddings. Search returned eight passages; the generated $8/month answer had
four citations, and a two-minute-video citation opened at 1:00–1:10. Across
1,019 process samples the maximum observed engine count was one and final
coalition membership was zero. The repeated synthetic recording qualifies this
bounded flow; maximum input bytes/resolution/context, representative enterprise
quality, lower-memory hardware and Windows delivery remain open.

## Media byte and geometry correction

[Ten real-decoder boundary cases](../evaluation/resources/boundaries/README.md)
verified exact 32 MiB input acceptance and one-byte-over rejection, image pixel
and side limits, and thin/odd geometry. They exposed zero-sized scale output and
odd-dimension video encoding failures. Shared bounded scale expressions now pass
five regression geometries; all 153 Rust tests and Clippy passed. Normalization
uses v2 fingerprints; existing entries remain intact and reimports may create new
entries. The subsequent package check rebuilt the correction and verified native thin-image
and odd-video imports, stored dimensions and v2 fingerprints; see the report follow-up. Padded files
and solid images establish boundary behavior only; complex maximum-size streams,
context pressure, enterprise quality, lower-memory devices and Windows remain open.

The geometry package follow-up leaves the isolated profile at five documents,
60 chunks and 58 embeddings. Normal quit left zero test-bundle executables.
Two new geometry-test passages remain unindexed; lower-memory hardware, Windows
and representative enterprise acceptance remain open.

The corrected geometry imports subsequently completed native embedding and
retrieval: [follow-up evidence](../evaluation/resources/boundaries/README.md#native-embedding-and-retrieval-follow-up).
The isolated profile has 60/60 embeddings; the previous 58 vectors are unchanged.
Both new vectors are finite normalized 768-D arrays. The video and thin image
ranked first for their respective synthetic queries, and source navigation and
normal shutdown passed. These bounded checks leave target-hardware, Windows,
context-pressure and enterprise-quality qualification open.

## Context-limit feedback and recovery

[Eight real-model calls](../evaluation/context/README.md) showed that character-
bounded evidence can exceed the 4,096-token generation context. Four intentional
oversized requests now return a specific context-limit message through bounded
structured-error parsing; ordinary requests before and afterward returned the
correct budget with valid citations. Full tests (155), final focused inference
checks (12), lint and independent review passed. The subsequent
[packaged native check](../evaluation/context/README.md#packaged-native-feedback-and-recovery)
rebuilt the app, retained five results after the actionable context error, and
returned a correct one-citation recovery answer on the same engine PID (2.2 s,
196 input tokens). The collector recorded 1,336 samples and an empty final
coalition. This measures rejection/recovery phases; successful near-full-context
memory remains unverified. Test profile: seven documents, 66 chunks, 60 embeddings;
six new text passages remain unembedded. Representative enterprise quality,
lower-memory hardware and Windows acceptance remain open.


## Successful context-pressure fixture selection

[Six fixture sizes](../evaluation/context/pressure.md) produced eight correct
four-citation answers across twelve calls. The selected case used 3,648 prompt
and 301 completion tokens within the 4,096-token configuration. Two larger calls
failed structured-answer parsing, with cause unverified; two still larger calls
received explicit context-limit errors. Desktop import files are prepared.
Successful-context native memory measurement and structure-failure diagnosis
remain open, alongside consumer hardware, Windows and enterprise qualification.


The subsequent [native pressure measurement](../evaluation/context/pressure.md#native-desktop-pressure-measurement)
completed two correct four-citation answers with 3,646 input tokens, then a short
recovery answer. 882 process samples cover the release app coalition and its
engine; final coalition empty. First/repeated pressure phases peaked at
1,420,963,264 / 1,423,994,368 summed footprint bytes. This closes the bounded
successful long-context measurement on the development Mac. Early launch,
system-wide memory, complex maximum media and target-device qualification remain
open. Structure-failure diagnosis, Windows and enterprise-quality gates remain
incomplete.


## Generation-limit feedback correction

[Response capture](../evaluation/context/length-limit.md) resolves the pressure
fixture's structure failure: `finish_reason: length`, 3,783 prompt + 313 completion
tokens, truncated JSON at the 4,096-token capacity. The response boundary now
returns an actionable generation/context-limit message before parsing. Regression
failed first; 156 full Rust tests, thirteen focused tests, Clippy and formatting
passed. Two live limit errors were followed by two correct near-limit answers on
the same engine. The desktop package has not yet been rebuilt for this correction.
Other hardware, Windows and enterprise-quality acceptance remain incomplete.


The generation-limit correction subsequently passed its
[native package check](../evaluation/context/length-limit.md#packaged-native-follow-up):
rebuilt release, exact message with four results retained, correct short recovery
answer on the same engine PID, and zero bundle executables after quit. The test
profile has 15 documents, 74 chunks and 60 embeddings. Fourteen synthetic text
passages remain unembedded. This closes native verification of the message;
Windows, target-device resources and representative enterprise acceptance remain
incomplete.


## Detailed maximum-pixel image

[Native image import](../evaluation/resources/complex-image/README.md) accepted a
20MP grayscale noise PNG (20,010,168 bytes), storing a 512 × 410 preview and one
identical media part. The collector captured 706 samples, with no decoder sample;
normal quit left the coalition empty. A separate direct normalization measurement
reported 80,248,832-byte maximum RSS and 62,702,976-byte peak footprint, with an
output hash matching the native preview. It does not establish the simultaneous
whole-app decoding peak. This extends maximum-pixel evidence; maximum-byte video,
early launch, consumer hardware, Windows and enterprise-quality gates remain open.
