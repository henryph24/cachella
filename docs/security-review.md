# Security review: macOS end-user preparation

Date: 11 October 2026. Scope: the current source tree and locked dependencies, with local execution on Apple silicon macOS 14.4. This is an engineering review with independent code review and focused regression tests. It is not a penetration-test certification or a claim of enterprise readiness.

## Threat model and coverage

The protected assets are imported documents/media, stored embeddings, local settings, model downloads and ephemeral engine credentials. Inputs include selected files, imported source instructions, model responses, local HTTP services and downloaded artifacts. The local OS account and explicitly selected executable are trusted. Same-user malware, a compromised OS and hostile administrators are outside the protection offered here.

| Boundary | Reviewed controls and evidence |
| --- | --- |
| Webview and native commands | React text rendering, CSP, main-window capability, native pickers, bounded command inputs and operation coordination in `src/`, `desktop.rs` and Tauri configuration. No concrete HTML injection or prompt-to-command execution path was identified. |
| Local model connections | Literal loopback endpoints, redirects/proxies disabled, bounded JSON, managed TLS pinning and per-start authentication in `inference.rs`, `embedding.rs` and `runtime.rs`. Independently managed servers remain explicitly trusted recipients of evidence. |
| Local media serving | Validated content IDs, GET/HEAD restrictions, MIME allowlist, nosniff and bounded byte ranges in `asset.rs` and `store.rs`. |
| Source ingestion | Text/PDF/media input bounds, fixed decoder arguments/demuxers, local snapshots, subprocess deadlines and PDF extraction limits. Native parser memory is not governed by a hard OS quota. |
| Downloads and files | Catalog-only model downloads, pinned size/hash checks, restricted HTTPS hosts, no-follow anchored installation/recovery paths and atomic publication. |
| Process lifetime and credentials | Cleared subprocess environments, argument arrays, guardian leases, temporary TLS material and shutdown/recovery paths. Helpers run with the user's OS privileges. |
| Distribution | Current Mac test bundle is ad-hoc signed with no Developer ID or sealed resources. No published release was found. Signing, notarization and clean-machine install checks remain mandatory release work. |

Two scoped audit agents reviewed native files/processes and desktop/network/content boundaries independently. A separate fresh-context reviewer examines the resulting security changes. Static review cannot establish absence of vulnerabilities in the native parser dependencies or resistance to every hostile document.

## Findings and changes

### App-data directory permissions

Startup previously used `create_dir_all` and inherited the ambient umask. With traversable parent directories, a `0755` app directory and default SQLite permissions could expose imported content to another local account. On the inspected Mac, `~/Library` and `Application Support` are already `0700`, which mitigates that exposure on this host.

The new Unix initializer creates the app-data leaf with `0700`, opens an existing leaf without following a symlink, applies mode bits through the opened handle, and checks the result before recovery, settings reads or SQLite open. Existing content is preserved. The OS-provided parent remains trusted. Windows retains its existing per-user ACL behavior and requires separate ACL qualification. Independent review demonstrated that a macOS extended ACL could still grant access despite `0700`. The initializer now checks extended ACLs through the retained file descriptor and rejects a custom ACL or an ACL inspection error before library access. It preserves the user's policy for administrator review. It provides no encryption; Linux ACLs and Windows ACLs require separate qualification.

Regression evidence: the previous implementation failed three of four tests (new directory mode, existing directory mode, symlink rejection). The hardening passes all five final tests, including preserving a regular file encountered at the expected directory path and rejecting an explicit macOS everyone-access ACL. The reviewer's exact-module reproduction captured the ACL bypass before the fix.

### Blocking special-file inputs

Manual model and media paths were opened before descriptor metadata established that they were regular files. A FIFO can block a normal read-open indefinitely, preventing the later rejection from running. This requires selecting or substituting a local special file; it is an availability issue under the user's account.

The selected-file boundary rejects non-regular descriptors using nonblocking opens on Unix, with shared wiring for model header validation and hashing, text import, and media/PDF input and output. Descriptor checks close the check/open replacement gap for the Unix FIFO case. The existing behavior of explicitly selecting a symlink to a regular file is preserved. Windows named-pipe behavior is outside the Unix fix's verified scope.

### Oversized passage projections

The word-based chunker permits a whitespace-free token as large as the 2 MiB input bound. Eight matching passages could therefore return roughly 16 MiB of text across IPC. This is finite resource amplification, with the 500-document and per-file limits still applying.

A shared retrieval projection limits each passage preview to 8 KiB and marks truncation. It applies to keyword results, semantic results and pending embedding batches, including existing stored chunks. Full source inspection, stored text, chunk identities and vectors remain unchanged. The first 1,800 Unicode characters used for generation and 1,500 text bytes used for embeddings fit within the projection, preserving those model inputs. The regression covers an existing 2 MiB Unicode passage, all three retrieval paths and full source preservation.

## Dependency checks

- `npm audit --json`: zero reported vulnerabilities.
- `cargo-audit audit --file src-tauri/Cargo.lock --json`: zero vulnerability advisories; two informational warnings across 567 dependencies. RustSec database revision `7eebec69c352c7191b1f13eb95dd510eeca5d1de` dated 9 October 2026.
- [RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html): `glib` iterator unsoundness, patched in 0.20.0. The locked 0.18.5 occurs in the GTK dependency chain.
- [RUSTSEC-2024-0370](https://rustsec.org/advisories/RUSTSEC-2024-0370.html): `proc-macro-error` is unmaintained; no patched version listed.
- The desktop dependency tree for `aarch64-apple-darwin` contains neither warned package. They remain recorded in the lockfile; a future Linux release needs separate disposition. No broad dependency upgrade or warning suppression was applied.

These checks cover published npm/Rust advisories for the checked lockfiles. They do not cover all possible defects in the vendored PDF fork, separately bundled llama.cpp/FFmpeg/x264 binaries, model weights or system WebKit. Review those components' provenance and advisories again for each release.

## Residual risks and release gates

1. Imported evidence can manipulate model answers even with untrusted-evidence instructions and valid citation IDs. The model has no tools; users must inspect source evidence. Representative adversarial-corpus evaluation remains open.
2. Local library contents and backups have no application-level encryption. Protect the OS account, use appropriate disk encryption and govern backups.
3. Untrusted PDF/media/model parsing is isolated or bounded in several ways, but has no OS sandbox or hard process-memory quota. Parser compromise would run with user privileges. Further sandboxing and malicious-corpus testing remain release-hardening work.
4. Full application OS-version compatibility, consumer-memory limits, signed/notarized installation, Windows runtime acceptance, custom ACL configurations and remote connector authorization remain unqualified.
5. Source-license selection, historical publication-content review and private vulnerability reporting must be completed before public release.

See [Apple's Gatekeeper guidance](https://support.apple.com/en-us/102445) and [Tauri's macOS signing guidance](https://v2.tauri.app/distribute/sign/macos/) for distribution requirements. The README provides setup instructions without instructing users to disable platform security.

## Verification results

- Unix app-data regressions: five passed, including the explicit macOS ACL case. [Initial red](../evaluation/security/app-data-red.txt), [ACL reproduction](../evaluation/security/acl-red.txt), [final green](../evaluation/security/app-data-green.txt).
- Oversized projection: demonstrated failure before the fix; two final tests passed across keyword, pending-index and semantic paths. [Red](../evaluation/security/projection-red.txt), [green](../evaluation/security/projection-green.txt).
- FIFO regression: the worker's pre-fix child hit the one-second deadline and was killed; final full-suite execution passes the bounded child test. The worker's initial red output was observed in its tool transcript and was not retained as a separate log.
- `cargo test --manifest-path src-tauri/Cargo.toml --lib --features desktop`: **167 passed, 0 failed**, 2.72 s. [Output](../evaluation/security/rust-tests.txt).
- `npm test`: **67 passed across seven files**, 3.45 s. [Output](../evaluation/security/frontend-tests.txt).
- `npm run build`: TypeScript and Vite passed. [Output](../evaluation/security/frontend-build.txt).
- Rust 1.99 all-target desktop Clippy with `-D warnings`: exit 0, 8.05 s. Formatting and `git diff --check` passed. [Lint output](../evaluation/security/clippy.txt).
- Prepared macOS release build: **114.83 MiB**, Rust build 17.29 s. [Build output](../evaluation/security/package-build.txt).
- Native isolated startup: `com.cachella.securitycheck` displayed **Workspace ready**, recognized the included engine/decoder, created an app-data directory with mode **0700**, and contained zero documents. Quit left **zero** test-app processes. No model was downloaded or started. [Recorded state](../evaluation/security/native-startup.json).
- Independent review: the ACL bypass and remaining hashing reopen were corrected; final exact-source probes passed six checks, with FIFO rejection measured at 2.2 ms. No remaining demonstrated defect in the patch's reviewed scope. [Reviewer evidence](../evaluation/security/independent-green.txt).
- [Dependency audit summary](../evaluation/security/dependency-audit.json) retains both informational warnings.
- `gitleaks dir` over 38 source/documentation/evidence files in the review snapshot: exit 0, zero detected secrets. This snapshot scan does not replace the outstanding historical publication-content review.
- Documentation validation: four edited/new documents, 29 relative file-link targets resolved. The README controls were also inspected in the packaged native UI.

The new directory helper is called before recovery/settings/database access. The shared file opener is used by six production reader sites, including subsequent embedding hashing. The passage projection is shared by all three retrieval/indexing query paths. These checks cover the changed execution paths; no full fresh model-download/inference run, notarized install, hostile-parser fuzz campaign or Windows native rerun was performed in this session.
