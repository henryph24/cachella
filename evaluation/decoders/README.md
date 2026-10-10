# Included decoder qualification, 2026-10-10

Status: macOS ARM64 decoder preparation and app-bundle resource integration verified. Windows cross-build is now verified; native Windows execution remains incomplete. Fresh-profile macOS import/playback/restart is verified in [native UI evidence](native.md). See [Windows build evidence](windows.md).

## Selection behavior

Fresh settings select the included decoder. Legacy JSON path strings preserve manual selection. Included settings persist their source and resolve the current application resource directory on each use. Failed settings publication preserves the previous in-memory selection. The UI provides current selection status and an explicit included-decoder action.

Independent selection review found no concrete code defect. The initial packaging gap was resolved by `desktop:build:mac` and both resource overlays. Direct unit tests for the media-selection busy/closing branches are absent; these branches were inspected. Packaged fresh-profile macOS behavior is now verified in [native UI evidence](native.md).

## Source and build evidence

FFmpeg 9.0.2 came from https://ffmpeg.org/releases/ffmpeg-9.0.2.tar.xz. Its 12,040,788-byte archive has SHA256 `8c3850283eb25fa026482078a04051e0be17347b09ef81a0849bec15a96e002e`. The detached signature verified against the release key fingerprint `FCF986EA15E6E293A5644F10B4322F04D67658D8`; see [signature verification](source-signature.txt).

x264 came from the official VideoLAN archive for commit `0480cb05fa188d37ae87e8f4fd8f1aea3711f7ee`. Its 848,652-byte archive has SHA256 `f05c59f2e83d494c36307025dca2d3afc6b4d185f3a3453d06cc4fecd7094057`. This archive was fetched over HTTPS and hash-pinned; no x264 signature verification is claimed.

The prototype statically links FFmpeg libraries and x264. System libraries remain dynamic. It includes GPL components. Complete source archives, upstream notices and standalone rebuild scripts are included in the prepared and packaged decoder resources.

The first production smoke failed because the configure component is `image_png_pipe`, while the runtime input format is `png_pipe`. Correcting the configure component resolved PNG import. Build inspection also found x264 assembly targeting macOS 14.0; rebuilding with explicit assembler flags and deployment environment removed those linker warnings. Final binaries declare macOS 11.0; actual execution on macOS 11.0 is unverified.

[Prototype configure arguments](prototype-config.json) record the exploratory build. These contain local paths and are evidence only. The production preparation script is now `scripts/prepare_decoder.py`; the prototype arguments remain historical evidence.

## Checks

- `cargo test --manifest-path src-tauri/Cargo.toml --lib --features desktop`: [134 passed, 0 failed](rust-tests.txt), 1.48 seconds.
- `npm test`: [53 passed](frontend-tests.txt), six test files.
- `npm run build`: [TypeScript and Vite build passed](frontend-build.txt).
- All-target desktop Clippy with `-D warnings`: [passed](clippy.txt), 6.95 seconds. Rust formatting completed.
- `decode_smoke` using the final prototype: [11 fixtures passed](decode-smoke.txt), covering all eight supported extension families. This invokes production `media::prepare`, including guardian execution, private input copies, probing, previews and indexing parts. Fixture success does not establish support for every codec variant in those containers.
- `file`, `otool -L`, `vtool -show-build`, and FFmpeg protocol inspection: [ARM64, system-only dynamic dependencies, minimum OS 11.0, file/pipe protocols](binary-inspection.txt). Combined stripped executable size: 18,021,376 bytes.

## Unfinished

Windows runtime/signing, media answers and broader consumer quality/resource qualification remain unfinished. No deployment or publication occurred. Existing manual decoder support remains available.


## Source preparation and package integration

`python3 scripts/prepare_decoder.py --sources .local/decoder-source` completed a fresh build. A second invocation with `--replace` completed the replacement path. Each builds freshly extracted hash-pinned archives, disables ambient compiler/library overrides, verifies ARM64/system-library dependencies and file/pipe protocols, and includes complete source archives plus license/rebuild materials. The workspace and installed toolchain are trusted. A locally generated receipt records diagnostics and file integrity; it is never used to reuse or authenticate a compiled binary.

The initial source preparation covered macOS ARM64. The subsequent Windows source recipe and cross-build are documented in [Windows build evidence](windows.md). The default `desktop:build` retains its engine-only behavior. `desktop:build:mac` prepares both components and applies both Tauri resource overlays; the macOS CI matrix selects this command. The Windows workflow now builds its decoder with MSYS2 before using `desktop:bundle:windows`; remote CI was not run.

Publication checks destination ownership under the build lock, builds before replacing existing resources, and restores the prior directory on a failed publication. If rollback itself fails or the process is interrupted between renames, the sibling `.decoder-previous-*` directory preserves previous resources for recovery. An abrupt kill can leave `.decoder-build-*` source/build staging; automatic build-directory recovery is not implemented. These are build-time files in the trusted workspace.

Verification:

- [Python suite](python-tests.txt): 26 tests passed, including 12 decoder tests. Tests cover corrupt pins, unsafe archive members, path aliases, extraction caps, unsupported hosts, publication/rollback failures, and real timeout cleanup of a spawned grandchild. POSIX process tests and macOS preparation tests are explicitly platform-scoped.
- [Fresh preparation](preparer-build.txt) and [replacement preparation](preparer-replace-build.txt) both completed.
- [Package build](package-build.txt): one debug app bundle, 144.99 MiB, desktop Rust build finished in 19.58 seconds. Command: `npx tauri build --debug --bundles app --features desktop --config src-tauri/tauri.engine.conf.json --config src-tauri/tauri.decoder.conf.json`.
- [Package resource inspection](package-inspection.txt): all 15 decoder payload files match the prepared receipt; both executables retain execute permission; the inference engine is present. This establishes packaging integrity relative to the local build.
- [Production import using packaged binaries](package-smoke.txt): 11 fixtures passed across all eight supported extension families. This invokes `media::prepare` through the existing smoke example. A subsequent [native fresh-profile check](native.md) verified video import, playback and restart persistence.
- Independent review completed two rounds. The packaging wiring finding was fixed and verified; no further findings.
- Package scripts, both JSON resource overlays and workflow YAML parsed with expected macOS/Windows command selection. The package's normal frontend build passed. Rust behavior was unchanged in this increment; the previous 134-test and Clippy results apply to its source changes.
