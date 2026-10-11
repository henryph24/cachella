# Contributing to Cachella

Cachella is preparing for a public open-source release. Source licensing is pending; the repository currently requires collaborator access.

## Development

Follow the [README](README.md#for-developers) and [development guide](docs/development.md). Use the committed npm and Cargo lockfiles. Keep generated engines, decoders, model weights and local user data outside commits.

Before submitting executable changes, run the [documented checks](docs/development.md#checks). Add focused regression coverage for behavior changes and exercise affected component boundaries. Documentation-only changes require link and formatting checks. Platform-specific changes need execution on that platform; cross-compilation alone does not establish runtime behavior.

Describe the problem, resulting behavior, checks run and remaining limits in the pull request. Keep patches focused. Use synthetic or explicitly shareable fixtures; exclude account tokens, private documents and personal data from issues, logs and tests.

## Reporting issues

Include the operating system and architecture, app revision, model and engine versions, reproducible steps and redacted diagnostic output. State whether the issue occurs with a bundled or manually selected engine. Do not attach model weights or a private knowledge library.

For a suspected security vulnerability, avoid a public issue containing exploit details or secrets. Follow the [security policy](SECURITY.md). A public-release private reporting channel still needs configuration.

## Third-party changes

Preserve upstream notices. Changes to the vendored PDF parser, engine preparation or decoder recipe must retain the relevant license and provenance materials. Model weights and binary packages have separate distribution requirements documented in [third-party notices](THIRD_PARTY_NOTICES.md).
