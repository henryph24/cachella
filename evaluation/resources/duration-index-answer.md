# Full-duration video indexing and cited answer

The existing release app completed all 36 new passages from `limit120.mp4`.
The isolated library now has 58/58 stored embeddings across three documents.
Read-only SQLite checks confirm the two-minute fixture has 24 image parts and
12 audio parts, covering 0–120,000 ms. Each of its 36 chunks has an embedding.
[Per-document counts](duration-index-counts.json).

With Single residency enabled, explicitly starting generation suspended the
embedding engine. Semantic search resumed embeddings, and answer generation
resumed the generation engine. Search returned eight audio passages; six were
from the two-minute fixture, including its final 1:50–2:00 segment.

Question: “What monthly cloud budget does the video mention?”

Answer: “The project cloud budget is eight dollars per month.”

Manual review matches the synthetic source's spoken budget. The response carried
four citations, including two from `limit120.mp4`. Opening its first citation
showed the stored video preview and matched interval 1:00–1:10. Displayed inference
time was 6.3 s, with 1,349 input tokens and zero cached tokens; that display excludes
preceding retrieval and model-switch startup. The source repeats a short recording,
so this verifies sampled-media retrieval and generation, with no claim of continuous
two-minute reasoning or representative enterprise quality.
[Native observations](duration-index-answer-observations.json).

## Process measurements

The existing read-only coalition collector ran on the 36 GiB M3 Pro with the
[previously identified release artifact](../residency/hash-cancellation/release-artifacts.json).
It recorded 1,019 samples over 93.333 s, with 1,016 valid nonempty samples. The
50 ms target interval yielded a 91 ms median actual interval. Every sample had
at most one engine process; normal quit ended with zero coalition members.

| Phase | Valid samples | Peak resident sum (GiB) | Peak footprint sum (GiB) |
| --- | ---: | ---: | ---: |
| embedding_start | 253 | 1.166 | 1.149 |
| index_36_passages | 200 | 1.451 | 1.393 |
| generation_start | 164 | 1.452 | 1.301 |
| semantic_search_switch | 153 | 1.252 | 1.133 |
| answer_switch | 246 | 1.360 | 1.183 |

[Raw samples and artifact metadata](m3-pro-duration-index-answer.json).
Phase labels were manually set before UI actions and include subsequent idle time,
source inspection and shutdown; these are process-memory observations, with no
end-to-end latency benchmark. Attribution includes app, engine and associated
WebKit/Metal helpers. Early app launch and system-wide OS/driver memory remain
excluded; shared resident pages may be double-counted and brief peaks may be missed.

## Verification and limits

No executable code changed, so test-first is exempt and product suites were not
rerun. Verification used the native startup/index/search/answer/citation/quit
sequence, database counts, raw sample totals, final empty coalition, artifact
hashes and local report links. The original user profile remained untouched;
the isolated profile retains three documents, 58 chunks and 58 embeddings.

This completes indexing and a cited answer for the accepted duration-boundary
fixture. Maximum input bytes/resolution, generation context pressure, representative
enterprise documents, lower-memory hardware and native Windows delivery remain
open. Existing hash-cancellation evidence retains its previously documented limits.
