# Cachella 0.1.0 source alpha candidate

Status: prepared for review in the private repository. No public release, tag or
end-user installer is published. Package versions remain `0.1.0`; alpha status is
explicit in these notes and the README. The source-license decision is pending.

## Intended audience and setup

This candidate is for developers and invited testers evaluating local document
search and cited answers. Follow the [developer setup](../README.md#for-developers)
to build from source. End-user macOS setup is documented in the
[installation guide](../README.md#install-on-macos) for a prepared package; such a
public package is not yet available.

A prepared app includes its engine and decoder. Model weights download separately:
about 1.11 GB for the default answer model and 0.86 GB for semantic search. Optional
media-answer weights add about 3.40 GB. GGUF download size does not establish RAM
requirements. The core local features require no cloud API key or Cachella account.

## What has been exercised

| Environment | Evidence | Limit |
| --- | --- | --- |
| Apple silicon macOS 14.4 | Native packaged launch, local import/search/inference and lifecycle evidence in the [evaluation index](../evaluation/README.md) | Development packages; unsigned distribution and clean-Mac installation remain unqualified |
| Windows Server 2025, hosted x64 runner | [Release qualification](windows-runtime-qualification.md): NSIS install, ready window and exit 0, PDF/guardian helpers, Qwen3 inference, five retrieval cases, process cleanup | Windows 11 user flows, GPU performance and consumer hardware remain unqualified |
| Models | Qwen3 text answers; EmbeddingGemma 2 retrieval across text, image, audio and decomposed video | Synthetic checks; no general enterprise-quality claim |

Notion, Drive, watched Obsidian vaults, Composio and enterprise LMCache remain
planned. Back up test libraries before changing builds; cross-version backup
restoration and a public updater have not been qualified.

## Publication gates

| Gate | Current evidence | Remaining action |
| --- | --- | --- |
| Project license | Maintainer selection requested | Add the selected full license and align npm/Cargo metadata |
| Source content and history | [Publication review](publication-review.md); historical paths classified as development provenance | Keep the review's scope and path disclosure with the release |
| Dependency licensing | [Resolved inventory](dependency-licenses.md) | Resolve any missing metadata; retain notices and separate binary/model terms |
| Private security reporting | Private-repository API returned HTTP 404 | After public visibility is authorized, enable and verify GitHub private vulnerability reporting before announcing the release |
| Public repository/release | Existing repository remains private | Publish only the reviewed commit after the source gates are satisfied; mark the release as a prerelease |
| macOS installer | `security find-identity -v -p codesigning`: `0 valid identities found` | Provide Developer ID signing identity and notarization credentials, then qualify the signed artifact |
| Windows installer | Hosted unsigned development artifact passed six runtime gates | Qualify intended user installation and signing before advertising an end-user package |

Source publication and signed installer distribution have separate requirements.
Do not attach the existing development installers to an end-user release.

## Signed macOS package acceptance

Once the signing identity and notarization access are available, build the release
configuration with the existing engine and decoder resource overlays. Tauri
supports `APPLE_SIGNING_IDENTITY` and documented notarization credentials; keep
credentials in the OS keychain or CI secret store. See
[Tauri's signing instructions](https://v2.tauri.app/distribute/sign/macos/).

The app, nested llama-server/FFmpeg executables and dynamic libraries must carry
appropriate distribution signatures. Apple's guidance covers
[nested code and signing order](https://developer.apple.com/documentation/xcode/creating-distribution-signed-code-for-the-mac/)
and [notarization](https://developer.apple.com/developer-id/). Signature changes can
change executable hashes: keep the pinned input provenance and record fresh final
package hashes after signing.

Verify the final app with `codesign --verify --deep --strict --verbose=2`, inspect
its Developer ID authority, Team ID and hardened-runtime settings with
`codesign --display --verbose=4`, and verify the stapled ticket with
`xcrun stapler validate`. Gatekeeper assessment through
`spctl --assess --type execute --verbose=4` must pass. Ad-hoc signature verification
alone is insufficient for distribution acceptance.

On a fresh standard macOS user account, test the quarantined downloaded package:
install, first open, engine/model setup, import, search, cited answer, offline
restart, cancellation, quit, backup and uninstall. Record the exact artifact hash,
OS, chip and results. Do not disable Gatekeeper or remove quarantine to pass these
checks. Signing and this clean-account exercise have not been performed here.

## Release record and support

The release record must identify the exact source commit, source-license choice,
resolved dependency inventory, test run URLs, artifact SHA-256 values, supported
OS/architecture scope and remaining limitations. Include full corresponding source
and rebuild materials for bundled GPL-enabled decoder executables when distributing
them. Model weights remain separately downloaded under the publisher's terms.

Report ordinary bugs with OS/chip, app revision, model names and a minimal synthetic
reproduction. Remove document content, personal paths and credentials from reports.
Follow the [security policy](../SECURITY.md) for vulnerability reports. GitHub's
private reporting feature is documented for public repositories in its
[configuration guide](https://docs.github.com/en/code-security/how-tos/report-and-fix-vulnerabilities/configure-vulnerability-reporting/configure-for-a-repository).
