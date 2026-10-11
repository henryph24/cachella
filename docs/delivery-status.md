# Cachella delivery status

As of 11 October 2026, the local macOS demo and hosted Windows release runtime
have recorded execution evidence. Full delivery remains incomplete pending Windows
11 user acceptance, signing/publication and consumer-device qualification.

## Available demo

Release app: `src-tauri/target/release/bundle/macos/Cachella Media Check.app`
(114.87 MiB). This is an isolated test build using `com.cachella.mediacheck`;
it contains test documents and references separately downloaded local model files.
It is not a signed distributable or a self-contained bundle of model weights.

| Capability | Evidence and scope |
| --- | --- |
| Persistent local documents and source inspection | [Acceptance map](acceptance-audit.md); SQLite, text/PDF and native persistence checks |
| Text, image, audio and decomposed-video retrieval | [Multimodal evaluation](../evaluation/multimodal/README.md); synthetic modality fixtures |
| Quantized grounded text/media answers | [Media answer report](../evaluation/media-answers/README.md), [text rubrics](../evaluation/gemma4-e2b-text-assessment.md) |
| Local prompt reuse | [Resource benchmark](../evaluation/resources/README.md); built-in llama.cpp caching |
| One-managed-model mode | [Native residency checks](../evaluation/residency/README.md); switching, explicit stop, cancellation and restart |
| Context overflow and unfinished generation | [Native context rejection](../evaluation/context/README.md), [generation-limit recovery](../evaluation/context/length-limit.md) |
| Long-context desktop operation | [Pressure measurements](../evaluation/context/pressure.md); 3,646 input tokens, four citations, correct recovery |
| Media input bounds | [Duration](../evaluation/resources/duration-boundary.md), [geometry/bytes](../evaluation/resources/boundaries/README.md), [20MP noise image](../evaluation/resources/complex-image/README.md) |

Local prompt caching is implemented. LMCache remains optional future shared-server
work. No Fly.io resources or recurring cloud charges were introduced.

## Windows execution checkpoint

[Run 38131003959](https://github.com/henryph24/cachella/actions/runs/38131003959)
passed all six [Windows runtime gates](windows-runtime-qualification.md): release
installation, desktop readiness/close, installed PDF/guardian helpers, Qwen3
inference, five multimodal retrieval cases and zero remaining app/engine processes.
Native checks passed 154 Rust tests plus the explicit reparse test, 67 frontend
tests, lint, four GUI probe fixtures and 11 decoder fixtures. Python collected
41 tests with six platform skips; MSYS decoder tests collected 18 with two skips.

The hosted OS was Windows Server 2025 with about 8 GiB physical memory. This
single synthetic workload does not establish minimum RAM or Windows 11 consumer
performance. Standard-user install/uninstall, interactive imports, GPU backends,
per-user data isolation and the intended consumer workload remain to be tested.

The private GitHub runner is available; the earlier missing-runner blocker is
resolved. Each job has a 60-minute timeout, with installer and report artifacts
retained for three days. Evidence and the tested installer were also retained
locally. No Fly resources or paid VM subscriptions were created.

## Remaining qualification limits

Representative enterprise quality needs a defined workload and relevant content.
Existing quality results use eight text rubrics and synthetic media. They provide
development evidence; broad reliability or prompt-injection immunity is unproven.

Resource reports exclude early launch and system-wide allocations and contain
explicit missed-process cases. Maximum-byte complex video, combined stress and
lower-memory devices remain unqualified. These exclusions must accompany demo
claims. Signing, publication and an enterprise server require separate authorized
distribution/infrastructure setup.

Latest behavior-change checks: 156 Rust tests, thirteen focused inference tests,
all-target desktop Clippy and formatting passed. The release package was rebuilt
and its new message/recovery path verified natively. [Check evidence](../evaluation/context/length-limit.md).
This delivery summary changes documentation only; test-first is exempt. Links and
current environment evidence were checked. The full objective is incomplete.
