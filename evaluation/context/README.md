# Context-limit errors and recovery

Dense evidence can exceed the managed generation engine's 4,096-token context
while satisfying the app's four-source and 1,800-character-per-source limits.
Character limits do not establish a token budget. The existing live probe found
HTTP 400 failures for dense ASCII and supplementary Unicode evidence; ordinary
requests afterward succeeded on the same engine.

The production response boundary now recognizes HTTP 400 with exact JSON
`error.type = exceed_context_size_error` and displays:

> The question and retrieved evidence exceed this model's context limit. Try a shorter, more specific question. Your search results are still available.

This structured type matches the pinned local llama.cpp server's
`tools/server/server-common.cpp` and its chat-completion context-limit test.
Only the structured type is used. Raw engine error messages are never displayed.
Unknown types, malformed JSON, other HTTP status codes, oversized bodies and
truncated/stalled responses retain the generic status-only message.

The error parser limits its accumulated buffer to 8,192 bytes and uses a separate
2-second deadline. This cap bounds accumulated/parser data; reqwest may allocate
an individual transport frame before the application can reject it. Successful
responses retain the existing 1 MiB JSON handling. No retry, token truncation,
context expansion or model-setting changes are introduced.

## Live model evidence

[Four synthetic cases](cases.json) use four documents each and the same budget
question. Baseline and recovery contain ordinary evidence. Dense ASCII uses a
seeded alphanumeric sequence; dense Unicode uses supplementary CJK code points.
Every source remains within 1,800 characters. The production store retrieval and
answer path run through the existing `evaluate_local_rag` example.

The verified post-change invocation was:

```sh
src-tauri/target/release/examples/evaluate_local_rag src-tauri/resources/engine/llama-server .local/models/gemma-4-E2B-it-Q4_0.gguf evaluation/context/cases.json evaluation/context/gemma-after.json .local/models/mmproj-gemma-4-E2B-it-Q8_0.gguf
```

The rebuilt probe used the managed TLS/token runtime, one engine slot and the
selected Gemma 4 E2B Q4_0 plus Q8_0 projector. Eight requests ran with cache reuse
off/on for each case. [Final output](gemma-after.json), [run log](run-after.txt),
[artifact hashes](artifacts.json).

| Case | Cache off | Cache on | Manual assessment |
| --- | --- | --- | --- |
| Baseline | 5,215 ms; 546 prompt tokens; 0 cached | 4,682 ms; 546 prompt tokens; 545 cached | Both correctly state eight dollars per month with four valid citations |
| Dense ASCII | Context-limit rejection | Context-limit rejection | Explicit failure; no answer scored |
| Dense Unicode | Context-limit rejection | Context-limit rejection | Explicit failure; no answer scored |
| Recovery | 5,009 ms; 549 prompt tokens; 0 cached | 4,478 ms; 549 prompt tokens; 548 cached | Both correctly state eight dollars per month with four valid citations |

The probe records four errors because the oversized cases are intentional rejected
requests. Recovery uses the same running engine after those failures. These eight
calls establish bounded failure/recovery evidence, with no general performance,
quality, maximum-token memory or consumer-hardware claim. Rejected requests expose
no token count through the current result type. [Pre-change outputs](gemma-before.json)
and [pre-change log](run-before.txt) retain the generic-error baseline. The initial
baseline used the existing release probe while a rebuild was underway; artifact
hashes identify the verified post-change run only.

## Verification

- The error-message regression failed before the change with actual `HTTP 400`.
  [Red](error-red.txt), [initial green](error-green.txt).
- Full desktop Rust library suite with the bundled real decoder enabled:
  155 passed, zero failed, 2.72 s. [Output](full-tests.txt).
- Final focused inference suite after test-server improvements: 12 passed,
  zero failed, 2.01 s. Includes exact 8,192-byte acceptance, oversized rejection,
  malformed/unknown types, HTTP 500, premature close after valid JSON, and a
  stalled body whose independent deadline ends before the client timeout.
  [Output](final-inference-tests.txt).
- Final all-target desktop Clippy with `-D warnings` and Rust formatting passed.
  Initial Clippy found ignored mock-server read lengths; the final helper reads
  a complete bounded header. [Final lint](clippy-final.txt).
- Release probe rebuilt successfully in 3.30 s before the verified live run.
  [Build](build-after.txt). Later edits affected test code only.
- Independent response-boundary review found no behavior defect. Its three
  suggested boundary cases are included in the final focused suite.
- Process inspection found zero matching probe/engine executables after normal
  completion. [Process evidence](process-check.json).

The probe stores above were in-memory. The packaged follow-up below uses the
isolated test profile. Full product acceptance remains incomplete.


## Packaged native feedback and recovery

The isolated `com.cachella.mediacheck` release app was rebuilt with the correction.
[Build output](package-build.txt): optimized Rust build 27.26 s; bundle 114.87 MiB.
The native controls imported [dense evidence](native-fixtures/context-overflow.txt)
(74,500 bytes, five passages) and [recovery evidence](native-fixtures/context-recovery.txt)
(92 bytes, one passage). Keyword search `ContextOverflow` returned five passages;
Generate answer displayed the exact actionable message quoted above. All five
results and the generation control remained available. The initial broader query
also matched the recovery document on `cap`; the rejection check used the isolated
keyword query.

Keyword search `BudgetRecovery` then returned one passage. Generation answered:
“The BudgetRecovery monthly cap is eight dollars, which is the approved recurring
monthly budget.” One citation pointed to `context-recovery.txt`. The UI reported
2.2 s, 196 input tokens and zero cached tokens. Engine PID 9940 was unchanged
between rejection and recovery: [before](native-process-before.json),
[after](native-process-recovery.json). These are single-run observations, without
a cache speedup claim. [Native observations](native-observations.json),
[package/source/fixture hashes](native-artifacts.json).

The existing read-only coalition collector recorded 1,336 samples over 205.454 s,
with 1,335 samples included in phase totals. Actual median spacing was 154 ms
(target 100 ms). [Raw measurements and exclusions](native-resources.json).

| Phase | Valid samples | Sampled peak resident sum, bytes | Sampled peak footprint sum, bytes |
| --- | ---: | ---: | ---: |
| Imports and model startup | 1,154 | 1,478,934,528 | 1,369,808,896 |
| Oversized request | 96 | 1,482,555,392 | 1,256,693,824 |
| Recovery answer | 67 | 1,531,707,392 | 1,334,650,496 |
| Shutdown | 18 | 1,531,461,632 | 984,770,176 |

Phases include UI idle time and associated helpers. The rejected request does not
establish memory use for a successfully filled 4,096-token context. Read-only
attachment excludes early launch; sampling may miss short peaks; resident sums
can count shared pages multiple times. System-wide driver/OS allocations and
power are excluded. Hardware was the M3 Pro with 36 GiB RAM.

Normal native quit ended with `final_member_count: 0` and
`stop_reason: coalition_empty`; a separate `ps -axo pid=,ppid=,comm=` inspection
found zero matching bundle executables. Read-only SQLite counts were seven
documents, 66 chunks and 60 embeddings. The six new text passages remain
unembedded; this check used keyword retrieval. The original desktop profile was
outside this test's scope.

This follow-up changed documentation and evidence only, so test-first was exempt.
The package build ran TypeScript checking, Vite and Rust release compilation.
Previous executable tests above remain applicable. Successful near-full-context
memory, representative enterprise quality, lower-memory consumer hardware and
Windows acceptance remain open.
