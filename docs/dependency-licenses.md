# Source-alpha dependency license inventory

The locked graph contains 566 third-party Cargo packages and 210 npm package instances; 0 have missing license declarations and 0 have any missing recorded metadata. There are 8 additional distribution boundaries.

This is an engineering inventory of the source alpha. Legal clearance, verification of every upstream license/NOTICE text, model publisher terms and the final per-platform binary notice bundle remain open. Cachella's own source uses MIT; upstream licenses remain separate.

## Reproduce

```sh
python3 scripts/dependency_inventory.py --cargo /opt/homebrew/bin/cargo
python3 scripts/dependency_inventory.py --cargo /opt/homebrew/bin/cargo --check
python3 -m unittest discover -s scripts -p test_dependency_inventory.py -v
```

The generator uses Python 3.11+, the npm v3 lockfile and locked offline Cargo metadata with `desktop` enabled. It applies no platform filter, preserving Windows, Unix and other resolved target dependencies. Use `--allow-network` only when a locked package is unavailable locally. The JSON records exact sources, versions, checksums/integrity, npm instances and flags, Cargo incoming edge kinds and target conditions, and input hashes.

Cargo runtime/build/development labels describe graph reachability. Normal dependencies of a build dependency inherit build context; proc-macros can execute during compilation despite normal edges. npm development labels include build tooling; the lockfile cannot establish which code enters a frontend bundle. All-target reachability does not establish binary inclusion.

License expressions and legacy declaration objects are preserved verbatim as metadata. An `OR` expression records available alternatives; this report makes no selection. Missing metadata is surfaced explicitly. No license combination is automatically labelled compatible.

## Separate distribution boundaries

| Material | Pinned version/revision | License evidence | Distribution and pending review |
| --- | --- | --- | --- |
| SQLite amalgamation | 3.46.0 | public-domain dedication (upstream source statement); rusqlite bundled feature in src-tauri/Cargo.toml; header version and copyright disclaimer in the locked libsqlite3-sys package | Linked into the native application; the libsqlite3-sys MIT declaration covers the Rust wrapper separately. Check any additional native code activated by final target/features and collect its notices. |
| [llama.cpp engine](https://github.com/ggml-org/llama.cpp/commit/8b543610254aafe121942eaf79a23c3847177270) | b11531 | MIT; scripts/licenses/llama.cpp-LICENSE.txt and scripts/prepare_engine.py | Separately prepared command-line engine and libraries bundled by the desktop engine resource configuration. Audit all code and libraries inside each upstream binary archive and complete a target-specific notice bundle. |
| [FFmpeg decoder](https://ffmpeg.org/releases/ffmpeg-9.0.2.tar.xz) | 9.0.2 | GPL-2.0-or-later; scripts/prepare_decoder.py: pinned sources, configure flags and generated README declaration | Separate ffmpeg/ffprobe executables; recipe enables GPL and libx264 and copies COPYING/ LICENSE materials, complete source archives and rebuild scripts. Verify exact upstream license texts and redistribution obligations for the final built decoder; zlib license text is not classified by this generator. |
| [x264](https://code.videolan.org/videolan/x264/-/archive/0480cb05fa188d37ae87e8f4fd8f1aea3711f7ee/x264-0480cb05fa188d37ae87e8f4fd8f1aea3711f7ee.tar.bz2) | 0480cb05fa188d37ae87e8f4fd8f1aea3711f7ee | GPL-2.0-or-later; scripts/prepare_decoder.py: pinned sources, configure flags and generated README declaration | Statically linked into the prepared FFmpeg executables. Verify exact upstream license texts and redistribution obligations for the final built decoder; zlib license text is not classified by this generator. |
| [zlib decoder dependency](https://zlib.net/zlib-1.3.2.tar.xz) | 1.3.2 | MISSING; scripts/prepare_decoder.py: pinned sources, configure flags and generated README declaration | Windows decoder links pinned zlib statically; macOS uses SDK-supplied zlib. Verify exact upstream license texts and redistribution obligations for the final built decoder; zlib license text is not classified by this generator. |
| [Qwen3 1.7B GGUF](https://huggingface.co/unsloth/Qwen3-1.7B-GGUF/tree/d7f544eead698dbd1f15126ef60b45a1e1933222) | d7f544eead698dbd1f15126ef60b45a1e1933222 | Apache-2.0 (local catalog declaration; publisher terms unverified); src-tauri/src/models.rs catalog; pinned third-party GGUF conversions | Downloaded separately on user request; model weights and projectors have their own distribution terms. Verify publisher and base-model terms at this exact revision, conversion provenance and notices before hosting or redistributing weights. |
| [Gemma 4 E2B GGUF and projector](https://huggingface.co/ggml-org/gemma-4-E2B-it-GGUF/tree/b4243c156154b6dca9324415f8c7ccc098b4aed1) | b4243c156154b6dca9324415f8c7ccc098b4aed1 | Apache-2.0 (local catalog declaration; publisher terms unverified); src-tauri/src/models.rs catalog; pinned third-party GGUF conversions | Downloaded separately on user request; model weights and projectors have their own distribution terms. Verify publisher and base-model terms at this exact revision, conversion provenance and notices before hosting or redistributing weights. |
| [EmbeddingGemma 2 GGUF and projector](https://huggingface.co/ggml-org/embeddinggemma-2-GGUF/tree/bfcd298762cc34d0357ece5ebdd31791a3a374d8) | bfcd298762cc34d0357ece5ebdd31791a3a374d8 | Apache-2.0 (local catalog declaration; publisher terms unverified); src-tauri/src/models.rs catalog; pinned third-party GGUF conversions | Downloaded separately on user request; model weights and projectors have their own distribution terms. Verify publisher and base-model terms at this exact revision, conversion provenance and notices before hosting or redistributing weights. |

The vendored `lopdf` package is included in the Cargo inventory with its local path and MIT declaration. Preserve [its license](../vendor/lopdf/LICENSE) and [patch provenance](../vendor/lopdf/CACHELLA_PATCH.md) when distributing the patched source.

## Missing package metadata

No resolved third-party package lacks the recorded name/version/source/license fields. This result validates declarations only.

## Cargo resolved packages (566)

Exact source and integrity records are in [dependency-inventory.json](dependency-inventory.json). Cargo source labels below identify registry or local path; npm rows preserve every lockfile installation instance.

| Package | Version | Declared license | Contexts | Source/instance |
| --- | --- | --- | --- | --- |
| adler2 | 2.0.1 | 0BSD OR MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| aes | 0.9.3 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| ahash | 0.8.12 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| aho-corasick | 1.1.5 | Unlicense OR MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| alloc-no-stdlib | 2.0.4 | BSD-3-Clause | runtime | registry+https://github.com/rust-lang/crates.io-index |
| alloc-no-stdlib | 3.0.0 | BSD-3-Clause | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| alloc-stdlib | 0.2.4 | BSD-3-Clause | runtime | registry+https://github.com/rust-lang/crates.io-index |
| alloc-stdlib | 0.3.0 | BSD-3-Clause | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| ambient-authority | 0.0.2 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| android_system_properties | 0.1.6 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| anyhow | 1.0.104 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| ashpd | 0.11.1 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| asn1-rs | 0.7.2 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| asn1-rs-derive | 0.6.0 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| asn1-rs-impl | 0.2.0 | MIT/Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| async-broadcast | 0.7.2 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| async-channel | 2.5.0 | Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| async-executor | 1.14.0 | Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| async-fs | 2.2.0 | Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| async-io | 2.6.0 | Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| async-lock | 3.4.2 | Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| async-net | 2.0.0 | Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| async-process | 2.5.0 | Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| async-recursion | 1.2.0 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| async-signal | 0.2.14 | Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| async-task | 4.7.1 | Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| async-trait | 0.1.92 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| atk | 0.18.2 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| atk-sys | 0.18.2 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| atomic-waker | 1.1.2 | Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| autocfg | 1.5.1 | Apache-2.0 OR MIT | build | registry+https://github.com/rust-lang/crates.io-index |
| base64 | 0.21.7 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| base64 | 0.22.1 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| base64 | 0.23.1 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| bit-set | 0.8.0 | Apache-2.0 OR MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| bit-vec | 0.8.0 | Apache-2.0 OR MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| bit-vec | 0.9.1 | Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| bitflags | 1.3.2 | MIT/Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| bitflags | 2.13.2 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| block-buffer | 0.10.4 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| block-buffer | 0.12.1 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| block-padding | 0.4.2 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| block2 | 0.6.2 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| blocking | 1.7.0 | Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| brotli | 9.0.0 | BSD-3-Clause AND MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| brotli-decompressor | 5.0.3 | BSD-3-Clause/MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| brotli-decompressor | 6.0.1 | BSD-3-Clause/MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| bs58 | 0.5.1 | MIT/Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| bumpalo | 3.20.3 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| bytemuck | 1.25.2 | Zlib OR Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| byteorder | 1.5.0 | Unlicense OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| bytes | 1.12.1 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| cairo-rs | 0.18.5 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| cairo-sys-rs | 0.18.2 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| camino | 1.2.6 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| cap-fs-ext | 3.4.6 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| cap-primitives | 3.4.6 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| cap-std | 3.4.6 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| cargo-platform | 0.1.9 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| cargo_metadata | 0.19.2 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| cargo_toml | 1.0.1 | Apache-2.0 OR MIT | build | registry+https://github.com/rust-lang/crates.io-index |
| cbc | 0.2.1 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| cc | 1.6.0 | MIT OR Apache-2.0 | build | registry+https://github.com/rust-lang/crates.io-index |
| cesu8 | 1.1.0 | Apache-2.0/MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| cfb | 0.14.0 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| cfg-expr | 0.15.8 | MIT OR Apache-2.0 | build | registry+https://github.com/rust-lang/crates.io-index |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| cfg_aliases | 0.2.2 | MIT | build | registry+https://github.com/rust-lang/crates.io-index |
| chacha20 | 0.10.2 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| chrono | 0.4.45 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| cipher | 0.5.2 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| combine | 4.6.8 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| concurrent-queue | 2.5.0 | Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| const-oid | 0.10.2 | Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| cookie | 0.18.2 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| core-foundation | 0.10.1 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| core-foundation-sys | 0.8.7 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| core-graphics | 0.25.0 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| core-graphics-types | 0.2.0 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| core_detect | 1.0.0 | MIT/Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| cpubits | 0.1.1 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| cpufeatures | 0.2.17 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| cpufeatures | 0.3.1 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| crc32fast | 1.5.2 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| crossbeam-channel | 0.5.17 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| crossbeam-utils | 0.8.23 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| crypto-common | 0.1.7 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| crypto-common | 0.2.2 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| cssparser | 0.37.0 | MPL-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| cssparser-macros | 0.7.1 | MPL-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| ctor | 1.0.13 | Apache-2.0 OR MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| darling | 0.24.1 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| darling_core | 0.24.1 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| darling_macro | 0.24.1 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| data-encoding | 2.11.1 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| dbus | 0.9.12 | Apache-2.0/MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| defmt | 1.1.1 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| defmt-macros | 1.1.1 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| defmt-parser | 1.0.0 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| der-parser | 10.0.0 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| deranged | 0.5.8 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| derive_more | 2.1.1 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| derive_more-impl | 2.1.1 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| digest | 0.10.7 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| digest | 0.11.3 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| dirs | 7.0.0 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| dirs-sys | 0.5.0 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| dispatch2 | 0.3.1 | Zlib OR Apache-2.0 OR MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| displaydoc | 0.2.7 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| dlib | 0.5.3 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| dlopen2 | 0.8.2 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| dlopen2_derive | 0.4.3 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| dom_query | 0.28.0 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| downcast-rs | 1.2.1 | MIT/Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| dpi | 0.1.2 | Apache-2.0 AND MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| dtoa | 1.0.11 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| dtoa-short | 0.3.5 | MPL-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| dunce | 1.0.5 | CC0-1.0 OR MIT-0 OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| dyn-clone | 1.0.20 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| ecb | 0.2.1 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| embed-resource | 3.0.12 | MIT | build | registry+https://github.com/rust-lang/crates.io-index |
| embed_plist | 1.2.2 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| encoding_rs | 0.8.42 | (Apache-2.0 OR MIT) AND BSD-3-Clause | runtime | registry+https://github.com/rust-lang/crates.io-index |
| endi | 1.1.1 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| enumflags2 | 0.7.12 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| enumflags2_derive | 0.7.12 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| equivalent | 1.0.2 | Apache-2.0 OR MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| erased-serde | 0.4.10 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| errno | 0.3.14 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| event-listener | 5.4.2 | Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| event-listener-strategy | 0.5.4 | Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| fallible-iterator | 0.3.0 | MIT/Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| fallible-streaming-iterator | 0.1.9 | MIT/Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| fastrand | 2.5.0 | Apache-2.0 OR MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| fdeflate | 0.3.7 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| field-offset | 0.3.6 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| find-msvc-tools | 0.1.14 | MIT OR Apache-2.0 | build | registry+https://github.com/rust-lang/crates.io-index |
| flate2 | 1.1.10 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| fnv | 1.0.7 | Apache-2.0 / MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| foldhash | 0.2.0 | Zlib | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| foreign-types | 0.5.0 | MIT/Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| foreign-types-macros | 0.2.4 | MIT/Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| foreign-types-shared | 0.3.1 | MIT/Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| form_urlencoded | 1.2.2 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| fs-set-times | 0.20.3 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| futures-channel | 0.3.34 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| futures-core | 0.3.34 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| futures-executor | 0.3.34 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| futures-io | 0.3.34 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| futures-lite | 2.6.1 | Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| futures-macro | 0.3.34 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| futures-sink | 0.3.34 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| futures-task | 0.3.34 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| futures-util | 0.3.34 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| gdk | 0.18.2 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| gdk-pixbuf | 0.18.5 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| gdk-pixbuf-sys | 0.18.0 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| gdk-sys | 0.18.2 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| gdkwayland-sys | 0.18.2 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| gdkx11 | 0.18.2 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| gdkx11-sys | 0.18.2 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| generic-array | 0.14.7 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| getrandom | 0.2.17 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| getrandom | 0.3.4 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| getrandom | 0.4.3 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| gio | 0.18.4 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| gio-sys | 0.18.1 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| glib | 0.18.5 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| glib-macros | 0.18.5 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| glib-sys | 0.18.1 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| glob | 0.3.4 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| gobject-sys | 0.18.0 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| gtk | 0.18.2 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| gtk-sys | 0.18.2 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| gtk3-macros | 0.18.2 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| hashbrown | 0.12.3 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| hashbrown | 0.14.5 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| hashbrown | 0.17.1 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| hashlink | 0.9.1 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| heck | 0.4.1 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| heck | 0.5.0 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| hermit-abi | 0.5.3 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| hex | 0.4.3 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| html5ever | 0.39.0 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| http | 1.5.0 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| http-body | 1.1.0 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| http-body-util | 0.1.5 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| httparse | 1.10.1 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| hybrid-array | 0.4.15 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| hyper | 1.12.0 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| hyper-rustls | 0.27.10 | Apache-2.0 OR ISC OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| hyper-util | 0.1.21 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| iana-time-zone | 0.1.65 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| iana-time-zone-haiku | 0.1.2 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| ico | 0.5.0 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| icu_collections | 2.3.0 | Unicode-3.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| icu_locale_core | 2.3.0 | Unicode-3.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| icu_normalizer | 2.3.0 | Unicode-3.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| icu_normalizer_data | 2.3.0 | Unicode-3.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| icu_properties | 2.3.0 | Unicode-3.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| icu_properties_data | 2.3.0 | Unicode-3.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| icu_provider | 2.3.1 | Unicode-3.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| ident_case | 1.0.1 | MIT/Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| idna | 1.1.0 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| idna_adapter | 1.2.2 | Apache-2.0 OR MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| indexmap | 1.9.3 | Apache-2.0 OR MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| indexmap | 2.14.2 | Apache-2.0 OR MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| infer | 0.22.0 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| inout | 0.2.2 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| io-extras | 0.18.4 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| io-lifetimes | 2.0.4 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| ipnet | 2.12.2 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| itoa | 1.0.18 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| javascriptcore-rs | 1.1.2 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| javascriptcore-rs-sys | 1.1.1 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| jiff | 0.2.38 | Unlicense OR MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| jiff-core | 0.1.1 | Unlicense OR MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| jiff-static | 0.2.38 | Unlicense OR MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| jiff-tzdb | 0.1.9 | Unlicense OR MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| jiff-tzdb-platform | 0.1.3 | Unlicense OR MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| jni | 0.21.1 | MIT/Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| jni-sys | 0.3.1 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| jni-sys | 0.4.1 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| jni-sys-macros | 0.4.1 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| js-sys | 0.3.106 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| json-patch | 4.2.0 | MIT/Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| jsonptr | 0.7.1 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| keyboard-types | 0.8.3 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| lazy_static | 1.5.1 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| libappindicator | 0.9.0 | Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| libappindicator-sys | 0.9.0 | Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| libc | 0.2.190 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| libdbus-sys | 0.2.7 | Apache-2.0/MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| libloading | 0.7.4 | ISC | runtime | registry+https://github.com/rust-lang/crates.io-index |
| libloading | 0.8.9 | ISC | runtime | registry+https://github.com/rust-lang/crates.io-index |
| libredox | 0.1.25 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| libsqlite3-sys | 0.30.1 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| linux-raw-sys | 0.12.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| litemap | 0.8.3 | Unicode-3.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| lock_api | 0.4.14 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| log | 0.4.34 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| lopdf | 0.45.0 | MIT | runtime | path:vendor/lopdf |
| lru-slab | 0.1.3 | MIT OR Apache-2.0 OR Zlib | runtime | registry+https://github.com/rust-lang/crates.io-index |
| markup5ever | 0.39.0 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| maybe-owned | 0.3.4 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| md-5 | 0.11.0 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| memchr | 2.8.3 | Unlicense OR MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| memoffset | 0.9.1 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| mime | 0.3.17 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| minimal-lexical | 0.2.1 | MIT/Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| miniz_oxide | 0.8.9 | MIT OR Zlib OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| miniz_oxide | 0.9.1 | MIT OR Zlib OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| mio | 1.2.4 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| muda | 0.20.0 | Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| multiversion_no_op | 1.0.0 | Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| ndk | 0.9.0 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| ndk-context | 0.1.1 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| ndk-sys | 0.6.0+11769913 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| new_debug_unreachable | 1.0.6 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| nom | 7.1.3 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| nom | 8.0.0 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| num-bigint | 0.4.8 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| num-conv | 0.2.2 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| num-integer | 0.1.47 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| num-traits | 0.2.19 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| num_enum | 0.7.6 | BSD-3-Clause OR MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| num_enum_derive | 0.7.6 | BSD-3-Clause OR MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| objc2 | 0.6.5 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| objc2-app-kit | 0.3.2 | Zlib OR Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| objc2-cloud-kit | 0.3.2 | Zlib OR Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| objc2-core-data | 0.3.2 | Zlib OR Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| objc2-core-foundation | 0.3.2 | Zlib OR Apache-2.0 OR MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| objc2-core-graphics | 0.3.2 | Zlib OR Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| objc2-core-image | 0.3.2 | Zlib OR Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| objc2-core-location | 0.3.2 | Zlib OR Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| objc2-core-text | 0.3.2 | Zlib OR Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| objc2-encode | 4.1.0 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| objc2-exception-helper | 0.1.1 | Zlib OR Apache-2.0 OR MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| objc2-foundation | 0.3.2 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| objc2-io-surface | 0.3.2 | Zlib OR Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| objc2-quartz-core | 0.3.2 | Zlib OR Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| objc2-ui-kit | 0.3.2 | Zlib OR Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| objc2-user-notifications | 0.3.2 | Zlib OR Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| objc2-web-kit | 0.3.2 | Zlib OR Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| oid-registry | 0.8.1 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| once_cell | 1.21.4 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| option-ext | 0.2.0 | MPL-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| ordered-stream | 0.2.0 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| pango | 0.18.3 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| pango-sys | 0.18.0 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| parking | 2.2.1 | Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| parking_lot | 0.12.5 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| parking_lot_core | 0.9.12 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| pem | 4.0.0 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| percent-encoding | 2.3.2 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| phf | 0.13.1 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| phf_codegen | 0.13.1 | MIT | build | registry+https://github.com/rust-lang/crates.io-index |
| phf_generator | 0.13.1 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| phf_macros | 0.13.1 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| phf_shared | 0.13.1 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| pin-project-lite | 0.2.17 | Apache-2.0 OR MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| piper | 0.2.5 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| pkg-config | 0.3.34 | MIT OR Apache-2.0 | build | registry+https://github.com/rust-lang/crates.io-index |
| plist | 1.10.1 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| png | 0.17.16 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| png | 0.18.1 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| polling | 3.11.0 | Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| pollster | 0.4.0 | Apache-2.0/MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| portable-atomic | 1.15.0 | Apache-2.0 OR MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| portable-atomic-util | 0.2.8 | Apache-2.0 OR MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| potential_utf | 0.1.6 | Unicode-3.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| powerfmt | 0.2.1 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| ppv-lite86 | 0.2.21 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| precomputed-hash | 0.1.1 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| proc-macro-crate | 1.3.1 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| proc-macro-crate | 2.0.2 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| proc-macro-crate | 3.5.0 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| proc-macro-error | 1.0.4 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| proc-macro-error-attr | 1.0.4 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| quick-xml | 0.41.0 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| quick-xml | 0.42.0 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| quinn | 0.11.12 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| quinn-proto | 0.11.19 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| quinn-udp | 0.5.16 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| quote | 1.0.47 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| r-efi | 5.3.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | runtime | registry+https://github.com/rust-lang/crates.io-index |
| r-efi | 6.0.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| rand | 0.10.3 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| rand | 0.9.5 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| rand_chacha | 0.9.0 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| rand_core | 0.10.1 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| rand_core | 0.9.5 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| rand_pcg | 0.10.2 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| rangemap | 1.8.0 | MIT/Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| raw-window-handle | 0.6.2 | MIT OR Apache-2.0 OR Zlib | runtime | registry+https://github.com/rust-lang/crates.io-index |
| rcgen | 0.14.10 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| redox_syscall | 0.5.18 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| redox_users | 0.5.3 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| ref-cast | 1.0.27 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| ref-cast-impl | 1.0.27 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| regex | 1.13.1 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| regex-automata | 0.4.18 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| regex-syntax | 0.8.11 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| reqwest | 0.12.28 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| reqwest | 0.13.5 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| rfd | 0.15.4 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| ring | 0.17.14 | Apache-2.0 AND ISC | runtime | registry+https://github.com/rust-lang/crates.io-index |
| rusqlite | 0.32.1 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| rustc-hash | 2.1.3 | Apache-2.0 OR MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| rustc_version | 0.4.1 | MIT OR Apache-2.0 | build | registry+https://github.com/rust-lang/crates.io-index |
| rusticata-macros | 4.1.0 | MIT/Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| rustix | 1.1.5 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| rustix-linux-procfs | 0.1.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| rustls | 0.23.45 | Apache-2.0 OR ISC OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| rustls-pki-types | 1.15.1 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| rustls-webpki | 0.103.15 | ISC | runtime | registry+https://github.com/rust-lang/crates.io-index |
| rustversion | 1.0.23 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| ryu | 1.0.23 | Apache-2.0 OR BSL-1.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| same-file | 1.0.6 | Unlicense/MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| schemars | 0.8.22 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| schemars | 0.9.0 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| schemars | 1.2.2 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| schemars_derive | 0.8.22 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| scoped-tls | 1.0.1 | MIT/Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| scopeguard | 1.2.0 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| selectors | 0.38.0 | MPL-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| semver | 1.0.28 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| serde | 1.0.229 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| serde-untagged | 0.1.9 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| serde_core | 1.0.229 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| serde_derive | 1.0.229 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| serde_derive_internals | 0.29.1 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| serde_json | 1.0.151 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| serde_repr | 0.1.21 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| serde_spanned | 0.6.9 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| serde_spanned | 1.1.2 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| serde_urlencoded | 0.7.1 | MIT/Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| serde_with | 3.24.0 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| serde_with_macros | 3.24.0 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| serialize-to-javascript | 0.1.2 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| serialize-to-javascript-impl | 0.1.2 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| servo_arc | 0.4.3 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| sha2 | 0.10.9 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| sha2 | 0.11.0 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| shlex | 2.0.1 | MIT OR Apache-2.0 | build | registry+https://github.com/rust-lang/crates.io-index |
| signal-hook-registry | 1.4.8 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| simd-adler32 | 0.3.10 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| simdutf8 | 0.1.5 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| siphasher | 1.0.4 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| slab | 0.4.12 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| smallvec | 1.16.2 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| socket2 | 0.6.5 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| softbuffer | 0.4.8 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| soup3 | 0.5.0 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| soup3-sys | 0.5.0 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| stable_deref_trait | 1.2.1 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| string_cache | 0.9.0 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| string_cache_codegen | 0.6.1 | MIT OR Apache-2.0 | build | registry+https://github.com/rust-lang/crates.io-index |
| stringprep | 0.1.5 | MIT/Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| strsim | 0.11.1 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| subtle | 2.6.1 | BSD-3-Clause | runtime | registry+https://github.com/rust-lang/crates.io-index |
| swift-rs | 1.0.8 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| syn | 1.0.109 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| syn | 2.0.119 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| syn | 3.0.6 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| sync_wrapper | 1.0.2 | Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| synstructure | 0.13.2 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| synstructure | 0.14.0 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| system-deps | 6.2.2 | MIT OR Apache-2.0 | build | registry+https://github.com/rust-lang/crates.io-index |
| tao | 0.37.1 | Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| tao-macros | 0.1.4 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| target-lexicon | 0.12.16 | Apache-2.0 WITH LLVM-exception | build | registry+https://github.com/rust-lang/crates.io-index |
| tauri | 2.12.2 | Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| tauri-build | 2.7.1 | Apache-2.0 OR MIT | build | registry+https://github.com/rust-lang/crates.io-index |
| tauri-codegen | 2.7.1 | Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| tauri-macros | 2.7.1 | Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| tauri-runtime | 2.12.1 | Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| tauri-runtime-wry | 2.12.1 | Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| tauri-utils | 2.10.1 | Apache-2.0 OR MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| tauri-winres | 0.3.6 | MIT | build | registry+https://github.com/rust-lang/crates.io-index |
| tempfile | 3.27.0 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| tendril | 0.5.1 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| thiserror | 1.0.69 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| thiserror | 2.0.21 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| thiserror-impl | 1.0.69 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| thiserror-impl | 2.0.21 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| time | 0.3.55 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| time-core | 0.1.9 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| time-macros | 0.2.32 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| tinystr | 0.8.4 | Unicode-3.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| tinyvec | 1.13.3 | Zlib OR Apache-2.0 OR MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| tokio | 1.53.2 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| tokio-macros | 2.7.2 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| tokio-rustls | 0.26.6 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| tokio-util | 0.7.20 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| toml | 0.8.2 | MIT OR Apache-2.0 | build | registry+https://github.com/rust-lang/crates.io-index |
| toml | 1.1.8+spec-1.1.0 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| toml_datetime | 0.6.3 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| toml_datetime | 1.1.2+spec-1.1.0 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| toml_edit | 0.19.15 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| toml_edit | 0.20.2 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| toml_edit | 0.25.17+spec-1.1.0 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| toml_parser | 1.1.5+spec-1.1.0 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| toml_writer | 1.1.3+spec-1.1.0 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| tower | 0.5.3 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| tower-http | 0.6.11 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| tower-layer | 0.3.3 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| tower-service | 0.3.3 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| tracing | 0.1.44 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| tracing-attributes | 0.1.31 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| tracing-core | 0.1.36 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| tray-icon | 0.25.1 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| try-lock | 0.2.5 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| typeid | 1.0.3 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| typenum | 1.20.1 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| uds_windows | 1.2.1 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| unicode-bidi | 0.3.18 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| unicode-ident | 1.0.26 | (MIT OR Apache-2.0) AND Unicode-3.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| unicode-normalization | 0.1.25 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| unicode-properties | 0.1.4 | MIT/Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| unicode-segmentation | 1.13.3 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| untrusted | 0.9.0 | ISC | runtime | registry+https://github.com/rust-lang/crates.io-index |
| url | 2.5.8 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| urlencoding | 2.1.3 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| urlpattern | 0.6.0 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| utf8_iter | 1.0.4 | Apache-2.0 OR MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| uuid | 1.27.0 | Apache-2.0 OR MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| vcpkg | 0.2.15 | MIT/Apache-2.0 | build | registry+https://github.com/rust-lang/crates.io-index |
| version-compare | 0.2.1 | MIT | build | registry+https://github.com/rust-lang/crates.io-index |
| version_check | 0.9.5 | MIT/Apache-2.0 | build | registry+https://github.com/rust-lang/crates.io-index |
| vswhom | 0.1.0 | MIT | build | registry+https://github.com/rust-lang/crates.io-index |
| vswhom-sys | 0.1.3 | MIT | build | registry+https://github.com/rust-lang/crates.io-index |
| walkdir | 2.5.0 | Unlicense/MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| want | 0.3.2 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| wasi | 0.11.1+wasi-snapshot-preview1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| wasip2 | 1.0.4+wasi-0.2.12 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| wasm-bindgen | 0.2.129 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| wasm-bindgen-futures | 0.4.79 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| wasm-bindgen-macro | 0.2.129 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| wasm-bindgen-macro-support | 0.2.129 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| wasm-bindgen-shared | 0.2.129 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| wasm-streams | 0.5.0 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| wayland-backend | 0.3.17 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| wayland-client | 0.31.15 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| wayland-protocols | 0.32.13 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| wayland-scanner | 0.31.11 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| wayland-sys | 0.31.11 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| web-sys | 0.3.106 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| web-time | 1.1.0 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| web_atoms | 0.2.6 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| webkit2gtk | 2.0.2 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| webkit2gtk-sys | 2.0.2 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| webpki-roots | 1.0.9 | CDLA-Permissive-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| webview2-com | 0.39.1 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| webview2-com-macros | 0.8.1 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| webview2-com-sys | 0.39.1 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| weezl | 0.2.1 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| winapi | 0.3.9 | MIT/Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| winapi-i686-pc-windows-gnu | 0.4.0 | MIT/Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| winapi-util | 0.1.11 | Unlicense OR MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| winapi-x86_64-pc-windows-gnu | 0.4.0 | MIT/Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| window-vibrancy | 0.8.1 | Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| windows | 0.62.2 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| windows-collections | 0.3.2 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| windows-core | 0.62.2 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| windows-future | 0.3.2 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| windows-implement | 0.60.2 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| windows-interface | 0.59.3 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| windows-link | 0.2.1 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| windows-numerics | 0.3.1 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| windows-result | 0.4.1 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| windows-strings | 0.5.1 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| windows-sys | 0.45.0 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| windows-sys | 0.52.0 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| windows-sys | 0.59.0 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| windows-sys | 0.61.2 | MIT OR Apache-2.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| windows-targets | 0.42.2 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| windows-targets | 0.52.6 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| windows-threading | 0.2.1 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| windows-version | 0.1.7 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| windows_aarch64_gnullvm | 0.42.2 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| windows_aarch64_gnullvm | 0.52.6 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| windows_aarch64_msvc | 0.42.2 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| windows_aarch64_msvc | 0.52.6 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| windows_i686_gnu | 0.42.2 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| windows_i686_gnu | 0.52.6 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| windows_i686_gnullvm | 0.52.6 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| windows_i686_msvc | 0.42.2 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| windows_i686_msvc | 0.52.6 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| windows_x86_64_gnu | 0.42.2 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| windows_x86_64_gnu | 0.52.6 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| windows_x86_64_gnullvm | 0.42.2 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| windows_x86_64_gnullvm | 0.52.6 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| windows_x86_64_msvc | 0.42.2 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| windows_x86_64_msvc | 0.52.6 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| winnow | 0.5.40 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| winnow | 1.0.4 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| winreg | 0.56.0 | MIT | build | registry+https://github.com/rust-lang/crates.io-index |
| winx | 0.36.4 | Apache-2.0 WITH LLVM-exception | runtime | registry+https://github.com/rust-lang/crates.io-index |
| wit-bindgen | 0.57.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| writeable | 0.6.4 | Unicode-3.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| wry | 0.57.0 | Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| x11 | 2.21.0 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| x11-dl | 2.21.0 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| x509-parser | 0.18.1 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| yasna | 0.6.0 | MIT OR Apache-2.0 | runtime | registry+https://github.com/rust-lang/crates.io-index |
| yoke | 0.8.3 | Unicode-3.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| yoke-derive | 0.8.4 | Unicode-3.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| zbus | 5.19.0 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| zbus_macros | 5.19.0 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| zbus_names | 4.3.4 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| zcheapstr | 1.1.0 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| zerocopy | 0.8.62 | BSD-2-Clause OR Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| zerocopy-derive | 0.8.62 | BSD-2-Clause OR Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| zerofrom | 0.1.8 | Unicode-3.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| zerofrom-derive | 0.1.8 | Unicode-3.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| zeroize | 1.9.1 | Apache-2.0 OR MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| zerotrie | 0.2.5 | Unicode-3.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| zerovec | 0.11.8 | Unicode-3.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| zerovec-derive | 0.11.6 | Unicode-3.0 | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| zlib-rs | 0.6.8 | Zlib | runtime | registry+https://github.com/rust-lang/crates.io-index |
| zmij | 1.0.23 | MIT | build, runtime | registry+https://github.com/rust-lang/crates.io-index |
| zvariant | 5.15.0 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| zvariant_derive | 5.15.0 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |
| zvariant_utils | 4.2.0 | MIT | runtime | registry+https://github.com/rust-lang/crates.io-index |

## Npm resolved packages (210)

Exact source and integrity records are in [dependency-inventory.json](dependency-inventory.json). Cargo source labels below identify registry or local path; npm rows preserve every lockfile installation instance.

| Package | Version | Declared license | Contexts | Source/instance |
| --- | --- | --- | --- | --- |
| @asamuzakjp/css-color | 5.1.11 | MIT | development | node_modules/@asamuzakjp/css-color |
| @asamuzakjp/dom-selector | 7.1.1 | MIT | development | node_modules/@asamuzakjp/dom-selector |
| @asamuzakjp/generational-cache | 1.0.1 | MIT | development | node_modules/@asamuzakjp/generational-cache |
| @asamuzakjp/nwsapi | 2.3.9 | MIT | development | node_modules/@asamuzakjp/nwsapi |
| @babel/code-frame | 7.29.7 | MIT | development | node_modules/@babel/code-frame |
| @babel/compat-data | 7.29.7 | MIT | development | node_modules/@babel/compat-data |
| @babel/core | 7.29.7 | MIT | development | node_modules/@babel/core |
| @babel/generator | 7.29.8 | MIT | development | node_modules/@babel/generator |
| @babel/helper-compilation-targets | 7.29.7 | MIT | development | node_modules/@babel/helper-compilation-targets |
| @babel/helper-globals | 7.29.7 | MIT | development | node_modules/@babel/helper-globals |
| @babel/helper-module-imports | 7.29.7 | MIT | development | node_modules/@babel/helper-module-imports |
| @babel/helper-module-transforms | 7.29.7 | MIT | development | node_modules/@babel/helper-module-transforms |
| @babel/helper-plugin-utils | 7.29.7 | MIT | development | node_modules/@babel/helper-plugin-utils |
| @babel/helper-string-parser | 7.29.7 | MIT | development | node_modules/@babel/helper-string-parser |
| @babel/helper-validator-identifier | 7.29.7 | MIT | development | node_modules/@babel/helper-validator-identifier |
| @babel/helper-validator-option | 7.29.7 | MIT | development | node_modules/@babel/helper-validator-option |
| @babel/helpers | 7.29.10 | MIT | development | node_modules/@babel/helpers |
| @babel/parser | 7.29.9 | MIT | development | node_modules/@babel/parser |
| @babel/plugin-transform-react-jsx-self | 7.29.7 | MIT | development | node_modules/@babel/plugin-transform-react-jsx-self |
| @babel/plugin-transform-react-jsx-source | 7.29.7 | MIT | development | node_modules/@babel/plugin-transform-react-jsx-source |
| @babel/runtime | 7.29.10 | MIT | development | node_modules/@babel/runtime |
| @babel/template | 7.29.7 | MIT | development | node_modules/@babel/template |
| @babel/traverse | 7.29.10 | MIT | development | node_modules/@babel/traverse |
| @babel/types | 7.29.8 | MIT | development | node_modules/@babel/types |
| @bramus/specificity | 2.4.3 | MIT | development | node_modules/@bramus/specificity |
| @csstools/color-helpers | 6.1.2 | MIT-0 | development | node_modules/@csstools/color-helpers |
| @csstools/css-calc | 3.4.3 | MIT | development | node_modules/@csstools/css-calc |
| @csstools/css-color-parser | 4.2.6 | MIT | development | node_modules/@csstools/css-color-parser |
| @csstools/css-parser-algorithms | 4.0.2 | MIT | development | node_modules/@csstools/css-parser-algorithms |
| @csstools/css-syntax-patches-for-csstree | 1.1.15 | MIT-0 | development | node_modules/@csstools/css-syntax-patches-for-csstree |
| @csstools/css-tokenizer | 4.0.2 | MIT | development | node_modules/@csstools/css-tokenizer |
| @esbuild/aix-ppc64 | 0.28.2 | MIT | development | node_modules/@esbuild/aix-ppc64 |
| @esbuild/android-arm | 0.28.2 | MIT | development | node_modules/@esbuild/android-arm |
| @esbuild/android-arm64 | 0.28.2 | MIT | development | node_modules/@esbuild/android-arm64 |
| @esbuild/android-x64 | 0.28.2 | MIT | development | node_modules/@esbuild/android-x64 |
| @esbuild/darwin-arm64 | 0.28.2 | MIT | development | node_modules/@esbuild/darwin-arm64 |
| @esbuild/darwin-x64 | 0.28.2 | MIT | development | node_modules/@esbuild/darwin-x64 |
| @esbuild/freebsd-arm64 | 0.28.2 | MIT | development | node_modules/@esbuild/freebsd-arm64 |
| @esbuild/freebsd-x64 | 0.28.2 | MIT | development | node_modules/@esbuild/freebsd-x64 |
| @esbuild/linux-arm | 0.28.2 | MIT | development | node_modules/@esbuild/linux-arm |
| @esbuild/linux-arm64 | 0.28.2 | MIT | development | node_modules/@esbuild/linux-arm64 |
| @esbuild/linux-ia32 | 0.28.2 | MIT | development | node_modules/@esbuild/linux-ia32 |
| @esbuild/linux-loong64 | 0.28.2 | MIT | development | node_modules/@esbuild/linux-loong64 |
| @esbuild/linux-mips64el | 0.28.2 | MIT | development | node_modules/@esbuild/linux-mips64el |
| @esbuild/linux-ppc64 | 0.28.2 | MIT | development | node_modules/@esbuild/linux-ppc64 |
| @esbuild/linux-riscv64 | 0.28.2 | MIT | development | node_modules/@esbuild/linux-riscv64 |
| @esbuild/linux-s390x | 0.28.2 | MIT | development | node_modules/@esbuild/linux-s390x |
| @esbuild/linux-x64 | 0.28.2 | MIT | development | node_modules/@esbuild/linux-x64 |
| @esbuild/netbsd-arm64 | 0.28.2 | MIT | development | node_modules/@esbuild/netbsd-arm64 |
| @esbuild/netbsd-x64 | 0.28.2 | MIT | development | node_modules/@esbuild/netbsd-x64 |
| @esbuild/openbsd-arm64 | 0.28.2 | MIT | development | node_modules/@esbuild/openbsd-arm64 |
| @esbuild/openbsd-x64 | 0.28.2 | MIT | development | node_modules/@esbuild/openbsd-x64 |
| @esbuild/openharmony-arm64 | 0.28.2 | MIT | development | node_modules/@esbuild/openharmony-arm64 |
| @esbuild/sunos-x64 | 0.28.2 | MIT | development | node_modules/@esbuild/sunos-x64 |
| @esbuild/win32-arm64 | 0.28.2 | MIT | development | node_modules/@esbuild/win32-arm64 |
| @esbuild/win32-ia32 | 0.28.2 | MIT | development | node_modules/@esbuild/win32-ia32 |
| @esbuild/win32-x64 | 0.28.2 | MIT | development | node_modules/@esbuild/win32-x64 |
| @exodus/bytes | 1.16.0 | MIT | development | node_modules/@exodus/bytes |
| @jridgewell/gen-mapping | 0.3.13 | MIT | development | node_modules/@jridgewell/gen-mapping |
| @jridgewell/remapping | 2.3.5 | MIT | development | node_modules/@jridgewell/remapping |
| @jridgewell/resolve-uri | 3.1.2 | MIT | development | node_modules/@jridgewell/resolve-uri |
| @jridgewell/sourcemap-codec | 1.6.0 | MIT | development | node_modules/@jridgewell/sourcemap-codec |
| @jridgewell/trace-mapping | 0.3.31 | MIT | development | node_modules/@jridgewell/trace-mapping |
| @napi-rs/lzma-linux-x64-gnu | 1.5.1 | MIT | development | node_modules/@napi-rs/lzma-linux-x64-gnu |
| @rolldown/pluginutils | 1.0.0-rc.3 | MIT | development | node_modules/@rolldown/pluginutils |
| @rollup/rollup-android-arm-eabi | 4.64.3 | MIT | development | node_modules/@rollup/rollup-android-arm-eabi |
| @rollup/rollup-android-arm64 | 4.64.3 | MIT | development | node_modules/@rollup/rollup-android-arm64 |
| @rollup/rollup-darwin-arm64 | 4.64.3 | MIT | development | node_modules/@rollup/rollup-darwin-arm64 |
| @rollup/rollup-darwin-x64 | 4.64.3 | MIT | development | node_modules/@rollup/rollup-darwin-x64 |
| @rollup/rollup-freebsd-arm64 | 4.64.3 | MIT | development | node_modules/@rollup/rollup-freebsd-arm64 |
| @rollup/rollup-freebsd-x64 | 4.64.3 | MIT | development | node_modules/@rollup/rollup-freebsd-x64 |
| @rollup/rollup-linux-arm-gnueabihf | 4.64.3 | MIT | development | node_modules/@rollup/rollup-linux-arm-gnueabihf |
| @rollup/rollup-linux-arm-musleabihf | 4.64.3 | MIT | development | node_modules/@rollup/rollup-linux-arm-musleabihf |
| @rollup/rollup-linux-arm64-gnu | 4.64.3 | MIT | development | node_modules/@rollup/rollup-linux-arm64-gnu |
| @rollup/rollup-linux-arm64-musl | 4.64.3 | MIT | development | node_modules/@rollup/rollup-linux-arm64-musl |
| @rollup/rollup-linux-loong64-gnu | 4.64.3 | MIT | development | node_modules/@rollup/rollup-linux-loong64-gnu |
| @rollup/rollup-linux-loong64-musl | 4.64.3 | MIT | development | node_modules/@rollup/rollup-linux-loong64-musl |
| @rollup/rollup-linux-ppc64-gnu | 4.64.3 | MIT | development | node_modules/@rollup/rollup-linux-ppc64-gnu |
| @rollup/rollup-linux-ppc64-musl | 4.64.3 | MIT | development | node_modules/@rollup/rollup-linux-ppc64-musl |
| @rollup/rollup-linux-riscv64-gnu | 4.64.3 | MIT | development | node_modules/@rollup/rollup-linux-riscv64-gnu |
| @rollup/rollup-linux-riscv64-musl | 4.64.3 | MIT | development | node_modules/@rollup/rollup-linux-riscv64-musl |
| @rollup/rollup-linux-s390x-gnu | 4.64.3 | MIT | development | node_modules/@rollup/rollup-linux-s390x-gnu |
| @rollup/rollup-linux-x64-gnu | 4.64.3 | MIT | development | node_modules/@rollup/rollup-linux-x64-gnu |
| @rollup/rollup-linux-x64-musl | 4.64.3 | MIT | development | node_modules/@rollup/rollup-linux-x64-musl |
| @rollup/rollup-openbsd-x64 | 4.64.3 | MIT | development | node_modules/@rollup/rollup-openbsd-x64 |
| @rollup/rollup-openharmony-arm64 | 4.64.3 | MIT | development | node_modules/@rollup/rollup-openharmony-arm64 |
| @rollup/rollup-win32-arm64-msvc | 4.64.3 | MIT | development | node_modules/@rollup/rollup-win32-arm64-msvc |
| @rollup/rollup-win32-ia32-msvc | 4.64.3 | MIT | development | node_modules/@rollup/rollup-win32-ia32-msvc |
| @rollup/rollup-win32-x64-gnu | 4.64.3 | MIT | development | node_modules/@rollup/rollup-win32-x64-gnu |
| @rollup/rollup-win32-x64-msvc | 4.64.3 | MIT | development | node_modules/@rollup/rollup-win32-x64-msvc |
| @standard-schema/spec | 1.1.0 | MIT | development | node_modules/@standard-schema/spec |
| @tauri-apps/api | 2.12.2 | Apache-2.0 OR MIT | runtime | node_modules/@tauri-apps/api |
| @tauri-apps/cli | 2.12.1 | Apache-2.0 OR MIT | development | node_modules/@tauri-apps/cli |
| @tauri-apps/cli-darwin-arm64 | 2.12.1 | Apache-2.0 OR MIT | development | node_modules/@tauri-apps/cli-darwin-arm64 |
| @tauri-apps/cli-darwin-x64 | 2.12.1 | Apache-2.0 OR MIT | development | node_modules/@tauri-apps/cli-darwin-x64 |
| @tauri-apps/cli-linux-arm-gnueabihf | 2.12.1 | Apache-2.0 OR MIT | development | node_modules/@tauri-apps/cli-linux-arm-gnueabihf |
| @tauri-apps/cli-linux-arm64-gnu | 2.12.1 | Apache-2.0 OR MIT | development | node_modules/@tauri-apps/cli-linux-arm64-gnu |
| @tauri-apps/cli-linux-arm64-musl | 2.12.1 | Apache-2.0 OR MIT | development | node_modules/@tauri-apps/cli-linux-arm64-musl |
| @tauri-apps/cli-linux-riscv64-gnu | 2.12.1 | Apache-2.0 OR MIT | development | node_modules/@tauri-apps/cli-linux-riscv64-gnu |
| @tauri-apps/cli-linux-x64-gnu | 2.12.1 | Apache-2.0 OR MIT | development | node_modules/@tauri-apps/cli-linux-x64-gnu |
| @tauri-apps/cli-linux-x64-musl | 2.12.1 | Apache-2.0 OR MIT | development | node_modules/@tauri-apps/cli-linux-x64-musl |
| @tauri-apps/cli-win32-arm64-msvc | 2.12.1 | Apache-2.0 OR MIT | development | node_modules/@tauri-apps/cli-win32-arm64-msvc |
| @tauri-apps/cli-win32-ia32-msvc | 2.12.1 | Apache-2.0 OR MIT | development | node_modules/@tauri-apps/cli-win32-ia32-msvc |
| @tauri-apps/cli-win32-x64-msvc | 2.12.1 | Apache-2.0 OR MIT | development | node_modules/@tauri-apps/cli-win32-x64-msvc |
| @testing-library/dom | 10.4.2 | MIT | development | node_modules/@testing-library/dom |
| @testing-library/react | 16.3.3 | MIT | development | node_modules/@testing-library/react |
| @testing-library/user-event | 14.6.7 | MIT | development | node_modules/@testing-library/user-event |
| @types/aria-query | 5.0.4 | MIT | development | node_modules/@types/aria-query |
| @types/babel__core | 7.20.5 | MIT | development | node_modules/@types/babel__core |
| @types/babel__generator | 7.27.0 | MIT | development | node_modules/@types/babel__generator |
| @types/babel__template | 7.4.4 | MIT | development | node_modules/@types/babel__template |
| @types/babel__traverse | 7.28.0 | MIT | development | node_modules/@types/babel__traverse |
| @types/chai | 5.2.3 | MIT | development | node_modules/@types/chai |
| @types/deep-eql | 4.0.2 | MIT | development | node_modules/@types/deep-eql |
| @types/estree | 1.0.9 | MIT | development | node_modules/@types/estree |
| @types/react | 19.3.0 | MIT | development | node_modules/@types/react |
| @types/react-dom | 19.3.0 | MIT | development | node_modules/@types/react-dom |
| @vitejs/plugin-react | 5.2.0 | MIT | development | node_modules/@vitejs/plugin-react |
| @vitest/expect | 4.1.11 | MIT | development | node_modules/@vitest/expect |
| @vitest/mocker | 4.1.11 | MIT | development | node_modules/@vitest/mocker |
| @vitest/pretty-format | 4.1.11 | MIT | development | node_modules/@vitest/pretty-format |
| @vitest/runner | 4.1.11 | MIT | development | node_modules/@vitest/runner |
| @vitest/snapshot | 4.1.11 | MIT | development | node_modules/@vitest/snapshot |
| @vitest/spy | 4.1.11 | MIT | development | node_modules/@vitest/spy |
| @vitest/utils | 4.1.11 | MIT | development | node_modules/@vitest/utils |
| ansi-regex | 5.0.1 | MIT | development | node_modules/ansi-regex |
| ansi-styles | 5.2.0 | MIT | development | node_modules/ansi-styles |
| aria-query | 5.3.0 | Apache-2.0 | development | node_modules/aria-query |
| assertion-error | 2.0.1 | MIT | development | node_modules/assertion-error |
| baseline-browser-mapping | 2.11.28 | Apache-2.0 | development | node_modules/baseline-browser-mapping |
| bidi-js | 1.1.0 | MIT | development | node_modules/bidi-js |
| browserslist | 4.29.3 | MIT | development | node_modules/browserslist |
| caniuse-lite | 1.0.30001815 | CC-BY-4.0 | development | node_modules/caniuse-lite |
| chai | 6.3.0 | MIT | development | node_modules/chai |
| convert-source-map | 2.0.0 | MIT | development | node_modules/convert-source-map |
| css-tree | 3.2.1 | MIT | development | node_modules/css-tree |
| csstype | 3.2.3 | MIT | development | node_modules/csstype |
| data-urls | 7.0.0 | MIT | development | node_modules/data-urls |
| debug | 4.4.3 | MIT | development | node_modules/debug |
| decimal.js | 10.6.0 | MIT | development | node_modules/decimal.js |
| dequal | 2.0.3 | MIT | development | node_modules/dequal |
| dom-accessibility-api | 0.5.16 | MIT | development | node_modules/dom-accessibility-api |
| electron-to-chromium | 1.5.452 | ISC | development | node_modules/electron-to-chromium |
| entities | 8.1.0 | BSD-2-Clause | development | node_modules/entities |
| es-module-lexer | 2.3.2 | MIT | development | node_modules/es-module-lexer |
| esbuild | 0.28.2 | MIT | development | node_modules/esbuild |
| escalade | 3.2.0 | MIT | development | node_modules/escalade |
| estree-walker | 3.0.3 | MIT | development | node_modules/estree-walker |
| expect-type | 1.4.0 | Apache-2.0 | development | node_modules/expect-type |
| fdir | 6.5.0 | MIT | development | node_modules/fdir |
| fsevents | 2.3.3 | MIT | development | node_modules/fsevents |
| gensync | 1.0.0-beta.2 | MIT | development | node_modules/gensync |
| html-encoding-sniffer | 6.0.0 | MIT | development | node_modules/html-encoding-sniffer |
| is-potential-custom-element-name | 1.0.1 | MIT | development | node_modules/is-potential-custom-element-name |
| js-tokens | 4.0.0 | MIT | development | node_modules/js-tokens |
| jsdom | 29.1.1 | MIT | development | node_modules/jsdom |
| jsesc | 3.1.0 | MIT | development | node_modules/jsesc |
| json5 | 2.2.3 | MIT | development | node_modules/json5 |
| lru-cache | 11.5.3 | BlueOak-1.0.0 | development | node_modules/jsdom/node_modules/lru-cache |
| lru-cache | 5.1.1 | ISC | development | node_modules/lru-cache |
| lucide-react | 0.577.0 | ISC | runtime | node_modules/lucide-react |
| lz-string | 1.5.0 | MIT | development | node_modules/lz-string |
| magic-string | 0.30.21 | MIT | development | node_modules/magic-string |
| mdn-data | 2.27.1 | CC0-1.0 | development | node_modules/mdn-data |
| ms | 2.1.3 | MIT | development | node_modules/ms |
| nanoid | 3.3.20 | MIT | development | node_modules/nanoid |
| node-releases | 2.0.58 | MIT | development | node_modules/node-releases |
| obug | 2.2.1 | MIT | development | node_modules/obug |
| parse5 | 8.0.1 | MIT | development | node_modules/parse5 |
| pathe | 2.0.3 | MIT | development | node_modules/pathe |
| picocolors | 1.1.1 | ISC | development | node_modules/picocolors |
| picomatch | 4.0.7 | MIT | development | node_modules/picomatch |
| postcss | 8.5.29 | MIT | development | node_modules/postcss |
| pretty-format | 27.5.1 | MIT | development | node_modules/pretty-format |
| punycode | 2.3.1 | MIT | development | node_modules/punycode |
| react | 19.3.0 | MIT | runtime | node_modules/react |
| react-dom | 19.3.0 | MIT | runtime | node_modules/react-dom |
| react-is | 17.0.2 | MIT | development | node_modules/react-is |
| react-refresh | 0.18.0 | MIT | development | node_modules/react-refresh |
| require-from-string | 2.0.2 | MIT | development | node_modules/require-from-string |
| rollup | 4.64.3 | MIT | development | node_modules/rollup |
| saxes | 6.0.0 | ISC | development | node_modules/saxes |
| scheduler | 0.28.0 | MIT | runtime | node_modules/scheduler |
| semver | 6.3.1 | ISC | development | node_modules/semver |
| siginfo | 2.0.0 | ISC | development | node_modules/siginfo |
| source-map-js | 1.2.2 | BSD-3-Clause | development | node_modules/source-map-js |
| stackback | 0.0.2 | MIT | development | node_modules/stackback |
| std-env | 4.3.0 | MIT | development | node_modules/std-env |
| symbol-tree | 3.2.4 | MIT | development | node_modules/symbol-tree |
| tinybench | 2.9.0 | MIT | development | node_modules/tinybench |
| tinyexec | 1.3.1 | MIT | development | node_modules/tinyexec |
| tinyglobby | 0.2.17 | MIT | development | node_modules/tinyglobby |
| tinyrainbow | 3.2.0 | MIT | development | node_modules/tinyrainbow |
| tldts | 7.4.18 | MIT | development | node_modules/tldts |
| tldts-core | 7.4.18 | MIT | development | node_modules/tldts-core |
| tough-cookie | 6.0.2 | BSD-3-Clause | development | node_modules/tough-cookie |
| tr46 | 6.0.0 | MIT | development | node_modules/tr46 |
| typescript | 5.9.3 | Apache-2.0 | development | node_modules/typescript |
| undici | 7.30.0 | MIT | development | node_modules/undici |
| update-browserslist-db | 1.3.4 | MIT | development | node_modules/update-browserslist-db |
| vite | 7.3.7 | MIT | development | node_modules/vite |
| vitest | 4.1.11 | MIT | development | node_modules/vitest |
| w3c-xmlserializer | 5.0.0 | MIT | development | node_modules/w3c-xmlserializer |
| webidl-conversions | 8.0.1 | BSD-2-Clause | development | node_modules/webidl-conversions |
| whatwg-mimetype | 5.0.0 | MIT | development | node_modules/whatwg-mimetype |
| whatwg-url | 16.0.1 | MIT | development | node_modules/whatwg-url |
| why-is-node-running | 2.3.0 | MIT | development | node_modules/why-is-node-running |
| xml-name-validator | 5.0.0 | Apache-2.0 | development | node_modules/xml-name-validator |
| xmlchars | 2.2.0 | MIT | development | node_modules/xmlchars |
| yallist | 3.1.1 | ISC | development | node_modules/yallist |

## Inventory limits

- License values retain the package declaration, including SPDX expressions and legacy objects; no expression is split into an assumed license choice.
- This inventory provides no legal clearance or final binary notice bundle. Declared metadata must be checked against license/NOTICE texts when distributing binaries.
- Resolved across platform conditions; counts exceed any one platform's compiled or shipped dependencies. Cargo registry declarations come from local locked-package metadata.
- Nested native/vendor code and complete upstream binary archive contents require a separate target-specific distribution audit.
- The Cachella project source uses MIT; dependencies retain their separate licenses. Model catalog declarations do not verify publisher terms.
