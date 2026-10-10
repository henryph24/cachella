# Local lopdf patch

Upstream: lopdf 0.45.0, downloaded from crates.io. Registry archive SHA-256:
`bfffda0fe1ab0157e1a13c14bebd3f28671f2fccb7922f0722ec53926e6922d3`.
The upstream MIT license is preserved in [LICENSE](LICENSE).

`src/encodings/cmap.rs` and `src/object.rs` change upstream behavior. Before constructing a
ToUnicode reverse map, it bounds the sum of represented source codes to 65,536
and the conservative total of cloned UTF-16 units to 1,048,576. Checked u64
arithmetic handles four-byte source-code ranges. An over-limit CMap returns an
error before range expansion. Font decoding propagates this error through its
usual fallback path, so PDF import fails explicitly.

This closes a gap in the upstream decompressed-stream limit: a short bfrange
can describe billions of mappings. The application regression first failed
against upstream with a bounded 65,537-entry input. Regression tests live in
`src-tauri/src/pdf.rs`. These limits apply per CMap; they are not a process-wide
memory quota. The guarded extraction helper also enforces an execution timeout.

Review this patch and its regressions when updating the dependency. Do not
replace the path dependency with the upstream crate until equivalent bounds
are verified. This copy retains upstream examples and tests for provenance;
Cachella's required checks run through its own Cargo manifest.
