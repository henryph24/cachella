# Media byte and geometry boundaries

Ten production-decoder cases now meet their expected outcomes: seven accepted,
three rejected. The initial run found an allowed 8,192 × 1 PNG failed because
FFmpeg's aspect-ratio reduction rounded its smaller dimension to zero. The same
failure affected portrait images and thin videos. An odd-sized 513 × 257 video
also failed because the normalized yuv420p encoder requires even dimensions.

## Change and execution path

`media::prepare` validates input, copies it to isolated scratch storage, probes
its dimensions, then calls `prepare_image` or `prepare_video`. Both image previews
and sampled video frames now share `IMAGE_SCALE`, which clamps each final scaled
dimension to at least one pixel and at most 512. `VIDEO_SCALE` clamps final preview
dimensions to positive even values, with a 720-pixel maximum. The existing
normalization validation, duration limits, payload bounds and process cleanup
remain in the same execution path.

The normalization fingerprint changes from `cachella-media-v1` to
`cachella-media-v2`, because stored preview geometry can change. Existing stored
media and vectors remain intact. Reimporting identical source bytes previously
stored under v1 creates a v2 entry; this change performs no automatic migration
or deletion. Earlier packaged-app measurements identify their v1 artifact.

## Boundary evidence

| Input | Expected and observed after fix |
| --- | --- |
| 5,000 × 4,000 PNG (20,000,000 pixels) | Accepted, one image part |
| 5,000 × 4,001 PNG | Rejected by pixel limit |
| 8,192 × 1 PNG | Accepted, 512 × 1 preview |
| 8,193 × 1 PNG | Rejected by side limit |
| 1 × 8,192 PNG | Accepted, 1 × 512 preview |
| 8,192 × 2 video | Accepted, 720 × 2 preview |
| 2 × 8,192 video | Accepted, 2 × 720 preview |
| 513 × 257 video | Accepted, 512 × 256 preview |
| 33,554,432-byte MP4 | Accepted, two media parts |
| 33,554,433-byte MP4 | Rejected by byte limit |

[Initial six-case results](before-fix.json),
[additional four failing cases](additional-before-fix.json),
[final ten-case results](after-fix.json),
[fixture metadata](fixtures.json), [source/tool hashes](artifacts.json).

The MP4 byte fixtures append a valid ISO BMFF `free` atom to the existing
three-second synthetic video. This qualifies the input byte boundary with the
same small encoded content; it does not measure a 32 MiB high-complexity stream.
The PNG fixtures are solid RGB images generated deterministically with PNG CRCs
and zlib-compressed scanlines. These are decoding fixtures with no visual-quality
claim. Runtime fixture generation requires Python's standard library only:

```sh
python3 evaluation/resources/boundary_fixtures.py evaluation/multimodal/fixtures/budget-video.mp4 evaluation/resources/boundaries
src-tauri/resources/decoder/ffmpeg -hide_banner -loglevel error -i evaluation/resources/boundaries/wide-video-input.png -frames:v 1 -c:v libx264 -pix_fmt yuv420p evaluation/resources/boundaries/wide-video.mp4
src-tauri/resources/decoder/ffmpeg -hide_banner -loglevel error -i evaluation/resources/boundaries/tall-video-input.png -frames:v 1 -c:v libx264 -pix_fmt yuv420p evaluation/resources/boundaries/tall-video.mp4
src-tauri/resources/decoder/ffmpeg -hide_banner -loglevel error -i evaluation/resources/boundaries/odd-video-input.png -frames:v 1 -c:v libx264 -pix_fmt yuv444p evaluation/resources/boundaries/odd-video.mp4
```

Use a new output location or remove previously generated test fixtures before
repeating the three FFmpeg commands; the commands do not overwrite files.

## Checks

- Test-first regression failed with `Rescaled dimensions 512x0 are invalid`.
  [Red output](regression-red.txt).
- The five-case real-decoder regression passed after the fix (0.78 s), validating
  preview dimensions and every image part's 1–512-pixel bounds.
  [Green output](regression-green.txt).
- `CACHELLA_FFMPEG=/Users/hungpq2412/cachella/src-tauri/resources/decoder/ffmpeg cargo test --manifest-path src-tauri/Cargo.toml --lib --features desktop`:
  153 passed, zero failed, 3.04 s. [Full output](rust-tests.txt).
  The new decoder regression is conditional on `CACHELLA_FFMPEG`; this run set it
  explicitly. A run without that variable skips the real-decoder regression body.
- All-target desktop Clippy with `-D warnings` passed (3.24 s), Rust formatting
  passed and the fixture script compiled with Python. [Clippy](clippy.txt).
- Release `decode_smoke` build passed (3.31 s). All ten rebuilt-probe results
  matched their expected exit codes. [Build](release-probe-build.txt).

The generation helper is evaluation-only; test-after was used to validate its
actual PNG metadata and resulting decoder behavior. No desktop UI code changed.
The native package follow-up below verifies two corrected geometry imports.
Native Windows remains unverified. Whole-app memory under complex maximum-size content, generation
context pressure, enterprise-quality evaluation and lower-memory hardware remain
open. This bounded decoder evidence does not complete full product acceptance.

## Packaged Mac follow-up

The isolated release app was rebuilt with the engine/decoder overlays and
`com.cachella.mediacheck` identity: 27.38 s optimized Rust build, 114.85 MiB bundle.
[Build log](package-build.txt), [source and executable hashes](package-artifacts.json).

Native import accepted `side8192.png` and `odd-video.mp4`; the latter opened its
stored source preview. Read-only extraction and FFprobe confirm stored dimensions
512 × 1 and 512 × 256. Both document IDs match SHA-256 of the v2 normalization
prefix plus original source bytes. Normal quit left zero test-bundle executables.
The earlier 58 embeddings remain; the isolated profile now has five documents,
60 chunks and two new unindexed passages. Original user profile untouched.
[Before counts](native-before.json), [native and stored-payload evidence](native-package-observations.json).

The picker initially accepted the path but left its Go overlay open; dismissing
that overlay exposed the correctly selected file. Later entry waited for the
focused path field before typing. No product change was made for this automation
behavior. No model inference or resource benchmark was repeated in this follow-up.
Docs and evidence-only updates are exempt from test-first; report links, stored
payload dimensions, fingerprints, package hashes and shutdown were verified.

## Native embedding and retrieval follow-up

The same corrected release package indexed both new geometry fixtures, reaching
60/60 passages. Read-only database checks confirm both new vectors contain 768
finite floats with norms in [0.999, 1.001]. All previous 58 profile identifiers and
vector SHA-256 hashes remain unchanged from the pre-index snapshot.

Semantic query `solid red image` returned eight results with `odd-video.mp4`
first. Query `thin red horizontal line` returned eight results with `side8192.png`
first and `odd-video.mp4` second. Opening the image result displayed its stored
source preview. Normal quit left zero test-bundle executables.
[Previous vector hashes](index-before.json),
[native and vector observations](native-index-observations.json).

The isolated profile now has five documents, 60 chunks and 60 embeddings, with
no pending passages. These two synthetic queries qualify the corrected imports'
embedding/retrieval path; they establish no general retrieval-quality score.
No executable changes or new package were made in this follow-up, so existing
code checks were reused. Documentation and evidence-only changes are exempt
from test-first; local links, row/vector checks and normal shutdown were verified.
