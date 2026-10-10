# Successful context-pressure fixture selection

The production `evaluate_local_rag` probe ran six synthetic fixture sizes with
cache reuse disabled and enabled, using the existing 4,096-token, single-slot
Gemma 4 E2B Q4_0 configuration and its Q8_0 projector. Each case supplies four
sources, all within the current 1,800-character answer limit. The budget fact is
followed by seeded alphanumeric reference text. These fixtures test token
pressure; they do not represent enterprise retrieval quality.

## Results

| Characters per source | Prompt tokens | Completion tokens | Cache off / on elapsed ms | Outcome |
| --- | ---: | ---: | ---: | --- |
| 900 | 2,823 | 299 | 7,527 / 5,074 | Both correct, four citations |
| 1,100 | 3,380 | 300 | 7,841 / 4,834 | Both correct, four citations |
| 1,150 | 3,516 | 296 | 7,878 / 4,790 | Both correct, four citations |
| 1,200 | 3,648 | 301 | 8,126 / 4,914 | Both correct, four citations |
| 1,250 | Unavailable | Unavailable | Unavailable | Both rejected by answer-structure parsing |
| 1,400 | Unavailable | Unavailable | Unavailable | Both explicit context-limit errors |

All eight successful answers state “The PressureBudget monthly cap is eight
dollars.” Manual review found that this matches the supplied fact; each of four
cited IDs exists in the returned evidence. Cached calls reported prompt tokens
minus one as cached; uncached calls reported zero. These are ordered single-run
observations, without a general speedup claim.

The selected 1,200-character case accounts for 3,949 prompt-plus-completion
tokens, 96.4% of the configured context capacity. This total is a usage-counter
observation; it does not directly measure KV allocation. Its four native import
files are prepared in [pressure-native-fixtures](pressure-native-fixtures).
Desktop titles derive from filenames, so native prompt token counts can differ
from this in-memory probe and must be measured separately.

The 1,250-character case's structured-answer failure is unresolved. The current
production result hides raw engine completion metadata after parsing failure.
Context exhaustion is a possible explanation; neither its finish reason nor raw
response was captured here. Do not label it a proven truncation or context error.

## Reproduction and evidence

```sh
src-tauri/target/release/examples/evaluate_local_rag src-tauri/resources/engine/llama-server .local/models/gemma-4-E2B-it-Q4_0.gguf evaluation/context/pressure-cases.json evaluation/context/pressure-results.json .local/models/mmproj-gemma-4-E2B-it-Q8_0.gguf
src-tauri/target/release/examples/evaluate_local_rag src-tauri/resources/engine/llama-server .local/models/gemma-4-E2B-it-Q4_0.gguf evaluation/context/pressure-refined-cases.json evaluation/context/pressure-refined-results.json .local/models/mmproj-gemma-4-E2B-it-Q8_0.gguf
```

Both commands exited zero; the probe reports per-call failures in its output.
[Initial fixtures](pressure-cases.json), [initial outputs](pressure-results.json),
[initial log](pressure-run.txt), [refined fixtures](pressure-refined-cases.json),
[refined outputs](pressure-refined-results.json), [refined log](pressure-refined-run.txt).
[Artifact hashes](pressure-artifacts.json), [zero matching probe/engine processes](pressure-process-check.json).

Fixture/documentation work is exempt from test-first. No executable code changed.
Validation checked twelve output records, eight correct budget strings with four
valid source IDs each, fixture lengths, artifact hashes and documentation links.
No desktop memory measurements were collected in this increment. Native context
pressure, diagnosis of the structure failure, lower-memory hardware, Windows and
representative enterprise acceptance remain unfinished.


## Native desktop pressure measurement

The previously verified release package imported all four files using its native
file picker. Read-only SQLite inspection confirmed that each stored passage
exactly matches its fixture. Keyword search `PressureBudget` returned four
passages. The single-managed-model policy was enabled; the configured generation
model was Gemma 4 E2B Q4_0 with its matching projector. The embedding engine stayed
stopped. No package rebuild was needed because executable code was unchanged.

Both desktop answers stated: “The PressureBudget monthly cap is eight dollars.
This information is present in multiple records.” All four fixture documents
were cited. The first answer displayed 8.5 s, 3,646 input tokens and zero cached;
the repeat displayed 5.2 s, 3,646 input tokens and 3,645 cached. These are two
ordered observations, without a generalized cache performance claim. The desktop
UI does not display completion-token counts, so the probe's 301 completion tokens
are not attributed to these desktop calls.

A subsequent `BudgetRecovery` query returned the correct eight-dollar answer
with one citation: 2.2 s, 196 input tokens, 67 cached. Only engine PID 11697 appears
in the complete sample set. [Native observations](pressure-native-observations.json).

The collector attached after app launch and sampled the app's resource/jetsam
coalition plus descendants. It recorded 882 samples over 135.553 s; 880 contributed
to phase totals. Median spacing was 154 ms against a 100 ms target. Normal quit
ended with `final_member_count: 0` and `stop_reason: coalition_empty`.
A separate `ps -axo pid=,ppid=,comm=` check found zero test-bundle executables.
[Raw accounting, executable hashes and exclusions](pressure-native-resources.json).

| Phase | Valid samples | Sampled peak resident sum, bytes | Sampled peak footprint sum, bytes |
| --- | ---: | ---: | ---: |
| import_and_start | 426 | 1,504,608,256 | 1,302,093,696 |
| first_pressure_answer | 163 | 1,641,168,896 | 1,420,963,264 |
| repeat_pressure_answer | 171 | 1,678,622,720 | 1,423,994,368 |
| recovery_answer | 107 | 1,722,630,144 | 1,411,886,720 |
| shutdown | 13 | 1,722,253,312 | 1,074,114,176 |

The native pressure phase now has successful long-context memory evidence on the
36 GiB M3 Pro. Phase windows include user-interface idle time; no per-request
allocation attribution is claimed. Early launch, brief unsampled peaks,
system-wide WindowServer/driver allocations, power, other workloads and target
consumer devices remain outside this measurement. Summed resident bytes may
count shared pages multiple times. These process-accounting numbers do not
establish a minimum device RAM requirement or an enforced byte-level memory cap.

Final test-profile counts: 11 documents, 70 chunks, 60 embeddings. Ten synthetic
text passages remain unembedded; this run used keyword retrieval. No executable
code changed; documentation/evidence validation is the applicable check.
The 1,250-character case's answer-structure failure, Windows execution,
lower-memory hardware and representative enterprise quality remain open.


## Subsequent structure-failure diagnosis

[Captured response and correction](length-limit.md) establish that the
1,250-character fixture exhausted context capacity during generation. The shared
answer boundary now recognizes `finish_reason: length` and gives an actionable
limit message. The earlier unresolved statements above describe their original
measurement scope. Production recovery succeeds; native package verification of
this later message remains pending.
