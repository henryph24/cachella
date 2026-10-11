# Changelog

## 0.1.0 source alpha candidate (unreleased)

Cachella is an experimental local desktop knowledge workspace for Apple silicon
macOS and x86-64 Windows. This candidate is intended for developers and invited
testers. No public installer or source release has been published.

### Included

- Local import of text, Markdown, selectable-text PDF, images, audio and video.
- SQLite keyword search and persistent local multimodal semantic indexing.
- Cited answers through quantized local models, with separately downloaded weights.
- Bundled llama.cpp and a decoder built from pinned sources.
- Cancellable model downloads, indexing and answers; optional one-model residency.
- Native macOS evidence and six passing hosted Windows runtime qualification gates.

### Limits

- Notion, Drive, watched Obsidian vaults and Composio integration remain planned.
- Prompt reuse uses llama.cpp. Enterprise LMCache integration remains planned.
- OCR, continuous video understanding and application-level storage encryption are absent.
- Consumer RAM requirements, Windows 11 user flows and GPU backends remain unqualified.
- Installer signing, macOS notarization and clean-device installation remain open.
- Existing quality observations use synthetic fixtures and small evaluation rubrics.

See the [alpha release notes](docs/alpha-release.md) for setup, evidence and
publication requirements, and [third-party notices](THIRD_PARTY_NOTICES.md) for
separate dependency and model terms.
