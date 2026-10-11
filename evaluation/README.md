# Local model evaluation

Publication copies normalize maintainer home paths to `/Users/developer` in
evaluation text and JSON. Measurements and artifact hashes remain unchanged; these
paths describe the recorded environment and are not runnable setup instructions.
Historical Git revisions retain the original paths. See the
[publication review](../docs/publication-review.md).

These eight synthetic cases exercise the actual SQLite retrieval and native inference path. Each case runs once with prompt reuse disabled and once with reuse enabled. The model engine uses one slot and a 4,096-token context, with the same structured-answer request as the desktop application.

## Reproduce

Download the exact artifacts described in [model-provenance.json](model-provenance.json) and verify their SHA-256 hashes. Use a TLS-capable llama-server (the recorded runs use build 10621 on an Apple silicon Mac with 36 GiB RAM).

```sh
cargo run --manifest-path src-tauri/Cargo.toml --example evaluate_local_rag -- /path/to/llama-server /path/to/model.gguf evaluation/local-rag-cases.json evaluation/results.json
```

The harness starts its own authenticated local engine, records every answer or error and stops the child. It checks the abstention flag against the fixture. Source identifiers are validated by the production inference path. The harness does not grade factual accuracy. Review every captured answer and its citations against the stored rubric before drawing quality conclusions.

## Interpretation limits

- Eight synthetic cases provide development evidence; they do not establish production accuracy or injection resistance.
- Cold/warm labels mean the request's cache flag was disabled/enabled. These are sequential requests within one loaded model process, not independent system-wide cold starts.
- Model file size does not measure peak application RAM, GPU memory or available context capacity.
- The Q8 and Q4 artifacts come from different GGUF publishers. Their comparison does not isolate quantization from conversion differences.
- No Windows performance or battery/power measurements are included.
- Reports named `baseline` preserve outputs before abstention normalization. Current reports replace model-written abstentions with a fixed explanation and clear citations. This presentation mitigation depends on the model setting the abstention flag; it does not validate ordinary factual claims.

## Managed Gemma configuration

The evaluator accepts an optional final projector path. This selects the
production media-capable evidence path used by the managed Gemma desktop bundle,
including for text sources. [The Gemma assessment](gemma4-e2b-text-assessment.md)
records sixteen application calls across the existing eight cases and their
manual rubric review. Original text-only usage remains supported.
