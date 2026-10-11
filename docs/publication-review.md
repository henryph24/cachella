# Source publication review

Checked 11 October 2026 against `8e50280` and the current preparation changes.
Scope: repository content, reachable history, release claims and distribution
prerequisites. This is a bounded engineering review; it does not certify the
absence of every secret or determine all legal distribution obligations.

## Content and history

An independent read-only inspection covered 664 tracked files, 11 reachable
commits and 686 unique historical blob versions. It found no demonstrated private
source documents, credential files, databases, model weights, release executables
or removed historical filenames. Atlas records are synthetic retrieval fixtures;
upstream author contacts and signature records retain their attribution role.

There are 49 files with binary bytes totaling 91,603,631 bytes, consisting of media
fixtures and application icons. Two approximately 32 MiB MP4s are padded boundary
fixtures; a 20,010,168-byte PNG is generated seeded noise. Their generation
provenance and test role are retained.

Developer home paths appeared in 144 tracked text files: 137 maintainer evaluation
files, one hosted-runner record and six upstream lopdf examples. Current evaluation
files normalize the maintainer home prefix to `/Users/developer`; metrics and
hashes are unchanged. Original local records are retained under the ignored
`.local/alpha-review/original-evidence/` directory. Upstream examples and hosted
runner paths remain unchanged.

Existing Git history still contains the original developer paths and author
attribution. This review classifies those paths as development provenance; it
found no associated credential or private-document exposure requiring a history
rewrite. Anyone cloning a public repository can inspect that history. No history
rewrite was performed.

The 24 previously untracked delivery files were also inspected: eight contain
home-path provenance. They remain outside this preparation commit, alongside
unrelated existing working changes.

## Credential scan

Command: `gitleaks git --redact --report-format json` using Gitleaks 8.30.1.
The baseline scan covered 11 commits and about 33.62 MB. It reported one
`generic-api-key` finding in `evaluation/engines/README.md:32` from the initial
commit. Manual review confirms the line is explanatory prose about authenticated
embedding and vector dimensions. It contains no credential. The scanner's raw
exit was 1; no blanket suppression was added.

A separate scan of the prepared tracked-file snapshot is recorded in the final
verification section. Local ignored credentials, build directories, node_modules
and downloaded model files are outside the publication snapshot.

## Release boundaries

- Repository visibility remains private; no public tag, release or announcement exists.
- macOS signing identity discovery returned `0 valid identities found`.
- No Apple signing/notarization credential variables checked in this session were configured.
- Private vulnerability reporting could not be verified: the private repository endpoint returned HTTP 404. GitHub documents this feature for public repositories.
- Windows Server 2025 runtime qualification passed. Windows 11 end-user installation, consumer hardware and GPU qualification remain open.
- Project source-license selection remains pending. Resolved dependency metadata is inventoried separately; a binary notice/distribution review is still required.

## Final verification

- `gitleaks dir` on the prepared Git-index snapshot reported the same single prose finding at `evaluation/engines/README.md:32`; raw exit 1, manually reviewed as a false positive. No other findings were returned.
- `python3 -m unittest discover -s scripts -p test_dependency_inventory.py`: seven tests passed. The initial test-first run failed because the generator module did not yet exist.
- `python3 scripts/dependency_inventory.py --cargo /opt/homebrew/bin/cargo --check`: inventory matches 566 Cargo packages, 210 npm instances, zero missing package license declarations and eight separately recorded boundaries.
- All 137 normalized evidence files match the original bytes with only the documented home-prefix substitution. JSON measurements and non-path values are preserved.
- The 12 changed Markdown documents contain 102 local file-link targets; all resolve.
- `git diff --cached --check`: passed.

Independent review identified an unsafe optional metadata replay path. That option was removed; a regression failed before the fix and passed afterward. The CLI now always collects locked Cargo metadata.

Application tests were not rerun: application executable behavior and build configuration are unchanged. Documentation and evidence normalization are exempt from test-first. Dependency metadata is an inventory; zlib license classification, exact model publisher terms and per-target binary notices still require review.
