# Maximum-duration video import on the development Mac

The packaged app accepted an exactly 120-second video as 36 media passages and
rejected a 121-second video with `Audio and video are limited to 120 seconds.`
The rejected import added no document. Both checks used the production decoder
and the existing native import flow in the isolated `com.cachella.mediacheck`
profile. No product code changed.

## Fixtures and reproducibility

Both fixtures repeat the existing synthetic `budget-video.mp4` (red circle and
spoken budget). They are 384 × 384 pixels. The accepted file is 1,281,132 bytes,
well below the 32 MiB file limit. This is a duration-boundary check, with no
maximum-resolution or maximum-file-size qualification.

The bundled decoder omits the `lavfi` input device. An initial generated-source
command failed with `Unknown input format: lavfi`; fixture creation then reused
the existing media file. The successful commands use the bundled FFmpeg:

```sh
src-tauri/resources/decoder/ffmpeg -hide_banner -loglevel error -stream_loop 40 -i evaluation/multimodal/fixtures/budget-video.mp4 -t 120 -c:v libx264 -preset ultrafast -pix_fmt yuv420p -c:a aac -b:a 64k evaluation/resources/fixtures/limit120.mp4
src-tauri/resources/decoder/ffmpeg -hide_banner -loglevel error -stream_loop 40 -i evaluation/multimodal/fixtures/budget-video.mp4 -t 121 -c:v libx264 -preset ultrafast -pix_fmt yuv420p -c:a aac -b:a 64k evaluation/resources/fixtures/over121.mp4
```

[Probe output and fixture hashes](duration-fixtures.json) establish the durations.
`cargo build --release --manifest-path src-tauri/Cargo.toml --example decode_smoke`
passed in 18.00 s. The existing production-path probe returned
`PASS: limit120.mp4 -> video/mp4 preview, 36 indexed parts` (exit 0), and the
121-second probe returned the duration error (exit 1).
[Build](duration-probe-build.txt), [accepted](duration120-decode.txt),
[rejected](duration121-decode.txt).

## Native process accounting

The unchanged release artifact from the
[hash-cancellation package check](../residency/hash-cancellation/release-artifacts.json)
ran with both models stopped. The existing read-only coalition collector sampled
the app and attributed helpers, including the native file picker and decoder.
Phase boundaries were set manually before confirming the file selection;
phase duration includes UI idle time and is not an import-latency measurement.

| Phase | Valid samples | Peak resident sum (GiB) | Peak footprint sum (GiB) |
| --- | ---: | ---: | ---: |
| idle_before_import | 311 | 0.454 | 0.438 |
| import_120_seconds | 353 | 0.536 | 0.475 |
| reject_121_seconds | 593 | 0.483 | 0.677 |

The run recorded 1,294 samples over 54.471 s, with 1,257 valid nonempty samples.
Twelve valid samples include FFmpeg; none captured FFprobe. Target sampling was
10 ms, actual median spacing 42 ms. The rejection phase includes native picker,
preview and shutdown activity, so its larger peak cannot be assigned to rejection
logic alone. Normal quit ended with an empty coalition.
[Raw measurement](m3-pro-release-duration120.json),
[native observations and SQLite counts](duration-native-observations.json).

The isolated profile now contains three documents, 58 chunks, 22 embeddings,
three assets and 58 media parts. The new 36 passages are unindexed. The original
user profile remains untouched.

## Verification scope and remaining work

These are evaluation artifacts with no executable changes; test-first is exempt.
The existing release decoder probe, actual native import/rejection, read-only
row counts, raw measurement totals, local links and fixture hashes were checked.
Product test suites were not rerun because product code was unchanged.

Process accounting excludes early application launch, system-wide OS/driver
allocations and other users. Resident sums can double-count shared pages; sampled
peaks can miss short-lived processes. A 36 GiB M3 Pro establishes no 8/16 GiB or
Windows compatibility. Maximum-size/resolution inputs, indexing all 36 new parts,
model context pressure and representative enterprise content remain unqualified.
