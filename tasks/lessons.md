# Cachella lessons

## 2026-10-09: SQLite key changes require an upgrade path

The semantic-index implementation changed the embedding table key from `(chunk_id, profile)` to `chunk_id`. `CREATE TABLE IF NOT EXISTS` left the prior development database unchanged, and the native indexing flow failed with `ON CONFLICT clause does not match any PRIMARY KEY or UNIQUE constraint`. Fresh temporary-database tests passed and did not cover this state.

Before changing an existing SQLite schema, inspect the prior schema and add a transactional migration with a regression that opens a populated prior-version database. Verify the upgrade through the application entry point and preserve source documents. Fresh-database tests alone do not establish upgrade compatibility.

## 2026-10-09: Owned subprocess cleanup must survive observation failure

The resource collector's `finally` block called process discovery before terminating its private process group. A failing `ps` call therefore bypassed signals and reaping. Independent review reproduced the leaked probe. Cleanup now signals the group created by `Popen(start_new_session=True)` directly, waits for the owned child and closes its pipe even when discovery fails. A regression injects discovery failure; another times out a probe with a child.

Keep cleanup dependent on ownership established at creation. Observation, metrics and logging failures must not prevent terminating and reaping an owned process. Verify both normal completion and the failure of the observation mechanism itself.

## 2026-10-10: Cancellation must finish before process exit

The initial model installer requested cancellation on desktop quit and relied on future-local temporary-file destructors. Tauri's terminal exit could call `process::exit` before the async future resumed or dropped, leaving an incomplete download. Independent review identified the race; a child-process test failed on the original behavior.

Normal quit now closes the install gate, requests cancellation and prevents desktop exit while an install is active. It waits for the guard that follows temporary-file cleanup before requesting exit again. New installs and runtime startup are rejected after closing begins; startup and shutdown use the same runtime lock. The child-process regression and packaged native quit both verified partial cleanup. Forced process termination remains outside this guarantee and needs a separate orphan-recovery design.

## 2026-10-10: Reused executable artifacts need a trusted payload anchor

The first engine preparer verified extracted files against hashes in an editable local manifest. Independent review replaced the prepared executable and updated its manifest hash; reuse accepted both changes even though the manifest declared the pinned upstream archive. Parent reproduced the failure on a disposable copy. Archive verification during the original download did not authenticate later prepared-directory reuse.

The preparer now pins the canonical extracted file-list fingerprint in its static artifact catalog and enforces it during both fresh publication and reuse. The regression changes the binary and manifest together and verifies rejection. Keep the trust anchor outside the mutable cache being checked; checking a cached file against a hash stored beside it only detects accidental single-file changes.

## 2026-10-10: Guardian startup needs an explicit ownership boundary

The first private-pipe guardian could be killed on an acknowledgement timeout after spawning its tool. That removed the process responsible for cleanup and could orphan the tool. Error returns after spawn also relied on `std::process::Child` dropping, which does not kill or reap. A relay join could block cancellation when output remained unread.

Use a pre-spawn READY/START commit gate: direct termination is safe before START, and post-START failures must close the lease and let the owner clean up. Keep an owned-child cleanup guard across every later error path. Await normal output completion while continuing to observe cancellation. Failure-injection tests must synchronize on child readiness; fixed sleeps can test a different lifecycle phase. The final guardian suite covers delayed acknowledgement, injected failure, abrupt parent exit and unread output; the native package force-kill removed two engines and two guardians with zero survivors.

## 2026-10-10: Parser byte limits must cover later expansion steps

PDF extraction initially used lopdf's bounded stream decoding and a guarded 30-second tool wait. Independent review traced a later ToUnicode reverse-map construction that expanded every source code in a range. A small CMap could therefore request billions of entries after passing the byte limit. A bounded 65,537-entry regression reproduced the missing cardinality check without allocating an unbounded input.

The pinned dependency now checks mapping count and conservative cloned UTF-16 units before reverse-map expansion. A first patch still allowed the font fallback to swallow the limit error; the failing regression exposed that path, and the error now propagates through extraction. Trace limits through all semantic expansion and fallback paths. Keep the limit's scope explicit: per-stream and per-CMap caps do not establish an operating-system process memory quota. Preserve the upstream license, patch provenance and positive normal-input coverage when maintaining a dependency patch.

## 2026-10-10: Cleanup needs an anchored directory handle

Interrupted-download recovery initially checked directory and lock pathnames with `symlink_metadata`, then reopened those paths for locking, scanning and deletion. Independent review identified the replacement window. A deterministic regression renamed the locked revision and put a symlink at its old pathname; cleanup deleted the outside fixture.

Open each owned descendant relative to a retained parent handle without following symlinks. Keep recovery, temporary creation/drop, verification and publication relative to that handle. Recheck directory identity before returning pathname-based selections. State the ambient parent trust boundary explicitly. Static symlink tests do not cover a pathname replacement after validation; retain a rename/swap regression. This patch uses cap-std/cap-fs-ext and preserves the stable lock inode across cooperating instances.

## 2026-10-10: Cancellation and completion need a shared finalization decision

The first answer-cancellation UI discarded results as soon as the user requested cancellation. If the answer completed before the cancellation IPC reply, finalization cleared its request identity. A later false/rejected cancellation could then lose the completed answer with no status. Review reproduced this ordering. Three delayed-acknowledgement regressions first failed.

Retain the operation identity and result until any in-flight cancellation acknowledgement resolves. Accepted cancellation suppresses the result; false/rejected cancellation preserves a completed answer and reports its status. Treat cancellation before submission separately: once the user cancels while registration is pending, never submit the work, even if reservation cleanup fails. A fourth regression caught that fallthrough. Keep cancellation scoped to its request ID so a late acknowledgement cannot affect a retry.

## 2026-10-10: Reject stale requests before shared admission

The generalized cancellation registry initially acquired the global operation permit before checking request identity. Independent review found that a delayed cancelled request could briefly occupy the permit and reject a valid retry. A regression confirmed that a stale ID consulted the busy gate and returned a busy error.

Validate current request identity and acquire the permit inside one registry critical section. Permit acquisition must remain non-blocking while holding the registry lock. Keep stale-request and valid-operation interleavings in regression coverage; sequential cancellation/retry tests alone do not prove admission isolation. The final review confirmed the revised lock ordering.

## 2026-10-10: Audit every cleanup branch and each platform implementation

Scratch cleanup initially reopened a workspace by its session-relative name, and a replacement-directory regression showed that cleanup deleted the replacement marker. Retaining the workspace directory handle fixed ordinary Drop and recovery. Independent review found the old name-based deletion still present in an allocation-error branch. It now calls the same retained-directory cleanup approach. Inspect failure paths as well as successful cleanup, and distinguish direct helper tests from forced failures through their callers.

The dependency API name did not establish equal behavior across operating systems. cap-primitives 3.4.6 removes an open directory through retained handles on Unix, but its Windows implementation obtains the path, closes the handle and calls std::fs::remove_dir_all. The Unix tests therefore establish only Unix rename safety. Windows-safe recursive deletion remains an explicit unfinished requirement; a passing Unix suite cannot qualify that fallback.

## 2026-10-10: Validate native records and budget the entire traversal

The first Windows directory parser used a padded structure size as its filename offset and formed a typed reference from byte storage. Native record boundaries must come from the documented ABI; safe byte decoding avoids alignment-dependent references. FILE_NAMES_INFORMATION uses a 12-byte header and four-byte record offsets. A portable regression exposed acceptance of a nonzero continuation offset with no following record, so continuations now require another complete header.

Independent review then found that limiting each query buffer left the accumulated directory results unbounded. The same entry and filename-byte budget must survive every query page and recursive child. Portable cumulative-page tests verify this accounting; Windows target compilation and lint checks validate bindings, while native deletion/share/reparse behavior still needs Windows execution. Keep those evidence scopes separate.

## 2026-10-10: Commit persisted selections before changing runtime mode

The endpoint command switched out of managed inference before saving the new endpoint. A failed publication left the stored and displayed endpoint unchanged but silently changed inference mode. A regression reproduced that state divergence. Hold the relevant runtime and selection guards, validate the transition, commit the settings, then apply in-memory mode and selection changes. Atomic file replacement alone does not make the enclosing application operation atomic; test the state that should survive a persistence failure.


## 2026-10-10: Preserve rejected build metadata before cleaning staging

The Windows decoder compiled, but PE validation returned only a generic non-system-import error. Automatic staging cleanup then removed the binaries and inspection evidence. A minimal MinGW compiler probe exposed UCRT API-set imports; the installed UCRT import archive listed their exact names, and Microsoft documentation identifies UCRT as a Windows 10+ OS component.

Report rejected dependency names and bounded configure diagnostics before cleaning a failed build. Keep dependency policy explicit and finite, and distinguish OS API-set forwarders from redistributable toolchain runtimes. A passing cross-build establishes architecture and linkage evidence; native Windows execution remains a separate gate. Regression tests now cover named rejection, accepted UCRT contracts and rejection of an unknown contract.

## 2026-10-10: Cancellation controls must survive missing display metadata

Adding a second generation bundle made settings remount default to Qwen while a
Gemma download was active. Selecting the active bundle from the catalog fixed
the resolved-catalog case, but review exposed the earlier interval: status can
arrive before catalog metadata. During that interval there was no Cancel button,
and the parent received a false busy state. The first remount regression resolved
the catalog before checking cancellation, so it missed the defect.

Derive operation busy state directly from shared installation status. Render
generic progress and cancellation while bundle metadata is unavailable, including
catalog failure. Regression assertions must run before resolving each delayed
dependency, then verify the transition after it resolves. Bundle display metadata
must not gate cancellation or runtime-edit protection.

The subsequent native check exposed another boundary: after a remounted Gemma
verification ended, the active-bundle override disappeared and the local Qwen
default hid the terminal cancellation message. Preserve the installer's bundle
selection through completion and cancellation until an explicit new user choice.
Assert the terminal selected model and status as well as the in-flight controls;
checking only busy=false does not establish a usable cancellation outcome.

## 2026-10-10: Publish effective runtime state through every UI response

Adding session-level automatic resume to runtime status left the cancelled file
picker returning the lower-level engine status, which always reported resume as
false. Independent review found the mismatch. Status-only responses must use the
same desktop projection as polling; successful configuration changes may
intentionally clear resume intent. Trace cancel and no-op branches alongside the
normal mutation path when extending a shared status object.

The first memory-policy control silently converted load errors to a Both display
and enabled edits. Keep failed or invalid settings unknown and disabled. Its
initial fix used a shared mounted ref for asynchronous reads; StrictMode's next
setup could reactivate an abandoned read and overwrite a later saved value.
Use a per-effect active flag for each read lifetime. A delayed abandoned-read
regression now verifies the saved Single state survives that ordering.

## 2026-10-10: Verify sampled membership before claiming whole-app resources

Engine process groups excluded WebKit helpers parented by launchd. The release
collector now uses an isolated app's resource/jetsam coalition pair plus descendants,
checks the root executable and rejects the collector's own coalition. Review
identified PID replacement between discovery and accounting; recheck each member's
coalition and exclude observed attribution/accounting failures from totals.

Successful three-second video imports yielded no FFmpeg sample at either a 250 ms
or 10 ms target interval. The finer loop's actual median interval was 41 ms because
process enumeration and accounting add overhead. After two misses, a 60-second
synthetic loop produced six valid FFmpeg observations. Record actual sampling
intervals, verify that the component appears in raw data, retain failed attempts,
and distinguish a missed process from zero memory use. Keep short and longer
workloads separate and preserve exclusions for OS-wide and early-launch memory.

## 2026-10-10: Propagate cancellation into blocking verification

Dropping a `spawn_blocking` await leaves an already-running worker alive. The
embedding startup hash continued across two model files after the registered
operation was cancelled. Signal cancellation with a future-owned drop guard and
check it between bounded reads and before publishing a complete digest. Test the
worker after it has started using explicit synchronization. Record that an
in-progress filesystem call cannot be interrupted by this cooperative signal;
request completion alone does not establish worker termination.

## 2026-10-10: Validate output geometry after aspect-ratio reduction

An allowed 8,192 × 1 input passed dimension checks but FFmpeg rounded its reduced
height to zero. Thin videos had the same failure, and odd dimensions violated
yuv420p encoding requirements. Bound the final computed dimensions, including
encoder divisibility, and exercise horizontal, vertical and odd-sized media with
the actual decoder. Keep normalization fingerprint versions aligned with changed
stored previews and state reimport consequences explicitly.

## 2026-10-11: Windows GUI readiness must observe the actual window

`WaitForInputIdle` rejected the debug console-subsystem Cachella executable before
any launch diagnostics were saved. A native console-subsystem fixture later
reproduced the exact exception while opening a visible window, exposing its ready
marker and exiting with code 0. Use input-idle as diagnostic information; require
an owned visible window, an application readiness marker and clean close. Save
exit codes, stderr and failure reports before returning a failed qualification.
The fixture establishes the old gate's defect; the earlier app's actual state was
not recorded and cannot be reconstructed from the exception alone.

## 2026-10-11: Verify source downloads before expensive compilation

Windows run 38129822008 spent about 17 minutes on checks before a zlib download
failed its pinned hash. The error omitted the observed length and hash. The next
run verified all three unchanged source pins using native Python before compiling,
then passed those verified archives into the MSYS build with a second verification.
Keep integrity rejection intact, report expected/observed metadata, and move
independent network prerequisites ahead of expensive build work. Do not repin an
archive merely to make a failed download pass.
