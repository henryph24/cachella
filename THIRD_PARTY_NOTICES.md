# Third-party notices

This inventory identifies major distribution boundaries. A complete dependency notice inventory remains a public binary-release gate.

| Component | Source and licensing record | Distribution notes |
| --- | --- | --- |
| Vendored lopdf | [Vendored license](vendor/lopdf/LICENSE), [manifest](vendor/lopdf/Cargo.toml) | MIT; retain the upstream license and identify local changes. |
| llama.cpp engine | [Pinned license](scripts/licenses/llama.cpp-LICENSE.txt), [preparation script](scripts/prepare_engine.py) | MIT; preparation packages pinned upstream engine resources and notices. |
| FFmpeg and x264 decoder | [Source recipe](scripts/prepare_decoder.py), [decoder report](evaluation/decoders/README.md) | The recipe enables GPL components. Preserve accompanying licenses, corresponding source archives and rebuild materials when distributing these executables. |
| Windows decoder zlib | [Source recipe](scripts/prepare_decoder.py) | The Windows decoder adds pinned zlib source and its upstream notices. |
| Model weights and projectors | [Catalog](src-tauri/src/models.rs), [model provenance](evaluation/model-provenance.json), [embedding provenance](evaluation/multimodal/provenance.json) | Downloaded separately; verify the publisher's terms for each exact model revision and redistribution scenario. |
| Rust and JavaScript dependencies | [Cargo manifest](src-tauri/Cargo.toml), [Cargo lockfile](src-tauri/Cargo.lock), [npm manifest](package.json), [npm lockfile](package-lock.json) | Each dependency retains its own license. Review the complete resolved dependency inventory before publishing binaries. |

Cachella's eventual source license does not replace upstream licenses. The current documentation is an engineering inventory; complete distribution review remains pending.
