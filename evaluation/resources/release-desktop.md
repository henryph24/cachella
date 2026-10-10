# Release desktop process accounting

Date: 2026-10-10. Apple M3 Pro, 36 GiB RAM. One optimized macOS desktop build,
three read-only sampling runs, isolated `com.cachella.mediacheck` profile.
The original application profile was not used. No product executable source changed.

## Result

The matched synthetic answer workload showed a sampled process-footprint peak of
1.165 GiB in one-model mode and 2.071 GiB with both engines loaded. This includes
the application, attributed WebKit/Metal helpers, guardians and engines. The
observed difference is 43.7% for these two phases in one ordered run. Different
background activity, cache state, workloads and hardware can change the result.
The displayed inference times were 3.5 and 3.7 seconds, respectively; they exclude
preceding startup/switch work and do not establish a latency advantage.

| Phase | Valid samples | Peak resident sum (GiB) | Peak physical footprint sum (GiB) |
| --- | ---: | ---: | ---: |
| UI idle after launch | 36 | 0.250 | 0.418 |
| Generation startup | 113 | 1.440 | 1.279 |
| Switch to embeddings | 87 | 1.442 | 1.191 |
| One-model semantic search | 54 | 1.151 | 1.147 |
| One-model answer and switch | 88 | 1.224 | 1.165 |
| Load embeddings beside generation | 139 | 2.486 | 1.973 |
| Both-model semantic search | 35 | 2.024 | 1.929 |
| Both-model answer | 58 | 2.143 | 2.071 |
| Both-model video picker/import | 93 | 1.506 | 2.229 |
| Stop/restore policy/quit | 60 | 1.147 | 2.241 |
| Separate 60-second video import, engines stopped | 435 | 0.537 | 0.552 |

The two peak columns can occur at different instants. Each column is the maximum
of a simultaneous sum across the sampled processes, so per-process lifetime peaks
are never added together. Footprint can exceed resident bytes because these are
different macOS accounting measures. The overall coarse-run peaks were 2.486 GiB
resident and 2.241 GiB footprint. The latter occurred during cleanup with media
picker helpers still present. It must remain in the reported application envelope.

## Workloads and evidence

1. Start generation, then embeddings in one-model mode. Search for `What does the
   video show, and what monthly cloud budget does it mention?` and generate an answer.
2. Stop generation, select Both, start both roles and repeat the same search/answer.
3. Reimport the same three-second video through the native picker. The library
   reuses its existing content. Restore Single after stopping both roles, then quit.
4. A finer-sampling run repeats the three-second import with no engines loaded.
5. A third run imports a 60.091641-second, 731,537-byte loop of the same video with
   engines stopped. It adds one synthetic document with 20 passages. Quit normally.

Both answer requests returned `The video shows a red circle. The project cloud
budget is $8 per month.`, two citations, 460 input tokens and zero cached tokens.
The initial library held one video with two passages and two vectors. The final
isolated profile holds two documents, 22 passages, two vectors, two assets and 22
media parts. The new 20 passages have not been indexed. The original application
profile remains separate. Final saved policy is Single and all measured coalitions
became empty. No release was published or cloud resources created.

| Run | Samples | Valid nonempty samples | Median observed interval | Final members |
| --- | ---: | ---: | ---: | ---: |
| [Modes and picker](m3-pro-release-desktop.json) | 899 | 897 | 327 ms | 0 |
| [Short decoder retry](m3-pro-release-desktop-decoder.json) | 511 | 505 | 41 ms | 0 |
| [Longer decoder workload](m3-pro-release-desktop-decoder60.json) | 755 | 735 | 41 ms | 0 |

The first two runs captured no FFmpeg process despite successful imports. The
longer workload captured FFmpeg in six valid samples. Neither short run is evidence
of the decoder's own peak. The longer run still may miss FFprobe or brief allocation
peaks. Requested intervals were 250 ms and 10 ms; discovery and sampling overhead
made actual intervals longer. These runs are sampling evidence, not a hard memory cap.

The coarse run excludes one sample with two exiting-process attribution errors,
plus its final empty sample. The short retry excludes five error samples and its
empty sample; the long run excludes 19 error samples and its empty sample. Discovery
also observes ESRCH when a same-user process exits between listing and inspection
(including `ps` itself). No other discovery-error code occurred. Raw reports retain
errors; incomplete process-accounting samples do not enter the peak table.

## Method and limits

The app was launched separately through CUA/Launch Services. The collector verifies
its exact executable path and rejects the collector's own coalition. It selects
same-user processes sharing both resource and jetsam coalition IDs, then includes
their descendants. It rechecks each member's coalition after accounting to reject
observed PID/coalition changes. This is a non-atomic observation; it does not prove
immunity to every process-reuse race. The sampled paths were inspected and comprised
the app, engine/guardian, WebKit content/network/GPU, Metal compiler, theme, native
file picker, QuickLook, audio and VideoToolbox services, plus FFmpeg in the long run.

The evaluation-only coalition ABI comes from Apple's
[proc_info_private.h](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/sys/proc_info_private.h)
and [mach/coalition.h](https://github.com/apple-oss-distributions/xnu/blob/main/osfmk/mach/coalition.h).
The public SDK's `proc_pid_rusage` with `RUSAGE_INFO_V4` supplies resident, footprint
and lifetime peak fields. The private coalition interface is checked for exact
structure size and nonzero IDs; it is not a cross-version compatibility promise.
The collector performs no app launch, process termination or UI mutation.

Early app launch before attachment, system-wide WindowServer/driver/OS allocations,
other users, electrical power and battery use are excluded. Metal/WebKit process
accounting is included where attributed to this coalition; it is not a complete
measurement of physical unified-memory consumption across the machine. Resident
sums can double count shared pages. Clean mapped weights, compression and file-cache
state complicate conversion to a minimum machine-RAM requirement. Background load
and filesystem caches were uncontrolled. No 8 GB, 16 GB or Windows qualification is
inferred. Larger libraries, maximum context, 32 MB/two-minute media inputs and
representative enterprise documents remain unqualified.

## Reproduction and checks

[Release build output](release-desktop-build.txt): Rust optimized build 51.51 s,
one 114.85 MiB app bundle. [Artifact hashes](release-desktop-artifacts.json) and
per-run sampler hashes identify the measured binaries. [Fixture provenance and
manual UI observations](release-desktop-fixtures.json) record the loop and answers. Models use the existing
[pinned Gemma/EmbeddingGemma configuration](gemma-media.md).

Compile the accounting helper and attach after launching the isolated app:

```sh
clang -Wall -Wextra -Werror evaluation/resources/memory_sample.c -o .local/media-memory-sample-checked
python3 evaluation/resources/desktop_measure.py --pid PID \
  --executable '/absolute/path/Cachella Media Check.app/Contents/MacOS/cachella' \
  --sampler .local/media-memory-sample-checked --control /tmp/phase.json \
  --output /tmp/desktop-memory.json --interval 0.25
```

The control file is JSON such as `{"phase":"idle_ui"}`. Publish phase changes by
atomic file replacement; normal application exit ends collection once its cohort
is empty. `{"phase":"finished","stop":true}` ends read-only observation without
terminating the application. Explicit manual stop or a duration limit can leave
members running, which is represented by `stop_reason` and `final_member_count`.

Test-first collector tests initially failed because the new module was absent.
The implemented selection tests cover reparented XPC, descendants, unrelated
coalitions, snapshot PID reuse, invalid IDs and attribution changes. Existing
aggregation/process-cleanup tests were reused: `python3 -m unittest discover -s
evaluation/resources -p 'test*measure.py'` reports eleven tests passed. Python
compilation/help and C compilation with warnings denied passed. An independent
review confirmed the ABI and identified isolation/PID-race limits; the collector
now rejects shared launch coalitions and rechecks attribution after sampling.
Product Rust/frontend suites were not repeated because product behavior is unchanged.
Documentation is exempt from test-first; linked files and raw numeric totals were
validated. Representative consumer/Windows qualification and final acceptance
remain unfinished.
