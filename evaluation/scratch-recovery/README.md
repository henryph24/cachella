# Owned scratch recovery

This increment covers production media decoding, PDF extraction and managed inference TLS files. New files live in a private `cachella-scratch-v1/session-<UUID>/workspace-<UUID>` tree below the OS temporary directory. A desktop startup scans that namespace before exposing commands. Examples initialize lazily when they first need a workspace.

## Ownership and recovery

The app holds a shared OS lease on its session. Every process guardian attaches an independent shared lease before acknowledging startup and retains it until its owned tool has been reaped. Recovery takes an exclusive lease before deleting a session, so a live parent or guardian keeps it in use. A stable coordinator serializes session creation, guardian attachment and recovery; contention has a five-second limit. No PID or file-age heuristic determines ownership.

The OS-provided temporary parent is the ambient trust boundary. All owned descendants are opened without following symlinks. On Unix, workspace cleanup and session recovery remove contents through retained directory handles; the real-directory replacement regressions confirm that replacement contents survive. The code preserves unrelated root names and root-level symlinks. An empty session interrupted before its lease file exists can be reclaimed under the coordinator. Malformed leases, unexpected nonempty incomplete sessions and recovery deletion failures return errors.

Unix directories and lease files use modes 0700 and 0600. Windows inherits the per-user temporary directory ACL; this implementation does not inspect Windows ACLs and has not been executed on Windows. Independent review also traced cap-primitives 3.4.6 on Windows: `remove_open_dir_all_impl` closes the retained directory and calls `std::fs::remove_dir_all(path)`. That pathname fallback leaves a concurrent-rename gap. The subsequent [Windows backend increment](../windows-scratch/README.md) replaces this fallback with native relative opens and deletion by handle. Windows execution is still required before claiming cross-platform recovery safety. External helpers still receive filesystem paths inside the private directory; this is not a sandbox against arbitrary malicious code running as the same OS user.

Each workspace attempts cleanup on ordinary Drop. Empty process session metadata may remain until the next startup, when its OS lease is released. An abrupt app exit may leave source snapshots, decoded media, extracted PDF results or TLS material; a surviving guardian first finishes child cleanup, and a later startup reclaims the abandoned session.

Legacy anonymous temporary directories have no reliable ownership marker and are preserved. Atomic settings publication files in app data remain outside this recovery contract. Model download partials have their separate catalog-scoped lease and recovery implementation.

## Verification approach

The runtime regression first failed because TLS files lived in anonymous temporary directories. The first cleanup implementation also failed a real-directory replacement test: name-based recursive removal deleted the replacement marker. Retained open-directory cleanup fixes that path. Independent review found the same pattern in the allocation failure branch; it now passes the retained directory into a cleanup helper. Its regression directly tests that helper after directory replacement. The initial red was a missing-helper compile failure, which does not demonstrate a forced shared-lock allocation failure. Review found no remaining material Unix defect within the stated trust boundary; Windows runtime qualification remains open.

The POSIX subprocess probe uses the production guardian. It freezes that known guardian after its tool starts, abruptly kills the parent, runs startup recovery while the guardian holds the session, then resumes the guardian and verifies tool cleanup and recovery. Freezing creates a deterministic crash window without changing production behavior. The PDF/media/TLS payloads in this probe are synthetic; parser and model integration are verified separately.

```sh
cargo build --manifest-path src-tauri/Cargo.toml --example scratch_recovery_probe --example process_guard_fixture
python3 evaluation/scratch-recovery/run_probe.py
```

Actual PDF, embedding and Q4 generation integration uses `pdf_smoke`, now running through the owned workspace and guardian leases. The Rust suite also decodes local image/audio/video fixtures when FFmpeg is available. The macOS verification environment has the selected FFmpeg installed.

## Verified results

- `npm run test:rust`: **117 passed; 0 failed**. Includes ten scratch tests, runtime TLS cleanup, PDF helper behavior and actual local image/audio/video decoding. [Full output](rust-tests.txt).
- Focused scratch suite: **10 passed**. [Output](allocation-cleanup-green.txt).
- All-target desktop Clippy with `-D warnings`: passed. [Output](clippy.txt).
- Rust formatting: exited 0. [Output](format.txt).
- POSIX crash/guardian probe: **four PASS lines**. [Raw output](process-probe.txt), [structured results](process-results.json), [runner](run_probe.py).
- Actual PDF, embedding and Q4 integration: **five PASS lines**, including cited page-3 answer and both engines stopped. [Raw output](model-smoke.txt), [model results](model-results.json). This exercised the normal production path before the final failed-allocation cleanup helper was added; the final helper has its targeted regression.
- TypeScript/Vite and native package build: **one macOS debug bundle, 115.23 MiB**. [Build output](package-build.txt). Frontend behavior was unchanged, so frontend unit tests were skipped for this increment.

Test-first artifacts: [anonymous TLS failure](runtime-red.txt), [replacement marker deletion](scratch-recovery-red.txt), [nine-test recovery suite](scratch-recovery-green.txt), and [missing allocation helper compile failure](allocation-cleanup-red.txt).

### Packaged startup

The native app was closed, and the subprocess fixture created three synthetic PDF/media/TLS workspaces in the default owned namespace. Its parent was killed and its actual guardian/tool were confirmed exited. The new packaged app started and removed all three workspaces/files. An unrelated root marker survived; the verifier then removed only its own exact test marker. The library retained eight documents, 51 passages and 51 vectors; the app had zero descendants.

`python3 evaluation/scratch-recovery/verify_native.py` returned `PASS: native startup reclaimed 3 workspaces / 3 files; unrelated file preserved; 8 documents / 51 passages / 51 vectors unchanged; 0 app descendants`. [Result](native-results.json), [output](native-check.txt), [preparation](prepare_native.py), [verification](verify_native.py). This is a one-session fixture check; prepare a new fixture before rerunning it.

Documentation-only updates are test-first exempt. Local links, fences, JSON report parsing and recorded counts are checked separately.

## Scope still open

The native Windows backend replaces the identified pathname-recursion fallback; it still requires Windows execution coverage. Settings-publication recovery is covered in a [separate report](../settings-recovery/README.md); legacy anonymous temporary files are intentionally preserved. Whole-app resource and quality qualification, media-to-text answers, decoder distribution, Windows execution/signing and full product acceptance remain separate unfinished work. No deployment or publication is authorized by this increment.
