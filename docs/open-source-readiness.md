# Open-source readiness

Checked 11 October 2026. Repository: [henryph24/cachella](https://github.com/henryph24/cachella). Git and the GitHub remote are already initialized. Visibility remains private while publication preparation is in progress.

- [x] Provide a public-facing README with setup, capabilities, limits and evidence.
- [x] Preserve detailed runtime and build guidance in a development guide.
- [x] Add contribution guidance and identify major third-party licensing boundaries.
- [ ] Select the project source license, add its full text and align package metadata.
- [x] Scan all 11 existing commits with Gitleaks 8.30.1 and review its one finding.
- [x] Complete the bounded publication-content review; see [findings and scope](publication-review.md).
- [x] Review evaluation artifacts and history; normalize personal home prefixes in 137 current evidence files and disclose retained historical paths.
- [x] Generate the resolved [dependency license inventory](dependency-licenses.md): 566 Cargo packages and 210 npm entries.
- [ ] Complete per-target binary notices, model terms and distribution review.
- [ ] Configure private vulnerability reporting and publish its instructions.
- [x] Qualify the Windows release installer and actual model execution on the hosted runner; consumer Windows 11 acceptance remains open.
- [ ] Validate the intended consumer hardware; qualify cloud source integrations before advertising them.
- [ ] Publish the repository after the maintainer's release decision; create signed releases separately.

## Windows checkpoint

[Run 38131003959](https://github.com/henryph24/cachella/actions/runs/38131003959) passed all six hosted Windows qualification gates, including release GUI readiness, installed helper modes, Qwen3 inference and five multimodal retrieval cases. The [retained report](windows-runtime-qualification.md) records Windows Server 2025 scope and the remaining Windows 11, signing, interactive-import and consumer-hardware limits.

## Publication boundaries

Generated binaries, model weights, local databases and credentials belong outside source commits. The repository includes development evaluation evidence and historical logs; ignore rules cannot remove files already tracked or erase history. A secrets scanner supplements review and cannot certify that a repository contains no sensitive information.

The current task prepares documentation and repository hygiene. Visibility changes, signed binary publication and accepting paid services are separate actions.

## Verification record

The [publication review](publication-review.md) records the 11-commit history scan,
prepared snapshot scan, content audit and remaining prerequisites. The scanner's
single prose finding is documented explicitly; its raw exit remains 1.

The inventory generator has seven focused fixture tests and a reproducibility check.
Documentation and evidence-path normalization are exempt from test-first; application
behavior is unchanged. Signing discovery returned `0 valid identities found`.
Private vulnerability reporting returned HTTP 404 and remains unverified.

See the [alpha release notes](alpha-release.md) for source and installer gates.
