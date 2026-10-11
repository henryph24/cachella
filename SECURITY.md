# Security

Cachella is a pre-release desktop application. It has no supported public release or completed independent penetration test. The current source review and verification scope are recorded in the [security review](docs/security-review.md).

## Reporting a vulnerability

While the repository is private, report concerns through your existing private channel with the maintainer. A public-release reporting channel still needs configuration. The repository API returned HTTP 404 for private vulnerability reporting during preparation; it has not been verified as enabled. See the [source-alpha publication gates](docs/alpha-release.md). Avoid posting exploit details, credentials or private source documents in a public issue. Include the app revision, operating system, affected path, impact and a minimal synthetic reproduction.

## Trust boundaries

- Imported documents and model output are untrusted. The app displays content as text, constrains local media serving and validates cited source identifiers. Citation validation does not establish factual accuracy or immunity to prompt injection.
- Manually selected engines and decoders execute with your user account's access. Select binaries only from a trusted source. The bundled resources use pinned preparation inputs; published packages still need signing and notarization.
- Managed inference uses a private authenticated loopback TLS connection. An independently managed local server is a separately trusted service that receives prompts and evidence; it may have its own network or logging behavior.
- Local storage has no application-level encryption. Protect your account, disk and backups. Local storage does not protect against malware or another process running as your user.
- Media/PDF/model parsing uses native components. Current input bounds and subprocess isolation do not provide an OS sandbox or a hard process-memory quota.
- Cloud connectors remain planned; they require explicit data-processing, authorization, retention and deletion review before release.

## Release requirements

Before public macOS distribution, sign the app and bundled executables with the intended Developer ID, notarize and staple the package, verify it under Gatekeeper on a clean Mac, and publish the package provenance. Keep dependency/advisory checks current and retain third-party distribution materials. Source licensing, private vulnerability reporting and consumer-hardware qualification are also pending.
