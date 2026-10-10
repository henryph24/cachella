# Optional one-model residency

Date: 2026-10-10. The default remains two managed models. The new persisted
`single` mode stops the previous managed engine before creating the next child.
It controls managed engine residency and provides no byte-level RAM guarantee.
External loopback servers remain independently managed.

## User behavior

Stop both models to change the policy in Local runtime. In one-model mode, start
each role once per application session. A successful explicit Start enables
on-demand resumption for that role. Search and indexing resume embeddings;
answering retrieves evidence and then resumes generation. A suspended role shows
On demand, with Stop available to disable further automatic starts. Explicit Stop,
changing that role's files, and switching generation to an external endpoint
clear its resume intent. Restart retains the policy and clears session intents.

Switches preserve the SQLite library and embedding rows. Embeddings are rehashed
and authenticated before reuse. Each stop discards that engine's in-memory prompt
cache. Repeated semantic questions therefore pay startup and hashing costs; this
tradeoff needs whole-app measurements on target hardware.

## Implementation and checks

The operation permit covers semantic retrieval, stopping/starting engines and
generation. Startup cleanup owns a connection identified by endpoint and token;
cancellation drops that cleanup before releasing the operation permit. Failed
switches leave the role eligible for retry without restoring the previous engine.
Policy publication uses the existing atomic settings helper and commits before
updating the in-memory mode or resume flags. Read-only index/status polling never
starts engines.

Test-first regression: `single_residency_stops_previous_owned_engine_before_next_start`
failed with `one-model mode kept the previous engine resident`; it passed after
the ordered stop/start change. Tests cover both switch directions, policy-save
failure/busy/closing states, explicit stop/external selection, cancelled automatic
switches through OperationJobs, and startup failure with retry admission.

The full Rust suite passed 148 tests with zero failures in 2.48 seconds. [Raw output](rust-tests.txt).
All-target desktop Clippy with warnings denied passed in 1.88 seconds. An initial
full run had 147 passes and a decoder fixture marker timeout; the isolated test
passed in 1.14 seconds. Its existing readiness-marker wait now has a five-second
bound to accommodate concurrent guarded-process starts; production decoder code
is unchanged.

Two independent review rounds covered Rust and frontend wiring. The Rust review
found a cancelled-picker status response losing the resume flag; that branch now
uses the desktop status helper. Frontend review found policy-load error handling
and an abandoned-effect ordering issue. Failed or invalid loads now show
Unavailable with a disabled control; each load owns its effect-lifetime guard.
Final frontend suite: 67 passed. TypeScript/Vite production build passed. The
final isolated desktop build succeeded (3.99 s Rust, 145.15 MiB app).
[Build log](../media-answers/native-residency-build.txt) and
[artifact fingerprints](artifacts.json) identify the tested build.

## Native observations

The isolated Mac app retained the policy, Gemma generation pair and EmbeddingGemma
pair. Explicitly starting embeddings then generation showed generation Running
and embeddings On demand. Index library automatically resumed embeddings and
suspended generation; it retained 2 of 2 indexed passages. This index call had no
new passages to embed; startup still executed its authenticated compatibility
query. Semantic search returned audio then frame from the synthetic video.
Generate answer switched to generation and returned red circle and $8/month with
two citations, 460 input tokens and zero cached tokens. The displayed 3.5 seconds
covers the inference request and excludes preceding model-switch/startup work.
It is not an end-to-end latency benchmark.

[Native observations](native-observations.json) are manually transcribed from CUA.
[Process samples](native-process-samples.json) contain 590 observations at a target
250 ms interval: maximum one managed engine, final zero bundle executables.
Sampling alone cannot exclude shorter overlap; the ordered stop-before-start code
and both-direction lifecycle tests establish the residency invariant. SQLite
retained 1 document, 2 chunks, 2 embeddings, 1 asset and 2 media parts.

Explicit Stop on suspended embeddings disabled its Stop and Index actions; the
next semantic search failed without restarting it. That exposed an existing
generation-specific error in the shared runtime connection helper. A new failing
regression reproduced it, and embedding readiness now reports which model to
start. The final focused desktop suite passed 20 tests, zero failures, 1.93 s
([output](final-desktop-tests.txt)); final all-target Clippy passed in 10.71 s.
The full suite was not repeated for this focused message-path correction.
The final app rebuilt in 15.61 s for Rust ([log](final-build.txt)). Restart showed
one-model policy retained, both model selections retained, both engines Stopped,
and Stop/Index actions disabled until an explicit Start.

The final native run also cancelled Index library while automatic embedding
resumption was verifying files: controls returned to idle, the 2/2 index remained,
generation stayed loaded and embeddings stayed On demand. Explicit Stop then
disarmed embeddings; semantic search displayed `Start the embedding model in
Local runtime first.` Normal quit left zero bundle executables and the same row
counts. This native cancellation covered the pre-spawn phase; backend cancellation
tests cover a newly spawned child. [Final fingerprints](final-artifacts.json).

Documentation-only changes were exempt from test-first. The HTML relative link
and all seven report evidence files were checked for existence; formatting passed.

## Outstanding qualification

 Whole-app memory/latency on
consumer targets, Windows execution/installer qualification and final acceptance
remain unfinished. No cloud resources were provisioned and no release was published.
