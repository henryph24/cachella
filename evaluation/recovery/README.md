# Interrupted model download recovery

## Behavior and boundaries

On desktop startup, recovery visits only the two pinned model bundle/revision directories. Installation retry runs the same recovery before checking or downloading assets. A stable `.installation.lock` file carries an exclusive OS lease for the entire scan, transfer, verification and publication. Startup skips a busy revision; a conflicting installation returns an explicit retry error. The lock file is retained after release to preserve its inode identity.

Recovery removes only regular `.download-<suffix>.part` files. Accepted suffixes are the previous six ASCII alphanumeric characters or the current 32 hexadecimal UUID characters. Complete models, unknown revisions, unrelated files, directories and symlink partials remain untouched. Symlinked storage directories or lock files fail explicitly. Errors stop desktop startup before commands become available; retry errors appear through existing installer progress.

`cap-std` and `cap-fs-ext` retain directory handles from the trusted application-data parent. Each owned root, bundle and revision opens without following a final-component symlink. Lock/model opens, partial creation, verification, cleanup and no-clobber hard-link publication use those handles. Installation reopens the expected directory chain and compares device/inode identity before returning selected file paths. Models remain mutable by the operating-system account; this does not sandbox an attacker controlling the parent directory or application process. Filesystems without hard-link support cannot publish a download and return an error.

The existing in-process gate still allows one model install per application process. The new lease coordinates cooperating processes sharing a revision. Close older builds before upgrading: earlier versions do not hold this lease. Rust 1.89+ is declared for standard-library file locking; runtime checks here use macOS arm64 and Rust 1.91.1. Windows execution remains unverified.

## Verification

The initial retry regression failed with `retry left the abandoned partial on disk`; see [red output](regression-red.txt). Independent review then found a pathname-check race. A deterministic test renamed the locked directory and replaced its path with a symlink; the initial implementation removed the external fixture. See [race red output](swap-regression-red.txt). The corrected regression verifies recovery, new partial creation/drop and directory-identity rejection after replacement.

The suite also checks active-owner preservation, forced child termination and subsequent OS-lock release, cleanup byte counts, idempotence, unknown names/revisions, missing roots, static symlinks, real loopback HTTP download/retry, content verification, cancellation and no-clobber publication. An exact conflict/retry test passed in isolation after a parallel-run transient; explicit unlock now ends the lease before descriptor close to avoid a forked child's temporary descriptor inheritance extending its lifetime.

Final `npm run test:rust` returned **97 passed, 0 failed**; see [Rust output](rust-tests.txt). The focused model suite passed 23 tests; see [model output](model-tests.txt). Direct Homebrew all-target desktop Clippy with warnings denied exited 0 ([output](clippy.txt)); Rust formatting passed. Independent review completed two rounds, with no remaining material issue under the stated trust boundary.

`npm run desktop:build -- --debug --bundles app` produced one **115.00 MiB** macOS debug bundle; see [build output](package-build.txt). Production `install_models_smoke` reverified and reused the existing **1,107,409,472-byte** Q4 model with its expected digest, header and size in **35,833 ms** during concurrent build work ([report](verified-reuse.json), [output](verified-reuse.txt)). This is an observed run, with no performance qualification inferred.

The packaged startup check reclaimed one disposable **4,096-byte** partial and preserved an unrelated probe. All three model hashes stayed identical. Read-only SQLite checks confirmed **7 documents, 12 chunks and 12 vectors**; the native UI showed seven documents/twelve passages. The app had zero descendant processes. The preserved disposable probe was removed by the test after verification. See [before state](native-before.json) and [native results](native-results.json). Forced process termination is covered separately by the production-lease child regression.

Frontend tests were not repeated because UI code is unchanged; TypeScript/Vite built through packaging. Windows execution and real OS-crash recovery remain unverified.

## Remaining scope

Decoder/PDF scratch directories and runtime TLS files still need orphan recovery. OS-crash durability, Windows execution, unrestricted local-account tampering and storage devices without hard-link support are unqualified. This increment does not complete whole-app memory qualification, media-to-text answers, decoder distribution, representative quality evaluation or the full product audit.
