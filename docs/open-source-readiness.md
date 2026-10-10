# Open-source readiness

Checked 11 October 2026. Repository: [henryph24/cachella](https://github.com/henryph24/cachella). Git and the GitHub remote are already initialized. Visibility remains private while publication preparation is in progress.

- [x] Provide a public-facing README with setup, capabilities, limits and evidence.
- [x] Preserve detailed runtime and build guidance in a development guide.
- [x] Add contribution guidance and identify major third-party licensing boundaries.
- [ ] Select the project source license, add its full text and align package metadata.
- [x] Scan all six existing commits with Gitleaks 8.30.1 and review its one finding.
- [ ] Complete the broader publication-content review.
- [ ] Review evaluation artifacts and history for personal paths, private content and unpublished information before exposing them publicly.
- [ ] Complete the resolved dependency license inventory and binary distribution review.
- [ ] Configure private vulnerability reporting and publish its instructions.
- [ ] Resolve Windows installer/runtime qualification and record actual model execution.
- [ ] Validate the intended consumer hardware; qualify cloud source integrations before advertising them.
- [ ] Publish the repository after the maintainer's release decision; create signed releases separately.

## Windows checkpoint

[Run 38028684256](https://github.com/henryph24/cachella/actions/runs/38028684256) built the Windows decoder and NSIS installer. Its qualification step failed while calling `WaitForInputIdle`, before real-model acceptance. A successful package build does not establish successful desktop startup.

## Publication boundaries

Generated binaries, model weights, local databases and credentials belong outside source commits. The repository includes development evaluation evidence and historical logs; ignore rules cannot remove files already tracked or erase history. A secrets scanner supplements review and cannot certify that a repository contains no sensitive information.

The current task prepares documentation and repository hygiene. Visibility changes, signed binary publication and accepting paid services are separate actions.

## Checks performed

- `gh repo view --json nameWithOwner,visibility`: `henryph24/cachella`, `PRIVATE`.
- `gitleaks git --redact`: six commits and 33.41 MB scanned; one generic-key finding. It matches explanatory prose about authenticated embedding and 768-dimensional vectors in `evaluation/engines/README.md:32`. The historical line equals the current prose; no credential appears in that line. The raw scanner exit remains 1 and no blanket suppression was added.
- Tracked-content inspection found user-directory paths in 143 text files, primarily evaluation evidence. Review/redaction before publication remains open.
- Five authored Markdown documents: 56 relative file-link targets resolve.
- `git diff --check`: passed for the authored patch.
- `git check-ignore .env .env.local model.gguf`: all three paths ignored.

This is documentation and ignore-rule preparation. Test-first is exempt; application tests were not rerun because executable behavior is unchanged. Secret scanning does not verify licensing, data sensitivity or model quality.
