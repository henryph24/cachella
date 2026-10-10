# Decoder format fixtures

Synthetic fixtures extend the PNG/WAV/MP4 inputs in `evaluation/multimodal/fixtures` to the other supported extension families. They contain no user library data.

Created using local FFmpeg 8.0 (Homebrew revision 2) from the existing fixtures:

- `image.jpg`: `red-circle.png`, default JPEG encoder.
- `audio.mp3`: `budget.wav`, `-c:a libmp3lame`.
- `audio.flac`: `budget.wav`, `-c:a flac`.
- `audio.ogg`: `budget.wav`, `-c:a libvorbis`.
- `video.webm`: `temporal-video.mp4`, `-c:v libvpx-vp9 -c:a libopus`.

Each command used `ffmpeg -v error -nostdin -y -i INPUT [codec arguments] OUTPUT`. `manifest.json` records sizes and hashes of these fixture bytes. Production smoke testing invokes `media::prepare`; no local fixture generator is needed on Windows CI.
