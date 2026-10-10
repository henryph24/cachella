# Cachella delivery status

As of 10 October 2026, the local macOS demo is built and has recorded execution
evidence. Cross-platform delivery remains incomplete pending native Windows
verification and the intended 16 GB device check. The active objective stays open.

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

## Delivery gates requiring another environment

1. Run the checked-in Windows build/test workflow and retain its actual output.
   The workspace has no Git repository or identified authorized remote runner.
2. Exercise NSIS installation, model/decoder setup, inference, four-modality
   import/retrieval and process recovery on Windows. A workflow file and prepared
   Windows resources are insufficient execution evidence.
3. Measure the core workload on the intended 16 GB consumer device. The available
   Mac has 36 GiB RAM; its process measurements do not establish minimum RAM.

[Current environment inspection](../evaluation/delivery/environment.json) reports
macOS, `git rev-parse` exit 128, no `qemu-system-x86_64` or `wine` on PATH, and no
UTM/Parallels/Fusion at the inspected app paths. Other remote environments were
not inventoried. A request for an available authorized Windows machine or
repository/runner is pending. No access, repository creation or remote publishing
is assumed from silence.

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
