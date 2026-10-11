# Hosted Windows runtime evidence

[Run 38131003959](https://github.com/henryph24/cachella/actions/runs/38131003959)
completed successfully on 11 October 2026 at source commit
`bdc8b8cb4dab7ab759779262fccd71763848a776`.

- [Run provenance](run.json) and [six-stage result](qualification.json).
- [Installer inventory and hashes](installer.json).
- [Release desktop readiness and clean close](desktop.json).
- [Installed PDF and guardian helpers](installed-helpers.json).
- [Qwen3 model verification](qwen-install.json) and [managed inference/lifecycle log](managed_runtime_smoke.log).
- [Embedding model verification](embedding-install.json), [retrieval results](multimodal.json) and [execution log](multimodal_smoke.log).
- [Native check summary](native-checks.txt).
- [Four native GUI fixtures](gui-fixtures.json), from earlier run 38129822008.
- [Earlier source-download failure](source-download-failure.txt), retained to explain the diagnostic/preflight change.

The raw multimodal JSON's macOS scope label was hardcoded in the shared example.
The parent report and workflow establish Windows Server 2025 execution; raw
report fields retain their original values. Repository text normalizes line
endings and trailing whitespace; original downloads remain under
`.local/windows-run-38131003959/reports/`. The example label is corrected for future
runs. Retrieval tests compare expected content against selected contrast fixtures;
they provide no general quality guarantee.

The local installer is retained outside Git at
`.local/windows-run-38131003959/installer/Cachella_0.1.0_x64-setup.exe`.
Its 37,193,636 bytes hash to
`b024ad7d61b1a74bae49216a1f9117a43442f64df3d17097419d5b2fa8409afe`,
matching the native installation report. It is an unsigned development artifact.

See the [qualification report](../../docs/windows-runtime-qualification.md) for
measurements and remaining Windows 11, interactive, signing and hardware checks.
