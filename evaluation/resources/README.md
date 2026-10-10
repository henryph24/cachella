# Local resource measurements

This probe measures the native Cachella runtime and inference paths on macOS. It starts its own authenticated model processes, records memory while they serve a fixed synthetic workload, verifies shutdown and writes a JSON report. The desktop application is unchanged by this probe.

## Reproduce

Use the pinned model artifacts in [generation provenance](../model-provenance.json) and [embedding provenance](../multimodal/provenance.json). The Q4 candidate and the embedding projector require separate model files. Run from the repository root:

```sh
clang -Wall -Wextra -Werror -O2 evaluation/resources/memory_sample.c -o .local/memory_sample
cargo build --release --manifest-path src-tauri/Cargo.toml --example resource_probe
python3 evaluation/resources/measure.py --sampler .local/memory_sample --output evaluation/resources/results.json -- src-tauri/target/release/examples/resource_probe /path/to/generation/llama-server /path/to/Qwen3-1.7B-Q4_K_M.gguf /path/to/embedding/llama-server /path/to/embeddinggemma-2-Q8_0.gguf /path/to/mmproj-embeddinggemma-2-Q8_0.gguf evaluation/multimodal/fixtures
python3 -m unittest discover -s evaluation/resources -p 'test_*.py' -v
```

The probe also accepts an optional final generation-projector path. With this
argument, the generation runtime loads that projector and runs six additional
image/audio/combined-media requests in each generation scenario (12 total).
Each of the three media questions runs once with reuse disabled and once enabled.
The collector fingerprints the additional projector and records
`media_generation_enabled: true`. The original six-argument workload remains
supported. Media answers use the production request builder, including modality
labels and audio-last presentation. This mode still excludes the desktop UI and
decoder; its PNG/WAV fixtures are already prepared.

The collector requires macOS and the Apple command-line developer tools. Its C helper reads `proc_pid_rusage` using the installed SDK's `rusage_info_v4` structure. The collector creates a private process group for the probe and samples only that group. Timeouts and collector failures terminate that group and reap the direct child. Existing user model services are outside this group.

## Workload and accounting

- Production generation configuration: one slot, 4,096-token context, a maximum of 384 generated tokens and the application's structured grounded-answer request.
- Generation: one short source and four 1,800-character sources containing repeated synthetic facts. Each size runs three cache-disabled/cache-enabled pairs in each of two scenarios, totaling 24 requests. Repetition deliberately measures identical-prefix reuse. Evidence size does not fill every possible 4,096-token context.
- Embeddings: production 2,048-token context/batch/microbatch, one slot, text query plus one PNG and one three-second WAV, three repetitions in each scenario, totaling 18 measured requests. Startup additionally performs its compatibility query.
- Scenarios: generation alone, embedding alone, both engines resident with serialized requests. Serialization follows the desktop operation guard.
- Sampling target: 250 ms plus collection overhead. Stable phase boundaries wait 600 ms to provide sampling opportunities; those waits are excluded from request and engine-startup durations. The report stores sample timestamps and process-level readings.
- Reported phase peaks are the maximum simultaneous sum of process readings. Per-process lifetime footprint high-water marks are retained in raw samples. Summing independent per-process lifetime peaks would not establish a simultaneous peak.
- Resident sums can count shared pages more than once. Physical footprint is macOS process accounting. Clean file-backed mappings can be excluded from that footprint, while compressed dirty memory is charged at its original size; [Apple explains these accounting categories](https://developer.apple.com/documentation/xcode/analyzing-the-memory-usage-of-your-metal-app). A footprint below the GGUF file size therefore does not establish the complete model working set. These figures include the native probe and its engines; desktop WebKit, decoder activity, operating-system memory and other apps are excluded. They cannot establish a whole-device minimum RAM requirement.
- Model-content hashing is timed separately. Engine startup includes authentication/readiness and the embedding compatibility check. Filesystem caches and background workload are uncontrolled; these are process-start observations.

The harness validates response structure and source membership through production code. It records answer content and abstentions for inspection. The workload does not qualify general answer quality, full-context stability, Windows performance or battery use.

## Verification

The aggregation tests distinguish simultaneous sums from independent maxima and reject incomplete process samples. Cleanup tests cover a timed-out probe with a child and a failing process-discovery command. A direct sampler check observed a touched 64 MiB allocation and a corresponding lifetime high-water mark. The C helper compiles with warnings treated as errors; Rust example checks use Clippy with warnings denied.

## Observed results: October 9, 2026

Apple M3 Pro with 36 GiB RAM. Two runs used the same release-mode probe and artifact hashes. The first run produced 221 memory samples in 66.2 seconds; the repeat produced 127 samples in 38.0 seconds. Together they completed 48 generations and 36 measured embedding requests. All generations returned cited, non-abstaining answers identifying the permission bug and Priya. This is a single synthetic question across two evidence sizes.

| Active workload | Run 1 sampled footprint peak | Run 2 sampled footprint peak |
|---|---:|---:|
| Qwen3-1.7B Q4_K_M generation | 0.646 GiB | 0.645 GiB |
| EmbeddingGemma 2 Q8 text/image/audio | 0.829 GiB | 0.900 GiB |
| Both engines loaded, serialized requests | 1.507 GiB | 1.488 GiB |

These simultaneous sums include the probe. They measure the named macOS footprint field and exclude clean mapped pages from that metric; use the accounting limits above when interpreting them. The two-file embedding model/projector occupies 864,676,480 bytes on disk; the Q4 generation model occupies 1,107,409,472 bytes. The observed footprint is insufficient evidence for an 8 GB hardware guarantee.

Full-response latency below is the median of six requests per cell across the two runs. Every cache-enabled request immediately follows an otherwise identical cache-disabled request. The context limit remains 4,096 tokens; the observed prompts contain 138 or 1,365 tokens.

| Loaded engines / prompt | Reuse disabled | Reuse enabled | Reported reused prompt tokens |
|---|---:|---:|---:|
| Generation only / 138 tokens | 714.5 ms | 616.5 ms | 137 / 138 |
| Generation only / 1,365 tokens | 1,624.5 ms | 763.0 ms | 1,364 / 1,365 |
| Both engines / 138 tokens | 723.5 ms | 633.5 ms | 137 / 138 |
| Both engines / 1,365 tokens | 1,783.5 ms | 796.5 ms | 1,364 / 1,365 |

Generation emitted 46 tokens for short evidence and 49 for the larger evidence. All reuse-disabled requests reported zero cached tokens. The larger generation-only workload shows a 53.0% reduction in median full-response latency for this identical-prefix sequence. This is local llama.cpp reuse. LMCache was absent, and no hosting-cost reduction was measured.

Initial generation startup took 834 and 831 ms. Embedding startup took 1,564 and 1,780 ms, with model-content hashing measured separately at 2,111 and 2,368 ms. Starting generation while the embedding engine remained loaded took 28,919 ms in the first run and 2,164 ms in the repeat. The outlier triggered the repeat; its cause is unresolved. Both values remain in the evidence.

Two repeat-run samples saw already-exiting engine processes return macOS error 3 (ESRCH), affecting three process readings at shutdown boundaries. The summaries exclude those incomplete samples. Active-workload samples contained no accounting errors. Both runs verified closed model listeners and an empty owned process group after completion.

### Product decision

Keep Qwen3-1.7B Q4_K_M as the prototype generation candidate and retain llama.cpp prefix reuse. Keep embedding and generation as separately controlled local processes. The intended 16 GB Windows baseline remains a design target awaiting execution; 8 GB support requires whole-app measurements on an actual constrained device. Avoid an advertised minimum based on these macOS component measurements. Whole-app/Metal allocation profiling, maximum-context and longer-media loads, other consumer CPUs/GPUs and representative answer-quality checks remain required before a hardware support claim.

Evidence: [first run](m3-pro-q4.json), [repeat run](m3-pro-q4-repeat.json), [generation quality assessment](../assessment.md). JSON reports retain request results, process-level samples, SDK lifetime high-water marks and artifact/fixture hashes.

## Gemma media generation extension

[Gemma media measurement](gemma-media.md) records the optional-projector run:
24 text answers, 12 media answers, 18 embedding requests and 218 memory samples.
Both engines resident reached sampled peaks of 2.25 GiB resident and 1.58 GiB
physical footprint under this synthetic workload. Whole-app and target-device
qualification remain separate gates.
