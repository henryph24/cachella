# Owned process lifecycle

The desktop application starts generation, embedding and media tools through a private-pipe guardian. The application owns the pipe's write end; the guardian owns the actual tool. Closing the lease requests termination and reaping. Application termination also closes the lease at the operating-system boundary.

## Baseline

The preceding macOS debug package started llama.cpp directly. A controlled `SIGKILL` of application PID 19417 left its owned llama-server PID 23023 alive after 5,078 ms. The probe then terminated that known orphan. A separate process check confirmed both PIDs were gone. See [baseline-forced-exit.json](baseline-forced-exit.json).

## Reproduction

Start an engine in the development package, then run:

```sh
python3 evaluation/lifecycle/force_exit.py \
  --app-executable src-tauri/target/debug/bundle/macos/Cachella.app/Contents/MacOS/cachella \
  --output evaluation/lifecycle/guarded-forced-exit.json
```

This command intentionally terminates the selected development application. It requires exactly one matching app root and at least one owned llama-server descendant. It records the process family before termination and observes that family afterward. Its final cleanup signals known survivors even if process observation fails. `--expect-survivor` is reserved for reproducing the earlier package's failure.

Probe checks:

```sh
python3 -m unittest discover -s evaluation/lifecycle -p 'test_*.py' -v
npm run test:rust
```

The Rust test script builds the guardian fixture before running library tests. The fixture supplies the guardian entry point that the Rust test harness cannot provide. Direct `cargo test` requires this fixture build first.

## Verification scope

The baseline report demonstrates the original failure. The rebuilt 108.64 MiB macOS debug package ran both generation and embedding engines. After app SIGKILL, the probe observed all 4 owned descendants (2 guardians and 2 engines) gone within 66 ms, with 0 survivors. This single observation establishes the tested lifecycle behavior; broader latency measurements require repeated runs. See [guarded-forced-exit.json](guarded-forced-exit.json). Relaunch and a read-only database check preserved 6 documents, 10 chunks and 10 embeddings. The final library suite reports 77 passing tests, including 11 guardian tests. The real Q4 generation integration reports 5 passing checks; the real multimodal integration reports 11 passing checks. Raw output: [Rust tests](rust-tests.txt), [generation](generation-smoke.txt), [multimodal](multimodal-smoke.txt), and [multimodal results](multimodal-results.json). Synthetic tests exercise owned direct children; arbitrary subprocess trees are outside the guarantee. Independently killing the guardian, an operating-system crash, Windows execution and recovery of leftover temporary/download files require separate evidence.

The guardian uses a READY/START handshake. Before START, it cannot own a tool; failed bootstrap cleanup can terminate it directly. After START, cleanup closes the lease and lets the guardian terminate and reap its tool. An owned-child guard covers subsequent error returns. Captured stdout and stderr must be drained concurrently, as with standard piped child processes; media preparation does this. Natural completion forwards remaining stdout, while lease closure ends waiting for abandoned output.

Independent review completed two rounds. Final review verified 11 guardian tests and both media output-cap and task-cancellation cleanup cases. Failure injection waits for a fixture readiness marker and applies only to the fixture executable. Windows execution remains unverified.

Final checks: the all-target desktop Clippy command with warnings denied exited 0; Rust formatting exited 0; `npm run desktop:build -- --debug --bundles app` produced 1 bundle after TypeScript and Vite built 1,749 modules. The lifecycle probe suite passed 3 tests. UI code was unchanged in this increment, so the frontend test suite was not repeated. Documentation is exempt from test-first; local links, JSON and Markdown formatting were validated separately.
