# PDF import and physical-page citations

Cachella imports selectable-text PDFs through its native file picker. A guarded helper extracts text locally. The store retains the original bytes, all physical pages (including blank pages), and page metadata for every searchable chunk. Keyword and semantic search return that metadata; answer citations open the selected page in an extracted-text preview.

## Limits and behavior

- Source: 16 MiB; 1 to 200 physical pages; 2 MiB total extracted text.
- Parser loading and page/font streams: 4 MiB decompression limit per applicable stream/page operation.
- Font CMaps: at most 65,536 represented source codes and 1,048,576 conservative expanded UTF-16 units per CMap.
- Helper: the existing owned-process runner applies a 30-second tool wait, bounded diagnostics and cancellation cleanup.
- Library: existing 500-document, 10,000-chunk and 512 MiB shared asset limits apply transactionally.
- Password-protected, permission-protected, malformed and entirely textless PDFs fail explicitly. Empty physical pages in an otherwise textual document remain navigable.
- The preview shows extracted text. Original layout, scanned-page OCR, printed page labels and encrypted-document unlocking are outside this increment. Page numbers identify physical positions in the PDF.

These are scoped input, decoding and time limits. They do not establish a process-wide memory quota or qualify every consumer device. Complex layouts, tables, reading order and representative enterprise PDF quality still require broader evaluation. Raw PDF bytes are retained in SQLite; the webview's media protocol continues to reject `application/pdf`.

## Dependency review

The pinned parser is lopdf 0.45.0, using its bounded loading and extraction APIs. Review found that upstream reverse font mapping could expand a short range into billions of entries despite a stream-byte cap. A bounded 65,537-entry regression first failed against upstream. The application now uses a local copy with two source files changed: mapping cardinality/payload checks before expansion, and propagation of the limit error through font fallback. The upstream MIT license is retained.

See [patch provenance and scope](../../vendor/lopdf/CACHELLA_PATCH.md), [failing regression](cmap-regression-red.txt), and [passing extraction regressions](cmap-regression-green.txt). Independent review completed two rounds and verified error propagation plus normal Unicode mapping. Windows execution remains unverified locally.

## Automated evidence

- [Rust output](rust-tests.txt): 90 passed, 0 failed. Includes extraction, protected PDFs, decompression limits, font mapping limits, additive schema migration, page metadata across FTS/semantic/index paths, persistence, capacity rollback and deletion.
- [Frontend output](frontend-tests.txt): 33 passed. PDF search/citation clicks select the physical page, library opening starts at page 1, and blank-page navigation preserves numbering.
- All-target desktop Clippy with warnings denied and Rust formatting passed.
- TypeScript/Vite built 1,750 modules; the final macOS debug build produced one 115.06 MiB app bundle.

## Actual local models

[Raw model output](model-smoke.txt) records five passing checks. [Model results](model-results.json) contain the extracted pages, keyword/semantic hits and actual Q4 answer with source metadata.

The synthetic document has launch text on page 1, a blank page 2 and a budget statement on page 3. Actual EmbeddingGemma 2 retrieval ranked page 3 first. Actual Qwen3-1.7B Q4 generation cited page-3 evidence. Reopening SQLite preserved all three pages, original bytes and two vectors; deleting the temporary test document removed searchable evidence and vectors. Both managed engines stopped.

Reproduce with downloaded local models:

```sh
cargo run --manifest-path src-tauri/Cargo.toml --example pdf_smoke -- \
  /path/to/llama-server /path/to/generation.gguf \
  /path/to/embedding.gguf /path/to/projector.gguf \
  evaluation/pdf/fixtures/project-brief.pdf evaluation/pdf/model-results.json
```

The test fixture is 2,599 bytes, SHA-256 `3ff70254aa56b77c03015907b5ac0c5d1e36ed3ebf1e25937964a27b49414f54`. Its three rendered pages were inspected, including the intentionally blank page. The generator is [create_fixture.py](create_fixture.py); it requires ReportLab.

## Packaged native verification

The final package imported the PDF through the file picker and retained the six existing documents. The library showed seven documents and twelve passages. Opening the PDF from the library selected page 1; the blank page 2 remained navigable; the budget search result opened page 3.

Actual Q4 generation answered “The Orion infrastructure budget is nine dollars per month.” with the citation “1. project-brief.pdf · Page 3”. The UI displayed 1.9 seconds, 569 input tokens and zero cached tokens. Clicking the answer citation opened page 3. These observations were captured in the preceding native verification session.

The continuation indexed both PDF passages and displayed “12 of 12 passages indexed”. Both runtimes were stopped before quitting. Relaunch displayed seven documents and twelve passages; the PDF reopened at page 1 of 3. Read-only SQLite checks verified twelve persisted vectors, all three PDF pages (including blank page 2), two PDF chunks linked to pages 1 and 3, two PDF vectors and the unchanged 2,599-byte source. A process-tree check found zero descendants of the relaunched app. See [native results](native-results.json).

This completes the selectable-text PDF increment on the tested macOS setup. Media-to-text answers, decoder distribution, orphan-file recovery, whole-app resource bounds, representative quality qualification, Windows execution/signing and the full product audit remain outstanding.
