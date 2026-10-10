# Detailed maximum-pixel image

The isolated release desktop accepted a 5,000 × 4,000 grayscale PNG containing
seeded random pixels: 20,000,000 pixels and 20,010,168 encoded bytes. This extends
the earlier solid-color pixel-boundary test with incompressible image data.
It exercises the maximum pixel count with an 8-bit grayscale image; maximum
encoded bytes, RGB/RGBA/color-depth combinations and video are separate cases.
[Fixture metadata](fixture.json).

The fixture was generated using Python's `random.Random(20000000).randbytes(5000)`
for each of 4,000 rows, prefixed by PNG filter byte zero. A default zlib compressor
processed the scanlines; the existing `boundary_fixtures.chunk` helper wrote PNG
IHDR/IDAT/IEND chunks with CRCs. IHDR uses bit depth 8, color type 0, and no interlace.
The file hash in the metadata identifies the exact tested bytes.

## Native import and preview

Native file-picker import reported one file processed. The source preview opened.
Read-only SQLite inspection found a 512 × 410 PNG preview, 143,181 bytes, and one
identical image part. [Stored evidence](stored-preview.json),
[native observations](native-observations.json). Final isolated-profile counts:
16 documents, 75 passages and 60 embeddings. The new noise image is unembedded;
this is an import/decoder check. No inference engines were started.

The existing coalition collector attached after launch with a 10 ms target. It
recorded 706 samples over 29.509 s; 702 samples contributed to phase totals.
Actual median spacing was 41 ms. Import phase includes source-preview opening
and UI idle time. [Raw samples and collector exclusions](native-resources.json).

| Phase | Valid samples | Sampled peak resident sum, bytes | Sampled peak footprint sum, bytes |
| --- | ---: | ---: | ---: |
| file_picker | 284 | 488,390,656 | 482,933,824 |
| image_import | 361 | 510,869,504 | 600,013,952 |
| shutdown | 57 | 496,648,192 | 286,014,912 |

Neither FFmpeg nor FFprobe appeared in the sampled membership. Consequently,
these totals do not establish the combined peak during decoding. Normal quit
ended with an empty coalition; a separate `ps -axo pid=,ppid=,comm=` inspection
found zero matching bundle executables.

## Separate decoder measurement

To address the missed decoder observation, the bundled FFmpeg normalization
command was executed directly under `/usr/bin/time -l`, using the production
image arguments, two-thread limits, scale filter and output cap.
[Exact argument array](direct-command.json), [time output](direct-time.txt).
It exited zero in displayed 0.02 s, with maximum resident set size 80,248,832 bytes
and peak memory footprint 62,702,976 bytes. Its output SHA-256 exactly matches the
native stored preview. This separate component run has a warmed input file and
excludes the app, scratch-copy/probe work and UI. Its peak cannot be added to the
sampled desktop peaks to claim a simultaneous total.

## Verification and limits

No executable changes were made; fixture/documentation work is exempt from
test-first. Native import/source opening, SQLite content/dimensions, output hash,
process cleanup, artifact hashes and report links were checked.
[Artifacts](artifacts.json). Existing production test results remain applicable.

This is one synthetic image on the 36 GiB M3 Pro. Early launch, brief unsampled
process peaks, system-wide allocations, hardware pressure, complex maximum-byte
video, Windows and consumer-device qualification remain open. This check makes
no enterprise retrieval-quality or minimum-RAM claim.
