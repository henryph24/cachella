# Multimodal feasibility

This directory records EmbeddingGemma 2 feasibility and native desktop integration checks through llama.cpp. The application now exposes text/image/audio/video imports and persisted semantic retrieval.

## Runtime and artifacts

- Model: `ggml-org/embeddinggemma-2-GGUF`, revision `bfcd298762cc34d0357ece5ebdd31791a3a374d8`.
- Text weights: `embeddinggemma-2-Q8_0.gguf`, 309,855,456 bytes; SHA256 `2188ac1deca4b77dffefd603c2776a9d76d9d74ec01841392982ebb840b09135`.
- Vision/audio projector: `mmproj-embeddinggemma-2-Q8_0.gguf`, 554,821,024 bytes; SHA256 `c4a8a52691ecef40618438928bdf9e68379b854e24166f292592353db0aab64f`.
- Apple M3 Pro, 36 GiB RAM. Installed llama-server build 10621 (`c1d0e7a00`) rejects this model architecture. The probe requires a newer source build containing commit `4fbc76dec51d0add466f0210855c0596589b60d4` or a later compatible release.

The [Google model card](https://huggingface.co/google/embeddinggemma-2) describes 740M parameters, 768-dimensional shared embeddings and Apache-2.0 licensing. [llama.cpp PR 30054](https://github.com/ggml-org/llama.cpp/pull/30054) merged text, vision and audio support on October 6, 2026. The [native server API](https://github.com/ggml-org/llama.cpp/blob/c1d0e7a00/tools/server/README.md) accepts base64 media via a structured prompt object.

The [Jina ONNX candidate](https://huggingface.co/onnx-community/jina-embeddings-v5-omni-nano-ONNX) publishes CC BY-NC 4.0 metadata, fixed image layouts and five-second audio inputs. Its commercial licensing path needs separate consideration. The initial native feasibility work uses EmbeddingGemma 2.

## Fixtures

All content is synthetic. PNGs contain a red circle and blue square on white backgrounds. Audio is macOS Samantha text-to-speech at 160 words/minute, converted with FFmpeg to 16 kHz mono PCM WAV:

- Budget: “The project cloud budget is eight dollars per month.”
- Lunch: “Lunch is tomato soup with fresh bread and green salad.”

The MP4 contains the red-circle image and budget audio. `video-frame.png` is extracted at 0.25 seconds; `video-audio.wav` is its separately decoded soundtrack. `video-metadata.json` preserves FFprobe stream metadata. These test separate visual and audio retrieval paths. They do not establish fused temporal video understanding.

`lunch-video.mp4` combines the blue square and lunch audio for mixed-media answer
quality checks. Its recipe and hashes are recorded in
[normalization provenance](../media-answers/normalization-provenance.json);
[answer results](../media-answers/normalization.md) distinguish the baseline
modality confusion from the corrected presentation.

## Run

The supporting source commit is pinned below. Build in a separate directory; the existing generation engine can remain installed. CMake discovers platform OpenSSL and Metal support according to the local toolchain.

```sh
cmake -S /path/to/pinned/llama.cpp -B /path/to/build -DCMAKE_BUILD_TYPE=Release -DLLAMA_BUILD_TESTS=OFF -DLLAMA_BUILD_EXAMPLES=OFF -DLLAMA_OPENSSL=ON
cmake --build /path/to/build --target llama-server -j 6
```

Start that separate loopback engine for these synthetic fixtures:

```sh
llama-server -m /path/to/embeddinggemma-2-Q8_0.gguf --mmproj /path/to/mmproj-embeddinggemma-2-Q8_0.gguf --embedding --pooling mean --host 127.0.0.1 --port 18081 -c 2048 -b 2048 -ub 2048 -np 1 --no-webui
python3 evaluation/multimodal/probe.py --fixtures evaluation/multimodal/fixtures --output evaluation/multimodal/results.json
```

The probe checks vector dimensions, finite components, unit norms, and expected top-1 ranking against one contrasting candidate per case. It records input hashes and timing. The five cases cover text, image, audio, video frame and video soundtrack. Passing is a feasibility result; it does not establish production retrieval accuracy.

The native integration example below exercises the authenticated, certificate-pinned product boundary, timestamped parts, persisted vectors, bounded decoding and deletion. The original Python probe uses a separate synthetic HTTP endpoint.

## Observed result: October 9, 2026

The pinned source build completed (`Built target llama-server`). All eight corpus embeddings and five query embeddings had 768 finite components and unit L2 norm. All five expected top-1 retrieval checks passed:

| Case | Top result | Expected-result cosine | Contrast-result cosine |
|---|---|---:|---:|
| text | text_budget | 0.805 | 0.513 |
| image | red_circle | 0.778 | 0.626 |
| audio | audio_budget | 0.853 | 0.502 |
| video_visual | video_frame | 0.783 | 0.626 |
| video_soundtrack | video_audio | 0.859 | 0.502 |

See [captured scores, timings and fixture hashes](results.json) and [artifact provenance](provenance.json). The combined Q8 model/projector files total 864,676,480 bytes; runtime memory is additional and was not benchmarked.

Two compatibility findings matter for integration:

1. The installed generation engine (build 10621) cannot load `gemma-embedding2`; use the pinned newer build or a verified compatible release.
2. The server overrides the C API's default media marker with a per-process value. Read `media_marker` and modality flags from `/props`; hardcoding `<__media__>` produced HTTP 500 and a marker-count mismatch. The probe now discovers and validates the marker before sending media.

The engine logs mark audio input experimental and cap image tokens according to the microbatch size. The probe used 2,048-token context/batch/microbatch and one slot. These results cover short synthetic media only. Windows execution, longer media, resumption, native-app vector persistence and page/timestamp source navigation remain unverified.


## Native integration check

```sh
cargo run --manifest-path src-tauri/Cargo.toml --example multimodal_smoke -- /path/to/llama-server /path/to/model.gguf /path/to/mmproj.gguf /path/to/ffmpeg evaluation/multimodal/fixtures evaluation/multimodal/native-results.json
```

The example uses the production decoder, SQLite store, asset-range handler, authenticated embedding client and runtime lifecycle. The [captured native report](native-results.json) records five passing retrieval comparisons across seven indexed parts, persistence after reopen, timestamp metadata, the text-only generation guard, deletion and process cleanup. This is synthetic integration evidence; it does not establish general retrieval quality or consumer-hardware performance.

`temporal-video.mp4` is a synthetic twelve-second navigation fixture: the existing red-circle PNG for six seconds followed by the blue-square PNG for six seconds, with the budget speech looped as its soundtrack. It is encoded locally with FFmpeg H.264/AAC. Normalization produces frames at 0, 5 and 10 seconds and audio segments at 0 and 10 seconds, allowing a nonzero seek check.

Additional local decoder checks cover JPEG, MP3, FLAC, OGG and WebM conversions of the same fixtures. The native example above covers PNG, WAV and MP4. Windows runtime and larger representative media collections remain unverified.


Packaged macOS UI verification imported PNG, WAV and the temporal MP4 through native pickers. The blue-square query returned the 10-second video frame first; opening and playing it displayed the blue square at elapsed 10 of 12 seconds. The image preview rendered, and the audio control loaded a three-second duration and entered playback. This checks native WebKit media handling and the custom asset protocol on macOS. The temporal fixture [hash and provenance](temporal-fixture.json) are recorded separately.
