# Packaged verification of corrected media answers

Date: 2026-10-10. This is a manual CUA accessibility check of the rebuilt
`com.cachella.mediacheck` application on Apple M3 Pro with 36 GiB RAM.
The isolated profile retains one synthetic video and its two indexed segments.
No original user library was used. Observations in the JSON report are manually
transcribed from CUA, not an automated test transcript.

## Build

```sh
npx tauri build --debug --bundles app --features desktop \
  --config src-tauri/tauri.engine.conf.json \
  --config src-tauri/tauri.decoder.conf.json \
  --config '{"identifier":"com.cachella.mediacheck","productName":"Cachella Media Check"}'
```

The frontend TypeScript/Vite build and desktop build succeeded. The final Rust
build line reports 15.50 seconds and the bundle is 145.09 MiB.
[Build output](native-normalized-build.txt) and
[artifact fingerprints](native-normalized-artifact.json) identify the package.
No release was signed or published.

## Native observations

1. Relaunch retained the Gemma generation model/projector, EmbeddingGemma pair,
   included decoder, one document and two passages.
2. Embedding startup completed and displayed `2 of 2 passages indexed`.
   An attempted generation start during embedding startup was rejected with
   `Wait for the current model operation to finish.` Generation started after
   embedding readiness. Startup timing was not measured as a benchmark.
3. Semantic search for `What does the video show, and what monthly cloud budget
   does it mention?` ranked the video audio first and frame second, each with
   0–3,042 ms metadata.
4. Generate answer returned `The video shows a red circle. The project cloud
   budget is $8 per month.` with two citation buttons. Displayed metrics:
   3.8 seconds, 460 input tokens, zero cached tokens.
5. The second citation opened the same video's local preview and showed
   `Matched interval: 0:00–0:03`. Playback itself was verified in the earlier
   [native check](native-ui.md) and was not repeated here.
6. Repeating Generate answer returned the identical complete answer and two
   citations: 3.2 seconds, 460 input tokens, 455 cached tokens. This verifies the
   displayed llama.cpp reuse counter. Two uncontrolled requests do not establish
   a general latency or cost-saving estimate.
7. Normal quit left zero processes with executables under the test app bundle.
   Read-only SQLite checks retained one document, two chunks, two embeddings,
   one asset and two media parts.

[Result and shutdown evidence](native-normalized-result.json).
The reproduced native omission is corrected for this fixture. The two-fixture
backend matrix and its quality limits remain documented in
[normalization.md](normalization.md).

## Scope

No executable source changed during this continuation, so test-first is exempt
for its documentation. The preceding 139-test Rust suite and Clippy checks were
not repeated. The new checks were package build, native interaction, persisted
row counts, scoped shutdown and report link/fingerprint validation.

Still unfinished: broader and adversarial answer quality, temporal video
reasoning, whole-app resource/latency qualification on target consumer hardware,
native Windows execution and installer qualification, release signing and final
acceptance. The test package and profile remain on disk, with the app stopped.
