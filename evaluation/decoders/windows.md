# Windows decoder source build, 2026-10-10

Status: cross-build and linkage verified on macOS ARM64. Native Windows execution, MSYS2 execution and installer behavior remain unverified.

## Implementation

The shared preparer accepts `--platform windows-x86_64`. It builds static zlib and x264 dependencies, retains x86 CPU assembly, builds FFmpeg/FFprobe with Win32 threads and disabled networking, and checks x86-64 PE format plus a finite Windows system-DLL allowlist. macOS retains its existing native flags and system zlib. A replacement cannot silently switch the output directory to a different target.

Windows output includes FFmpeg 9.0.2, x264 commit `0480cb05fa188d37ae87e8f4fd8f1aea3711f7ee`, zlib 1.3.2 source archives, upstream licenses, build/configuration logs, standalone rebuild scripts and a diagnostic receipt. Every preparation invocation rebuilds hash-verified source. Compiled outputs are never reused based on a receipt.

The zlib archive is 1,320,064 bytes, SHA256 `d7a0654783a4da529d1bb793b7ad9c3318020af77667bcae35f95d0e42a792f3`, matching the checksum on the [zlib release page](https://zlib.net/). No zlib signature verification is claimed.

## Verified checks

- `python3 scripts/prepare_decoder.py --platform windows-x86_64 --sources .local/decoder-source --output .local/decoder-windows --jobs 8`: [completed](windows-build.txt).
- [Output inspection](windows-inspection.txt): 19 files match the local build receipt; all three pinned source archives and the zlib license are included. `ffmpeg.exe` is 14,094,848 bytes and `ffprobe.exe` is 13,885,440 bytes, totaling 27,980,288 bytes. Both report PE x86-64. Imports are Windows system libraries and UCRT API-set contracts. x86 assembly is enabled; source protocol configuration enables file and pipe only. Included rebuild scripts match the workspace versions.
- `python3 -m unittest discover -s scripts -p 'test_*.py'`: [31 passed](windows-python-tests.txt), including 17 decoder tests. This covers target flags, replacement target identity, rejected import diagnostics, accepted UCRT names, rejected unknown contracts and existing source/publication/process cleanup regressions.
- macOS source preparation through the shared implementation: [completed](macos-regression-build.txt). [Production preparation smoke](macos-regression-smoke.txt): 11 fixtures passed across all eight supported extension families. Later changes affected Windows import validation only.
- Independent review completed two bounded rounds, with no remaining finding. It did not execute Windows binaries.
- Workflow YAML and package JSON parsed. Windows CI points to MSYS2 source preparation, all 11 existing fixture paths, native Rust decoder smoke and a bundle command containing both Tauri resource overlays. Five added fixture hashes match their [manifest](fixtures/manifest.json).

## Findings resolved during build

FFmpeg prepends the cross prefix to its default pkg-config command. Explicit `--pkg-config=pkg-config` fixed discovery after zlib and x264 compiled. A regression first failed before the flag was added.

The first completed binaries were rejected by an overly narrow import allowlist, and the generic error omitted the rejected names. Named diagnostics were added with a failing regression. A minimal compiler probe and the installed [UCRT import library](windows-ucrt-library.txt) identified the OS API-set names. [Microsoft documents UCRT as an OS component on Windows 10 and later](https://learn.microsoft.com/en-us/cpp/windows/universal-crt-deployment?view=msvc-170). The allowlist now permits those 15 exact contracts; unknown contracts and `libwinpthread-1.dll` remain rejected. The final build passed inspection.

A redundant diagnostic build was deliberately interrupted after the compiler probe identified the cause; its owned process exited 130. The final build started fresh after the policy correction. No failed binary output was promoted or reused.

## Windows delivery path

The workflow uses [MSYS2 setup](https://github.com/msys2/setup-msys2), UCRT64 GCC/NASM and `/usr/bin/python3` for POSIX source tooling. After preparing the decoder, native Windows Cargo runs `decode_smoke`, and `npm run desktop:bundle:windows` packages the engine and decoder overlays. The README documents the same two-step local workflow. The engine-only `desktop:build` remains available as a separate development path.

The Windows decoder targets Windows 10+ and relies on the OS UCRT. Native Python on Windows is intentionally rejected for source compilation; the builder requires the MSYS2 POSIX Python environment. The resulting executables have no MSYS runtime import.

## Unfinished

No Windows CI job or native Windows command ran in this session. Runtime decoding, process cleanup on Windows, installer/signing behavior and Windows consumer performance remain release gates. Fresh-profile native macOS video import, playback and restart persistence were subsequently verified in [native UI evidence](native.md). Media-to-text answers, broader quality/resource qualification and final application acceptance remain unfinished. No publication or deployment occurred.
