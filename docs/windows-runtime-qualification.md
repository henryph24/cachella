# Windows runtime qualification

## Purpose and scope

Cachella is a desktop application. Windows qualification must exercise the
installed Windows executable, its helper modes, bundled inference engine and
local models. A successful cross-compile alone does not establish runtime support.

The investigation uses the existing private GitHub Actions Windows runner. It
creates no Fly.io resources and requires no new VM subscription. Hosted runner
results cover that runner's OS and CPU configuration. Consumer Windows 11,
interactive imports, GPU acceleration and hardware performance require separate
qualification.

## Demonstrated readiness-test defect

Run [38028684256](https://github.com/henryph24/cachella/actions/runs/38028684256)
installed a debug bundle and stopped when `WaitForInputIdle` threw. The previous
script saved its first report only after successful GUI closure. No retained exit
code, stderr or window observations established whether Cachella had crashed.

The debug executable uses the console PE subsystem; release builds use the Windows
GUI subsystem. [.NET's API contract](https://learn.microsoft.com/en-us/dotnet/api/system.diagnostics.process.waitforinputidle)
allows an exception for missing message loops and other process conditions. That
exception alone cannot distinguish a working console-subsystem GUI from startup
failure.

Run [38129822008](https://github.com/henryph24/cachella/actions/runs/38129822008)
passed four native Windows fixtures:

| Fixture | Required result | Observation |
| --- | --- | --- |
| Console subsystem with a window | Visible owned window, ready marker, exit 0 | Passed despite the exact previous `WaitForInputIdle` exception |
| Windows GUI subsystem | Visible owned window, ready marker, exit 0 | Passed |
| Exit before opening a window | Reject and retain exit 42 plus stderr | Passed |
| Window without ready marker | Reject by deadline and remove owned process | Passed |

Evidence: [native fixture report](../evaluation/windows-runtime/gui-fixtures.json).
This proves a defect in the previous acceptance test. It does not retrospectively
prove why the previous Cachella process behaved as it did.

## Qualification gates

`scripts/qualify_windows.ps1` records six independent gates:

1. Install the release NSIS package, verify the pinned installed engine payload,
   record hashes and file inventory, and require PE subsystem 2.
2. Observe a visible Cachella window owned by the launched process, find
   `Workspace ready` through UI Automation, close the window and require exit 0.
3. Run the installed executable's PDF extraction mode and its process guardian
   using explicit private pipes to launch installed FFmpeg.
4. Install pinned Qwen3 weights and exercise authenticated managed inference,
   citations, stop/restart and credential rotation through the runtime smoke test.
5. Install pinned EmbeddingGemma 2 weights and run synthetic multimodal retrieval
   through the production engine and decoder.
6. Check for remaining Cachella or inference processes.

Installation failure prevents dependent stages. Later failures preserve their own
reports and allow the other independent checks to execute. Any required failure
fails the overall qualification. GUI diagnostics include executable subsystem,
window ownership, readiness observations, stdout/stderr, signed and hexadecimal
exit codes, and forced-cleanup status.

The installed helper checks matter because Rust smoke examples re-execute their
own example binary as the guardian. Those examples alone do not exercise the
release GUI executable's helper entry points.

## Native result: 11 October 2026

Run 38129822008 passed 154 Rust tests, the separately invoked Windows reparse
regression, frontend checks, lint and GUI fixtures. It then failed a zlib source
archive integrity check before decoder compilation. Its error lacked observed
size/hash metadata; the exact cause remains unknown. The same official download
and VideoLAN mirror matched the unchanged pin when checked from macOS.

The follow-up adds observed size/hash diagnostics and verifies source archives
before compilation. Run [38131003959](https://github.com/henryph24/cachella/actions/runs/38131003959)
verified all three unchanged source pins on Windows and completed successfully.
All six qualification stages passed. The tested source commit is
`bdc8b8cb4dab7ab759779262fccd71763848a776`.

The [upstream zlib page](https://zlib.net/) publishes the same pinned SHA-256
(`d7a0654783a4da529d1bb793b7ad9c3318020af77667bcae35f95d0e42a792f3`).
The pin and rejection behavior remain intact.

## Retained evidence and measurements

The [evidence index](../evaluation/windows-runtime/README.md) includes the original
JSON reports, model execution logs and native check summaries. The local installer
hash matches the installed artifact's report:

- File: `Cachella_0.1.0_x64-setup.exe`, 37,193,636 bytes.
- SHA-256: `b024ad7d61b1a74bae49216a1f9117a43442f64df3d17097419d5b2fa8409afe`.
- OS: Windows Server 2025 Datacenter, build 26100.
- Physical memory visible to the runner: 8,584,425,472 bytes, about 8 GiB.
- CPU label: AMD EPYC 9V74. The label names the host processor; it does not report
  the number of vCPUs allocated to this job.

| Check | Observed result |
| --- | --- |
| Release installation and pinned engine | Passed; PE subsystem 2 |
| Desktop probe | Ready marker observed; clean exit 0; no forced cleanup; 8.344 s total probe time including setup and close |
| Installed PDF helper | Three pages; exit 0 |
| Installed guardian helper | READY/START handshake, forwarded FFmpeg output, exit 0 |
| Qwen3 answer | One citation; 12.436 s for one synthetic request |
| Runtime security/lifecycle | Wrong TLS certificate and unauthenticated requests rejected; stop, restart, credential rotation and owner-drop cleanup passed |
| EmbeddingGemma 2 retrieval | Five contrast-ranking cases passed across text, image, audio, video frame and video audio |
| Indexing | Seven synthetic parts in 13.734 s |
| Persistence and cleanup | Persistence, timestamp metadata, deletion and closed engine listener passed; zero remaining app/engine processes |

These are single-run observations. The retrieval criterion compares expected
content with a selected contrast; it does not require every expected item to rank
first. Multimodal generative answers and single-resident model switching were not
exercised in this Windows run. The raw multimodal report contains an older
hardcoded `Synthetic macOS integration` scope label; the workflow and parent
qualification report establish Windows execution. The example's label was then
corrected to `Synthetic native integration`; historical evidence is preserved.

Native validation also passed 154 Rust tests plus one separately invoked reparse
test, 67 frontend tests, four GUI fixtures, 11 decoder fixtures, Clippy and
formatting. Python collected 41 tests with six platform skips; the separate MSYS
decoder suite collected 18 with two skips. The local helper suite passed nine
tests, and the source-diagnostic regression was observed failing before its fix.

To repeat the full hosted qualification from the repository:

```sh
gh workflow run desktop-checks.yml --ref main -f platform=windows -f qualify_runtime=true
```

## Remaining consumer acceptance

Use a Windows 11 x64 machine with representative RAM and CPU to measure cold model
load, first-token latency, sustained generation, indexing time, peak memory,
installation/uninstallation and interactive imports. Test standard-user accounts,
paths containing spaces and Unicode, per-user data isolation, restart, cancellation
and offline behavior. Signing and installer warning behavior also need end-user
validation.
GPU acceleration requires separate qualification for each supported backend.

A local Windows ARM VM on an Apple silicon Mac can help interactive debugging.
Its x64 application emulation does not establish native x64 CPU/GPU performance.
The hosted runner is the first diagnostic environment; a borrowed Windows 11 PC
is a useful next environment for consumer acceptance.
