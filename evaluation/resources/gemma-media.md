# Gemma media and embedding resource measurement

Date: 2026-10-10. One run on Apple M3 Pro, 36 GiB physical RAM, using the
release-mode resource probe and bundled llama.cpp engine. The raw report
[m3-pro-gemma-media.json](m3-pro-gemma-media.json) records OS/hardware metadata,
model and executable hashes, every response, phase events and process samples.
This is component measurement. Desktop WebKit, decoder, OS and unrelated apps
are excluded from the reported process sums.

## Workload

The existing probe now accepts an optional final generation projector. It loads
Gemma 4 E2B Q4_0 with its Q8_0 projector and EmbeddingGemma 2 Q8_0 with its own
projector. Model files occupy 4,263,525,728 bytes in total. Runtime allocation is
additional. Generation retains the production 4,096-token context and one slot;
embedding retains 2,048 tokens and one slot. Both runtimes are independently
started and authenticated through production code.

The run completed 24 text-answer requests, 12 media-answer requests and 18 measured
embedding requests. One additional embedding compatibility query occurs during
startup. Text answers use one short source or four repeated-fact sources. Media
answers use a red-circle PNG, a three-second spoken budget, and both together.
Each media case runs once with caching disabled and once enabled in each scenario.
The media code uses bounded file reads and the production media request builder.
Prepared PNG/WAV files bypass media decoding and SQLite retrieval in this probe.

Generation alone, embeddings alone and both engines resident are distinct phases.
Requests remain serialized, matching the desktop operation guard. The combined
phase includes five processes: probe, two guardians and two engines. Filesystem
caches and background workload were uncontrolled; no cold-boot result is claimed.

## Observations

The run lasted 63.581 seconds and collected 218 samples with no accounting errors.
All owned processes stopped and both managed listeners closed.

| Active workload | Sampled resident sum peak | Sampled physical-footprint sum peak |
| --- | ---: | ---: |
| Generation, text | 1,222,983,680 bytes | 879,683,200 bytes |
| Generation, media | 1,304,559,616 bytes | 934,095,040 bytes |
| Embeddings | 1,107,148,800 bytes | 876,044,480 bytes |
| Both engines, text/embedding workload | 2,352,578,560 bytes | 1,692,647,488 bytes |
| Both engines, media generation | 2,414,460,928 bytes | 1,698,055,232 bytes |

The last row is approximately 2.25 GiB resident and 1.58 GiB footprint. These are
sampled simultaneous sums, not sums of independent lifetime peaks. Resident sums
can count shared pages more than once; physical footprint can omit clean mapped
pages. Sampling can miss brief peaks. The accounting limits in
[the resource-method report](README.md) apply. These values cannot establish a
whole-device RAM requirement or qualify an 8 GB/16 GB configuration.

Generation startup took 1,146 ms alone and 1,138 ms with embeddings resident.
Embedding startup took 1,057 ms, with model-content hashing separately taking
1,923 ms. Stable boundaries include 600 ms observation waits, excluded from
request/startup durations. All timings are specific to this run.

For both engines resident, the text results below are medians of three identical
requests per cache condition. Media rows contain one request per condition.

| Answer workload | Prompt tokens | Cache disabled | Cache enabled | Reused tokens |
| --- | ---: | ---: | ---: | ---: |
| Short text | 145 | 1,214 ms | 1,044 ms | 140 |
| Longer text | 1,435 | 2,383 ms | 1,102 ms | 1,434 |
| Image | 214 | 1,229 ms | 854 ms | 209 |
| Audio | 205 | 1,077 ms | 800 ms | 200 |
| Image and audio | 342 | 1,687 ms | 1,154 ms | 337 |

Every cache-enabled request follows its identical cache-disabled request.
Disabled requests report zero cached tokens. These observations demonstrate local
llama.cpp prompt reuse on repeated evidence; they measure full-response latency,
not time to first token or hosting-cost reduction. LMCache was absent.

Manual review of all 36 answer results found the expected facts and supporting
source IDs: permission bug and Priya for text, red circle for image, $8/month for
audio, both facts for combined media. This is four distinct synthetic questions
with repeated conditions. It does not establish enterprise answer quality or
full-context stability. The longer text prompt used 1,435 tokens of the 4,096
context limit.

## Reproduction and checks

```sh
cc -Wall -Wextra -Werror -O2 evaluation/resources/memory_sample.c -o .local/media-memory-sample
cargo build --release --manifest-path src-tauri/Cargo.toml --example resource_probe
python3 evaluation/resources/measure.py \
  --sampler .local/media-memory-sample \
  --output /tmp/cachella-gemma-media-repeat.json -- \
  src-tauri/target/release/examples/resource_probe \
  src-tauri/resources/engine/llama-server \
  .local/models/gemma-4-E2B-it-Q4_0.gguf \
  src-tauri/resources/engine/llama-server \
  .local/models/embeddinggemma-2-Q8_0.gguf \
  .local/models/mmproj-embeddinggemma-2-Q8_0.gguf \
  evaluation/multimodal/fixtures \
  .local/models/mmproj-gemma-4-E2B-it-Q8_0.gguf
```

This evaluation-only extension uses exploratory test-after verification; product
behavior is unchanged. The original text-only invocation remains supported.

- Release build passed in 16.22 seconds.
- Example Clippy with `-D warnings` passed in 4.83 seconds; formatting passed.
- `python3 -m unittest discover -s evaluation/resources -p 'test_*.py' -v`:
  five aggregation/cleanup tests passed in 0.639 seconds.
- C helper compiled with warnings treated as errors. Python compilation and CLI
  help passed.
- Real run reported `PASS: 218 memory samples; owned process group empty`.
- Report validation confirmed 24 text, 12 media and 18 embedding request events,
  three successful startup events and no accounting errors.

No full application build or UI tests were repeated because only evaluation code
changed. Outstanding: whole-app/Metal allocation measurements on target hardware,
longer/context-limit workloads, Windows runtime/installer execution, battery/power
and representative quality qualification. This run supports retaining separate
local runtimes and built-in caching for the demo; it supplies no evidence that
LMCache is needed on an individual machine.
