# Generation-limit feedback

The 1,250-character-per-source pressure fixture previously surfaced a generic
answer-structure error. An isolated diagnostic copy of the production inference
module captured the engine response after bounded JSON decoding. The final
captured (cache-enabled) call reported `finish_reason: length`, 3,783 prompt tokens,
313 completion tokens and 4,096 total tokens; JSON ends inside a source ID.
[Captured synthetic response](structure-response.json), [diagnostic outputs](structure-probe.json),
[diagnostic log](structure-run.txt). The earlier uncached capture had the same
usage counts and finish reason, with zero cached tokens; the capture file was
overwritten by the final request.

The pinned llama.cpp code maps a limit stop to `finish_reason: length` in
`tools/server/server-task.cpp`; `tools/server/server-context.cpp` stops generation
when context capacity is reached with shifting disabled. Completion length was
below the app's requested 384 tokens. These observations establish context
exhaustion for this synthetic failure. The `length` finish reason can also denote
the generation-token limit, so the UI message covers both possibilities.

## Change

The production answer path now checks the exact first-choice finish reason after
bounded response decoding and before content parsing. A length-limited result
returns:

> The model reached its generation or context limit before finishing the answer. Try a shorter, more specific question. Your search results are still available.

Even complete-looking JSON with `length` receives this message because the engine
reported unfinished generation. Other finish reasons preserve existing parsing
and validation behavior. No automatic retry, context expansion, token-budget
change or raw-response logging was added. Both text and media generation use this
shared response boundary. The frontend's existing error path retains retrieval
results. [Source patch](length-change.diff).

## Verification

- Regression failed first: [red output](length-red.txt) shows the prior generic
  structure error where the new message was expected.
- Focused inference tests: 13 passed, 2.01 s. [Output](length-green.txt).
  The HTTP integration regression covers truncated, complete-looking and absent
  content with exact `length`; normal stop/null with valid content; and malformed
  content with a normal stop. Raw partial engine text is excluded from the error.
- Full Rust library suite with desktop and bundled real FFmpeg: 156 passed,
  zero failed, 3.05 s. [Output](length-full-tests.txt).
- All-target desktop Clippy with `-D warnings`: passed, 2.52 s.
  [Output](length-clippy.txt). Cargo format check exited zero.
- Rebuilt production probe completed four calls on one managed engine. Both
  1,250-character calls returned the new length-limit message; subsequent
  1,200-character calls returned the correct eight-dollar answer with four valid
  citations, 3,648 prompt and 301 completion tokens. Elapsed times: 8,090 / 4,897 ms;
  cached tokens: zero / 3,647. [Cases](length-recovery-cases.json),
  [results](length-recovery-results.json), [run](length-recovery-run.txt),
  [build](length-build.txt).
- Final process inspection found zero matching diagnostic probe, production probe
  or engine executables. [Process check](length-process-check.json).
- [Hashes](length-artifacts.json) identify the production source/probe, diagnostic
  copies and evidence. The temporary Cargo example was removed after the capture;
  isolated copies remain under `.local` for provenance. The diagnostic copy had
  one added write immediately after bounded decoding and used synthetic data.
  Production source contains no response capture.

The regression exercises the actual HTTP answer boundary and its downstream
validation. No frontend behavior changed. The desktop package has not yet been
rebuilt or exercised with this message. Windows, lower-memory hardware,
representative enterprise quality and remaining maximum-workload/early-launch
qualification are incomplete. This change improves failure feedback; it does not
make the oversized workload produce a complete answer.


## Packaged native follow-up

The isolated `com.cachella.mediacheck` app was rebuilt after the production change.
[Package build](length-package-build.txt): optimized Rust build 27.68 s; bundle
114.87 MiB. The build also ran TypeScript checking and Vite. Native file-picker
import accepted four 1,300-character [LimitBudget fixtures](length-native-fixtures).
Their unique keyword avoids the older pressure documents; they extend the
1,250-character fixture with seeded alphanumeric padding after replacing the
budget name. Read-only SQLite inspection found exact content matches in all four
stored passages. [Store evidence](length-native-store.json).

Keyword query `LimitBudget` returned four passages. Generation displayed the
exact generation/context-limit message above, with all four results retained and
the Generate answer control restored. No partial answer appeared. A subsequent
`BudgetRecovery` query returned one passage and the correct eight-dollar answer
with one citation to `context-recovery.txt`: displayed 2.2 s, 196 input tokens,
67 cached. The app, guardian and engine PID rows were identical before failure
and after recovery; engine PID 14306.
[Before](length-native-before-processes.json),
[after](length-native-recovery-processes.json),
[native observations](length-native-observations.json).

Native quit left zero matching bundle executables according to
`ps -axo pid=,ppid=,comm=`. [Six source/package/fixture hashes](length-native-artifacts.json)
identify this run. Final isolated-profile counts: 15 documents, 74 chunks,
60 embeddings; fourteen synthetic text passages remain unembedded. This check
used keyword retrieval. No new process-memory measurement was collected.

This follow-up changed fixtures, documentation and evidence only; test-first is
exempt. Previous 156-test Rust, lint and formatting results apply to the unchanged
production source. Native feedback and recovery for this correction are now
verified on the development Mac. Windows, lower-memory devices, representative
enterprise quality and remaining early-launch/maximum-workload qualification
remain incomplete.
