# Included inference engine

Cachella packages the official llama.cpp b11531 engine and its dependency libraries. The prepared artifacts target Apple silicon macOS and x86-64 Windows CPU inference. The source remains able to use a manually selected compatible llama-server. Other architecture packages are not yet pinned.

## Reproduce

Python 3.10 or later is required for artifact preparation. Standard library modules perform the download, checksum verification, bounded archive extraction and manifest creation. The preparation step never executes downloaded binaries.

```sh
npm run engine:prepare
npm run desktop:build -- --debug --bundles app
```

The default platform follows the build host. On Windows, use `--bundles nsis` with the desktop build command. Preparation writes `src-tauri/resources/engine`, which is excluded from version control. An existing prepared directory is checked before reuse; a changed directory fails preparation. Both fresh extraction and reuse must match a canonical file-list fingerprint pinned in the preparer, so editing a binary and its local manifest together cannot authorize altered contents. The exact-commit llama.cpp MIT notice is bundled separately as `LICENSE-llama.cpp.txt`; original dependency notices retain their filenames. Manual source builds using the raw Tauri CLI can omit the engine resource configuration. Rust and frontend unit tests require no engine download.

For an already downloaded archive:

```sh
python3 scripts/prepare_engine.py --archive /path/to/pinned-archive
```

Cross-host archive checks accept `--platform windows-x86_64 --output /path/to/separate-directory`. This validates the package contents; Windows execution is a separate check. The [official release](https://github.com/ggml-org/llama.cpp/releases/tag/b11531) and [recorded artifact metadata](provenance.json) identify exact archive sizes and hashes.

## Runtime selection

A new configuration uses the included engine when the resource exists. Existing manual selections are preserved. **Use included engine** explicitly changes a stopped runtime while preserving its model and projector. The saved bundled-engine flag resolves the current resource directory on the next launch, so a moved or updated app can find its packaged executable. Selecting a manual executable clears that flag. Selection and shutdown share the existing runtime locks and inference permit.

Engine and model selection remain separate. Model downloads are initiated by the user and verified through the existing model installer. Packaging the engine creates no background model download or automatic model startup.

## Evidence and limits

The official macOS archive passed its pinned SHA-256 check. Production `managed_runtime_smoke` returned five PASS lines covering TLS pinning, mandatory authentication, a cited Q4 answer, stop and credential rotation. Production `semantic_smoke` returned five PASS lines covering authenticated embedding, 768-dimensional persisted vectors, expected retrieval, deletion and process cleanup.

The [multimodal integration report](multimodal-results.json) records eleven PASS checks: five media decodes and stored previews, five expected retrieval comparisons, and persistence/timestamps/deletion/process cleanup. All fixtures are synthetic; these checks establish compatibility only.

The Windows archive can be validated on macOS. Windows engine execution, installer behavior and signing remain unverified. The CI source includes macOS and Windows packaging checks, but no remote run has occurred in this workspace. The macOS hosted-runner label is arm64 according to the [GitHub runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).

Final verification on 2026-10-10:

- `python3 -m unittest discover -s scripts -p 'test_*.py' -v`: 14 passed. Tests cover unsafe cross-platform paths, archive collisions, symlink expansion bounds, download failures, malformed manifests, execute permission and altered binary-plus-manifest reuse.
- `npm run test:rust`: 66 passed. The bundled default/relocation test first failed before implementation. Selection guards, model preservation, legacy configuration loading and failed-save behavior pass.
- `npm test -- --run`: 30 passed across 6 files. `npm run build` passed with 1,749 transformed modules; direct Homebrew Clippy with warnings denied and Rust format checks passed.
- [Fresh preparation and reuse](preparation-results.json) verified both platform archives: macOS has 61 files totaling 72,108,556 bytes; Windows has 52 files totaling 49,751,379 bytes. These counts include the pinned llama.cpp notice.
- Independent review completed in two rounds. The first found mutable-manifest reuse; the parent reproduced it. The fixed preparer rejects the same binary-plus-manifest mutation, and the final reviewer reported no remaining material finding.
- `npm run desktop:build -- --debug --bundles app`: `Finished 1 bundle`, 108.12 MiB. The engine inside the package matched all 61 manifest entries.
- [Native setup](native-results.json): the app preserved both previous manual choices until **Use included engine** was clicked. Both choices saved the included-engine flag and retained model/projector paths. Generation reached Running/Connected; embedding reached Running with 10 of 10 passages indexed. A blue-square query ranked the expected video frame at 10 seconds first through the existing index. Stop controls left zero app child processes, and all 6 documents, 10 chunks and 10 vectors remained.

Documentation changes are exempt from test-first. Engine packaging and selection checks are complete for this development Mac. Windows execution, signing, other CPU architectures and enterprise release qualification remain open. The macOS developer bundle is not signed for public distribution.
