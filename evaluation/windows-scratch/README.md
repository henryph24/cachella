# Windows scratch deletion

The Windows scratch path now uses a dedicated native backend. The previous cap-primitives Windows implementation closed its retained directory handle before recursively deleting a pathname. That left a replacement window.

The new backend opens direct children with `NtCreateFile` relative to retained parent handles, rejects reparse points for owned directories and lease files, and removes opened objects with `NtSetInformationFile(FileDispositionInformation)`. Cleanup has no inner-path recursive fallback. The trusted OS temporary parent can resolve normally, including a redirected parent. It requires read/traverse access; owned descendants acquire deletion access.

Owned directory handles share read, write and deletion access so independent parent/guardian session opens can coexist. Relative operations and deletion retain object identity across renames. Lease and coordinator files omit deletion sharing while held; recovery releases the abandoned lease before deletion, under the existing coordinator lock. Reparse children encountered during cleanup are deleted as links without traversing their targets. Windows ACL inheritance remains unchanged and is not independently inspected.

Cleanup permits at most 64 directory levels, 16,384 returned records and 4 MiB of UTF-16 filename bytes across a traversal. Each query uses an aligned 64 KiB buffer. The same budget spans every query batch and recursive child. Limit or sharing failures return errors without a pathname fallback; normal Drop cannot report its error, so abandoned workspaces remain eligible for later startup recovery.

## Verification scope

The [check harness](../../scripts/scratch-check/README.md) compiles the exact production scratch source and its tests. It excludes unrelated desktop dependencies and requires no Windows C compiler for `cargo check`. Shared direct dependency versions match the application lockfile.

- Windows target: `cargo check --manifest-path scripts/scratch-check/Cargo.toml --target x86_64-pc-windows-msvc --tests` passed. [Output](windows-check.txt).
- Windows target: the same harness passed `cargo clippy` with `--tests -- -D warnings`. [Output](windows-clippy.txt).
- macOS: `cargo test --manifest-path src-tauri/Cargo.toml --lib --features desktop` returned **123 passed, 0 failed** in 1.57 seconds. All-target desktop Clippy with warnings denied passed in 1.93 seconds, and `cargo fmt --check` passed. [Test output](mac-tests.txt), [Clippy output](mac-clippy.txt).
- The byte parser runs on macOS. Its continuation regression first failed with `assertion failed: parse_names(&record, 2).is_err()`, then passed after requiring space for the next header. [Failure](parser-red.txt). Other cases cover four-byte record padding, malformed lengths, and cumulative entry/filename budgets across query batches.
- Windows-only tests cover direct and ancestor replacement, held-file failure/retry, component rejection and reparse-target preservation. The existing session/guardian/cleanup tests also compile for Windows. They have not executed here.

The reparse test requires Windows Developer Mode or symbolic-link privilege and is explicitly ignored in ordinary local test runs. The desktop CI definition includes a Windows-only command that explicitly runs it with `--ignored`, so absence of that privilege will fail that check. CI has not been executed remotely.

Test-first native execution was unavailable on this macOS host. The first Windows compile found binding errors; source inspection corrected lock no-follow flags and record layout/alignment before the passing check. Independent review found a query-batch-only entry cap; the final budget spans the traversal and has portable cumulative-page tests. The second review found no remaining material issue in the fix; its portable harness run passed 16 tests. These findings are source and test evidence, and do not establish Windows kernel behavior.

## Reproduction and remaining work

The local cross checks use Rust 1.91.1 with the Windows MSVC standard library installed under `.local/rustup`, and build artifacts under `.local/scratch-check-target`. This leaves the user's global Rust toolchain selection unchanged.

On Windows, run:

```powershell
cargo test --manifest-path scripts/scratch-check/Cargo.toml
cargo test --manifest-path scripts/scratch-check/Cargo.toml deleting_a_reparse_child_never_traverses_its_target -- --ignored
```

Then run the full desktop build, process/guardian crash probe and native startup checks on Windows. Cross compilation does not link or execute the Windows app. No Windows runtime, Windows packaging/signing, or live Windows process behavior was verified here. Frontend tests, model inference and macOS repackaging were skipped because this increment changes the platform cleanup backend and its tests.

Settings-file recovery is covered in a [separate increment](../settings-recovery/README.md). The full product retains unfinished decoder distribution, media-to-text answers, consumer resource/quality qualification and final acceptance work. No deployment or publication occurred.

## API references

The implementation follows Microsoft's documented [relative file creation](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/nf-ntifs-ntcreatefile), [directory enumeration](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/nf-ntifs-ntquerydirectoryfile), [four-byte name-record layout](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/ns-ntifs-_file_names_information) and [handle-based disposition](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntifs/nf-ntifs-ntsetinformationfile).
