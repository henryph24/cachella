//! Bounded, local-only media preparation for the embedding runtime.
//!
//! The caller selects the decoder binary.  This module never accepts an URL,
//! playlist, or an input supplied to ffmpeg directly: it validates and copies
//! the user file into a private temporary directory before invoking ffmpeg.

use crate::scratch::Workspace;
use crate::store::Result;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock};
use tokio::time::{timeout, Duration};

pub const MAX_MEDIA_BYTES: usize = 32 * 1024 * 1024;
const MAX_PREPARED_BYTES: usize = 64 * 1024 * 1024;
const MAX_PARTS: usize = 36;
const MAX_AUDIO_PARTS: usize = 12;
const MAX_VIDEO_FRAMES: usize = 24;
const MAX_VIDEO_AUDIO_PARTS: usize = 12;
const MAX_DURATION_SECONDS: u64 = 120;
const MAX_DIMENSION: u64 = 8192;
const MAX_PIXELS: u64 = 20_000_000;
const OUTPUT_LIMIT: usize = 256 * 1024;
const NORMALIZATION_VERSION: &[u8] = b"cachella-media-v2";
// Clamp after aspect-ratio scaling: very thin inputs can otherwise round to zero.
const IMAGE_SCALE: &str = "scale=w='max(1,round(iw*min(1,min(512/iw,512/ih))))':h='max(1,round(ih*min(1,min(512/iw,512/ih))))'";
// yuv420p requires positive even dimensions, including for odd-sized sources.
const VIDEO_SCALE: &str = "fps=30,scale=w='max(2,2*trunc(iw*min(1,min(720/iw,720/ih))/2))':h='max(2,2*trunc(ih*min(1,min(720/iw,720/ih))/2))'";
const DECODER_MAX_ALLOC: usize = 64 * 1024 * 1024;
const DECODER_OUTPUT_BYTES: usize = MAX_MEDIA_BYTES;
const DECODER_THREADS: usize = 2;

type ManagedChild = Arc<Mutex<crate::process_guard::GuardedChild>>;
static ACTIVE_CHILDREN: OnceLock<Mutex<Registry>> = OnceLock::new();

struct Registry {
    shutting_down: bool,
    children: HashMap<u64, ManagedChild>,
    next_id: u64,
}

impl Default for Registry {
    fn default() -> Self {
        Self {
            shutting_down: false,
            children: HashMap::new(),
            next_id: 1,
        }
    }
}

struct ChildRegistration {
    id: u64,
    child: ManagedChild,
}

impl Drop for ChildRegistration {
    fn drop(&mut self) {
        registry()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .children
            .remove(&self.id);
        let _ = kill_and_reap(&self.child);
    }
}

#[derive(Debug, Clone)]
pub struct PreparedMedia {
    pub kind: String,
    pub mime: String,
    pub source: Vec<u8>,
    pub fingerprint: String,
    pub parts: Vec<MediaPart>,
}

#[derive(Debug, Clone)]
pub struct MediaPart {
    pub mime: String,
    pub data: Vec<u8>,
    pub start_ms: Option<u64>,
    pub end_ms: Option<u64>,
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Image,
    Audio,
    Video,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Audio => "audio",
            Self::Video => "video",
        }
    }
}

#[derive(Deserialize)]
struct Probe {
    #[serde(default)]
    streams: Vec<ProbeStream>,
    #[serde(default)]
    format: ProbeFormat,
}

#[derive(Deserialize)]
struct ProbeStream {
    codec_type: Option<String>,
    width: Option<u64>,
    height: Option<u64>,
    r_frame_rate: Option<String>,
}

#[derive(Default, Deserialize)]
struct ProbeFormat {
    duration: Option<String>,
}

/// Validate the selected ffmpeg and its neighbouring ffprobe executable.
pub fn validate_tool(path: &Path) -> Result<PathBuf> {
    let tool = canonical_file(path, "ffmpeg")?;
    let expected = if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    };
    if tool.file_name() != Some(OsStr::new(expected)) {
        return Err("Choose the ffmpeg executable itself.".into());
    }
    let probe_name = if cfg!(windows) {
        "ffprobe.exe"
    } else {
        "ffprobe"
    };
    let probe = tool
        .parent()
        .ok_or_else(|| "The selected ffmpeg has no parent folder.".to_string())?
        .join(probe_name);
    canonical_file(&probe, "ffprobe")?;
    Ok(tool)
}

/// Locate and validate the decoder shipped beside the application resources.
///
/// The returned path is derived from the current resource directory each time,
/// so an included selection survives an application move or replacement.
pub fn included_tool(resource_dir: &Path) -> Option<PathBuf> {
    let executable = resource_dir.join("decoder").join(if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    });
    validate_tool(&executable).ok()
}

/// Terminate and reap any direct ffmpeg/ffprobe child owned by active preparation.
///
/// This is synchronous so the desktop exit callback can use it before Tauri ends
/// its runtime.  Each lock is held only for an individual child operation.
pub fn stop_all() {
    let children = {
        let mut registry = registry()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        registry.shutting_down = true;
        registry
            .children
            .drain()
            .map(|(_, child)| child)
            .collect::<Vec<_>>()
    };
    for child in children {
        let _ = kill_and_reap(&child);
    }
}

/// Produce safe embedding inputs using the local ffmpeg selected by the user.
pub async fn prepare(ffmpeg: &Path, input: &Path) -> Result<PreparedMedia> {
    let ffmpeg = validate_tool(ffmpeg)?;
    let ffprobe = sibling_probe(&ffmpeg)?;
    let kind = classify(input)?;
    let original = read_input(input)?;
    validate_signature(kind, input, &original)?;
    let fingerprint = fingerprint(&original);

    let scratch = Workspace::new()?;
    let copied = copy_to_scratch(&scratch, input, &original)?;
    let info = probe(&ffprobe, &copied, kind).await?;
    validate_probe(kind, &info)?;

    let _original_duration_ms = if kind == Kind::Image {
        0
    } else {
        duration_ms(&info)?
    };
    let (source, parts, mime) = match kind {
        Kind::Image => prepare_image(&ffmpeg, &copied, &scratch).await?,
        Kind::Audio => prepare_audio(&ffmpeg, &ffprobe, &copied, &scratch).await?,
        Kind::Video => {
            prepare_video(&ffmpeg, &ffprobe, &copied, &scratch, has_audio(&info)).await?
        }
    };
    validate_prepared(&source, &parts)?;
    Ok(PreparedMedia {
        kind: kind.as_str().into(),
        mime: mime.into(),
        source,
        fingerprint,
        parts,
    })
}

fn canonical_file(path: &Path, label: &str) -> Result<PathBuf> {
    let canonical = std::fs::canonicalize(path)
        .map_err(|_| format!("Choose an existing {label} executable."))?;
    let metadata = std::fs::metadata(&canonical)
        .map_err(|_| format!("Choose a regular {label} executable."))?;
    if !metadata.is_file() {
        return Err(format!("Choose a regular {label} executable."));
    }
    Ok(canonical)
}

fn sibling_probe(ffmpeg: &Path) -> Result<PathBuf> {
    let name = if cfg!(windows) {
        "ffprobe.exe"
    } else {
        "ffprobe"
    };
    canonical_file(
        &ffmpeg
            .parent()
            .ok_or_else(|| "The selected ffmpeg has no parent folder.".to_string())?
            .join(name),
        "ffprobe",
    )
}

fn classify(input: &Path) -> Result<Kind> {
    let ext = input
        .extension()
        .and_then(OsStr::to_str)
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| "Choose a PNG, JPG, WAV, MP3, FLAC, OGG, MP4, or WEBM file.".to_string())?;
    match ext.as_str() {
        "png" | "jpg" | "jpeg" => Ok(Kind::Image),
        "wav" | "mp3" | "flac" | "ogg" => Ok(Kind::Audio),
        "mp4" | "webm" => Ok(Kind::Video),
        _ => Err("Choose a PNG, JPG, WAV, MP3, FLAC, OGG, MP4, or WEBM file.".into()),
    }
}

fn read_input(input: &Path) -> Result<Vec<u8>> {
    let file = crate::regular_file::open_regular_file(input).map_err(|error| {
        if error.kind() == std::io::ErrorKind::InvalidInput {
            "Media files must be regular files.".to_string()
        } else {
            "Choose an existing media file.".to_string()
        }
    })?;
    read_file_limited(
        file,
        MAX_MEDIA_BYTES,
        "Media files must be nonempty regular files of at most 32 MB.",
    )
}

fn read_file_limited(file: File, limit: usize, error: &str) -> Result<Vec<u8>> {
    let mut bytes = Vec::with_capacity(limit.min(64 * 1024));
    file.take((limit + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| error.to_string())?;
    if bytes.is_empty() || bytes.len() > limit {
        return Err(error.into());
    }
    Ok(bytes)
}

fn validate_signature(kind: Kind, input: &Path, bytes: &[u8]) -> Result<()> {
    let ext = input
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let valid = match ext.as_str() {
        "png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "jpg" | "jpeg" => bytes.starts_with(b"\xff\xd8\xff"),
        "wav" => bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WAVE",
        "mp3" => {
            bytes.starts_with(b"ID3")
                || (bytes.len() > 1 && bytes[0] == 0xff && bytes[1] & 0xe0 == 0xe0)
        }
        "flac" => bytes.starts_with(b"fLaC"),
        "ogg" => bytes.starts_with(b"OggS"),
        "mp4" => bytes.len() >= 12 && &bytes[4..8] == b"ftyp",
        "webm" => bytes.starts_with(b"\x1a\x45\xdf\xa3"),
        _ => false,
    };
    if !valid {
        return Err(format!("The .{ext} file signature is invalid."));
    }
    let expected = match kind {
        Kind::Image => ["png", "jpg", "jpeg"].as_slice(),
        Kind::Audio => ["wav", "mp3", "flac", "ogg"].as_slice(),
        Kind::Video => ["mp4", "webm"].as_slice(),
    };
    if !expected.contains(&ext.as_str()) {
        return Err("The media type does not match its extension.".into());
    }
    Ok(())
}

fn copy_to_scratch(scratch: &Workspace, input: &Path, data: &[u8]) -> Result<PathBuf> {
    let ext = input
        .extension()
        .and_then(OsStr::to_str)
        .ok_or_else(|| "Media files need an extension.".to_string())?;
    let target = scratch.path().join(format!("input.{ext}"));
    std::fs::write(&target, data).map_err(|e| format!("Could not isolate media input: {e}"))?;
    Ok(target)
}

fn fingerprint(original: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(NORMALIZATION_VERSION);
    digest.update(original);
    format!("{:x}", digest.finalize())
}

async fn probe(ffprobe: &Path, input: &Path, kind: Kind) -> Result<Probe> {
    let mut args = probe_prefix();
    append_input_args(&mut args, kind, input)?;
    args.extend([
        "-show_entries".into(),
        "stream=codec_type,width,height,r_frame_rate:format=duration".into(),
        "-of".into(),
        "json".into(),
    ]);
    let output = run_strings(ffprobe, args, Duration::from_secs(15)).await?;
    if !output.success {
        let detail = String::from_utf8_lossy(&output.stderr)
            .trim()
            .replace('\n', " ");
        return Err(if detail.is_empty() {
            "ffprobe could not inspect this media file.".into()
        } else {
            format!("ffprobe could not inspect this media file: {detail}")
        });
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|_| "ffprobe returned invalid media metadata.".into())
}

fn demuxer(kind: Kind, input: &Path) -> Result<&'static str> {
    let ext = input
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    match (kind, ext.as_str()) {
        (Kind::Image, "png") => Ok("png_pipe"),
        (Kind::Image, "jpg" | "jpeg") => Ok("mjpeg"),
        (Kind::Audio, "wav") => Ok("wav"),
        (Kind::Audio, "mp3") => Ok("mp3"),
        (Kind::Audio, "flac") => Ok("flac"),
        (Kind::Audio, "ogg") => Ok("ogg"),
        (Kind::Video, "mp4") => Ok("mov"),
        (Kind::Video, "webm") => Ok("matroska"),
        _ => Err("Unsupported media extension.".into()),
    }
}

fn validate_probe(kind: Kind, info: &Probe) -> Result<()> {
    match kind {
        Kind::Image => {
            let stream = info
                .streams
                .iter()
                .find(|s| s.codec_type.as_deref() == Some("video"))
                .ok_or_else(|| "The image contains no visual stream.".to_string())?;
            let (width, height) = (stream.width.unwrap_or(0), stream.height.unwrap_or(0));
            if width == 0
                || height == 0
                || width > MAX_DIMENSION
                || height > MAX_DIMENSION
                || width.saturating_mul(height) > MAX_PIXELS
            {
                return Err(
                    "Images are limited to 8,192 pixels per side and 20 megapixels.".into(),
                );
            }
        }
        Kind::Audio => {
            if !info
                .streams
                .iter()
                .any(|s| s.codec_type.as_deref() == Some("audio"))
            {
                return Err("The audio file contains no audio stream.".into());
            }
        }
        Kind::Video => {
            let stream = info
                .streams
                .iter()
                .find(|s| s.codec_type.as_deref() == Some("video"))
                .ok_or_else(|| "The video file contains no visual stream.".to_string())?;
            validate_visual_dimensions(stream, "Videos")?;
        }
    }
    Ok(())
}

fn validate_visual_dimensions(stream: &ProbeStream, label: &str) -> Result<()> {
    let (width, height) = (stream.width.unwrap_or(0), stream.height.unwrap_or(0));
    if width == 0
        || height == 0
        || width > MAX_DIMENSION
        || height > MAX_DIMENSION
        || width.saturating_mul(height) > MAX_PIXELS
    {
        return Err(format!(
            "{label} are limited to 8,192 pixels per side and 20 megapixels."
        ));
    }
    Ok(())
}

fn validate_normalized_video(info: &Probe) -> Result<()> {
    validate_probe(Kind::Video, info)?;
    let stream = info
        .streams
        .iter()
        .find(|s| s.codec_type.as_deref() == Some("video"))
        .ok_or_else(|| "The normalized video contains no visual stream.".to_string())?;
    if stream.width.unwrap_or(0) > 720 || stream.height.unwrap_or(0) > 720 {
        return Err("Normalized video exceeds the 720-pixel dimension limit.".into());
    }
    if frame_rate(stream.r_frame_rate.as_deref())? > 30.0 {
        return Err("Normalized video exceeds the 30 FPS limit.".into());
    }
    duration_ms(info)?;
    Ok(())
}

fn frame_rate(value: Option<&str>) -> Result<f64> {
    let value = value.ok_or_else(|| "ffprobe did not report a video frame rate.".to_string())?;
    let (numerator, denominator) = value
        .split_once('/')
        .ok_or_else(|| "ffprobe reported an invalid video frame rate.".to_string())?;
    let numerator = numerator
        .parse::<f64>()
        .map_err(|_| "ffprobe reported an invalid video frame rate.".to_string())?;
    let denominator = denominator
        .parse::<f64>()
        .map_err(|_| "ffprobe reported an invalid video frame rate.".to_string())?;
    if !numerator.is_finite() || !denominator.is_finite() || denominator <= 0.0 || numerator < 0.0 {
        return Err("ffprobe reported an invalid video frame rate.".into());
    }
    Ok(numerator / denominator)
}

fn has_audio(info: &Probe) -> bool {
    info.streams
        .iter()
        .any(|stream| stream.codec_type.as_deref() == Some("audio"))
}

fn duration_ms(info: &Probe) -> Result<u64> {
    let seconds = info
        .format
        .duration
        .as_deref()
        .unwrap_or("0")
        .parse::<f64>()
        .map_err(|_| "ffprobe reported an invalid duration.".to_string())?;
    if !seconds.is_finite() || seconds <= 0.0 || seconds > MAX_DURATION_SECONDS as f64 {
        return Err("Audio and video are limited to 120 seconds.".into());
    }
    Ok((seconds * 1000.0).ceil() as u64)
}

async fn prepare_image(
    ffmpeg: &Path,
    input: &Path,
    scratch: &Workspace,
) -> Result<(Vec<u8>, Vec<MediaPart>, &'static str)> {
    let output = scratch.path().join("preview.png");
    transcode(
        ffmpeg,
        input,
        Kind::Image,
        &output,
        ["-vf", IMAGE_SCALE, "-frames:v", "1", "-c:v", "png"],
    )
    .await?;
    let source = read_output(&output)?;
    let part = MediaPart {
        mime: "image/png".into(),
        data: source.clone(),
        start_ms: None,
        end_ms: None,
        label: "Image preview".into(),
    };
    Ok((source, vec![part], "image/png"))
}

async fn prepare_audio(
    ffmpeg: &Path,
    ffprobe: &Path,
    input: &Path,
    scratch: &Workspace,
) -> Result<(Vec<u8>, Vec<MediaPart>, &'static str)> {
    let preview = scratch.path().join("preview.wav");
    transcode(
        ffmpeg,
        input,
        Kind::Audio,
        &preview,
        ["-ac", "1", "-ar", "16000", "-c:a", "pcm_s16le"],
    )
    .await?;
    let normalized = probe(ffprobe, &preview, Kind::Audio).await?;
    validate_probe(Kind::Audio, &normalized)?;
    let duration_ms = duration_ms(&normalized)?;
    let source = read_output(&preview)?;
    let parts = audio_parts(
        ffmpeg,
        &preview,
        Kind::Audio,
        scratch,
        duration_ms,
        MAX_AUDIO_PARTS,
        "Audio",
    )
    .await?;
    Ok((source, parts, "audio/wav"))
}

async fn prepare_video(
    ffmpeg: &Path,
    ffprobe: &Path,
    input: &Path,
    scratch: &Workspace,
    source_has_audio: bool,
) -> Result<(Vec<u8>, Vec<MediaPart>, &'static str)> {
    let preview = scratch.path().join("preview.mp4");
    transcode(
        ffmpeg,
        input,
        Kind::Video,
        &preview,
        [
            "-vf",
            VIDEO_SCALE,
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-ac",
            "1",
            "-movflags",
            "+faststart",
        ],
    )
    .await?;
    let normalized = probe(ffprobe, &preview, Kind::Video).await?;
    validate_normalized_video(&normalized)?;
    let duration_ms = duration_ms(&normalized)?;
    let source = read_output(&preview)?;
    let mut parts = Vec::new();
    let frame_count = duration_ms.div_ceil(5_000).min(MAX_VIDEO_FRAMES as u64);
    for index in 0..frame_count {
        let start = index * 5_000;
        let output = scratch.path().join(format!("frame-{index}.png"));
        let start_arg = format!("{:.3}", start as f64 / 1000.0);
        transcode_at(
            ffmpeg,
            input,
            Kind::Video,
            &output,
            &start_arg,
            "0.1",
            ["-vf", IMAGE_SCALE, "-frames:v", "1", "-c:v", "png"],
        )
        .await?;
        parts.push(MediaPart {
            mime: "image/png".into(),
            data: read_output(&output)?,
            start_ms: Some(start),
            end_ms: Some((start + 5_000).min(duration_ms)),
            label: format!("Video frame at {}", timestamp(start)),
        });
    }
    if source_has_audio {
        parts.extend(
            audio_parts(
                ffmpeg,
                input,
                Kind::Video,
                scratch,
                duration_ms,
                MAX_VIDEO_AUDIO_PARTS,
                "Video audio",
            )
            .await?,
        );
    }
    Ok((source, parts, "video/mp4"))
}

async fn audio_parts(
    ffmpeg: &Path,
    input: &Path,
    kind: Kind,
    scratch: &Workspace,
    duration_ms: u64,
    limit: usize,
    label: &str,
) -> Result<Vec<MediaPart>> {
    let mut parts = Vec::new();
    let count = duration_ms.div_ceil(10_000).min(limit as u64);
    for index in 0..count {
        let start = index * 10_000;
        let end = (start + 10_000).min(duration_ms);
        let output = scratch.path().join(format!("audio-{label}-{index}.wav"));
        let start_arg = format!("{:.3}", start as f64 / 1000.0);
        let length_arg = format!("{:.3}", (end - start) as f64 / 1000.0);
        transcode_at(
            ffmpeg,
            input,
            kind,
            &output,
            &start_arg,
            &length_arg,
            ["-vn", "-ac", "1", "-ar", "16000", "-c:a", "pcm_s16le"],
        )
        .await?;
        parts.push(MediaPart {
            mime: "audio/wav".into(),
            data: read_output(&output)?,
            start_ms: Some(start),
            end_ms: Some(end),
            label: format!("{label} {}–{}", timestamp(start), timestamp(end)),
        });
    }
    Ok(parts)
}

async fn transcode<const N: usize>(
    ffmpeg: &Path,
    input: &Path,
    kind: Kind,
    output: &Path,
    filters: [&str; N],
) -> Result<()> {
    let mut args = decoder_prefix();
    append_input_args(&mut args, kind, input)?;
    if kind != Kind::Image {
        args.extend(["-t".into(), "121".into()]);
    }
    append_maps(&mut args, output);
    args.extend(filters.into_iter().map(str::to_string));
    args.extend([
        "-fs".to_string(),
        DECODER_OUTPUT_BYTES.to_string(),
        "-f".to_string(),
        output_format(output)?.to_string(),
        "-n".to_string(),
        output.to_string_lossy().into_owned(),
    ]);
    ensure_success(
        run_strings(ffmpeg, args, Duration::from_secs(45)).await?,
        "ffmpeg could not normalize this media file",
    )
}

async fn transcode_at<const N: usize>(
    ffmpeg: &Path,
    input: &Path,
    kind: Kind,
    output: &Path,
    start: &str,
    length: &str,
    filters: [&str; N],
) -> Result<()> {
    let mut args = decoder_prefix();
    args.extend(["-ss".into(), start.into()]);
    append_input_args(&mut args, kind, input)?;
    args.extend(["-t".into(), length.into()]);
    append_maps(&mut args, output);
    args.extend(filters.into_iter().map(str::to_string));
    args.extend([
        "-fs".to_string(),
        DECODER_OUTPUT_BYTES.to_string(),
        "-f".to_string(),
        output_format(output)?.to_string(),
        "-n".to_string(),
        output.to_string_lossy().into_owned(),
    ]);
    ensure_success(
        run_strings(ffmpeg, args, Duration::from_secs(45)).await?,
        "ffmpeg could not prepare this media segment",
    )
}

fn decoder_prefix() -> Vec<String> {
    vec![
        "-v".into(),
        "error".into(),
        "-nostdin".into(),
        "-xerror".into(),
        "-protocol_whitelist".into(),
        "file,pipe".into(),
        "-max_alloc".into(),
        DECODER_MAX_ALLOC.to_string(),
        "-threads".into(),
        DECODER_THREADS.to_string(),
        "-filter_threads".into(),
        DECODER_THREADS.to_string(),
        "-filter_complex_threads".into(),
        DECODER_THREADS.to_string(),
    ]
}

fn probe_prefix() -> Vec<String> {
    vec![
        "-v".into(),
        "error".into(),
        "-protocol_whitelist".into(),
        "file,pipe".into(),
        "-max_alloc".into(),
        DECODER_MAX_ALLOC.to_string(),
    ]
}

fn append_input_args(args: &mut Vec<String>, kind: Kind, input: &Path) -> Result<()> {
    if kind == Kind::Video
        && input
            .extension()
            .and_then(OsStr::to_str)
            .is_some_and(|ext| ext.eq_ignore_ascii_case("mp4"))
    {
        args.extend(["-enable_drefs".into(), "0".into()]);
    }
    args.extend([
        "-f".into(),
        demuxer(kind, input)?.into(),
        "-i".into(),
        input.to_string_lossy().into_owned(),
    ]);
    Ok(())
}

fn append_maps(args: &mut Vec<String>, output: &Path) {
    match output.extension().and_then(OsStr::to_str) {
        Some("png") => args.extend(["-map".into(), "0:v:0".into()]),
        Some("wav") => args.extend(["-map".into(), "0:a:0".into()]),
        Some("mp4") => args.extend([
            "-map".into(),
            "0:v:0".into(),
            "-map".into(),
            "0:a:0?".into(),
        ]),
        _ => {}
    }
}

fn output_format(path: &Path) -> Result<&'static str> {
    match path.extension().and_then(OsStr::to_str) {
        Some("png") => Ok("image2"),
        Some("wav") => Ok("wav"),
        Some("mp4") => Ok("mp4"),
        _ => Err("Unsupported normalized media output.".into()),
    }
}

async fn run_strings(tool: &Path, args: Vec<String>, limit: Duration) -> Result<CommandOutput> {
    run_command(tool, args.into_iter().map(Into::into).collect(), limit).await
}

pub(crate) async fn run_command(
    tool: &Path,
    args: Vec<std::ffi::OsString>,
    limit: Duration,
) -> Result<CommandOutput> {
    let mut command = Command::new(tool);
    configure_command(&mut command);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let registration = spawn_registered(&mut command)?;
    let child = registration.child.clone();
    let (stdout, stderr) = {
        let mut owned = child
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        (
            owned
                .stdout
                .take()
                .ok_or_else(|| "Could not capture media tool output.".to_string())?,
            owned
                .stderr
                .take()
                .ok_or_else(|| "Could not capture media tool output.".to_string())?,
        )
    };
    let result = timeout(limit, async {
        let stdout_task = tokio::task::spawn_blocking(move || read_capped_sync(stdout));
        let stderr_task = tokio::task::spawn_blocking(move || read_capped_sync(stderr));
        let (stdout, stderr, status) = tokio::try_join!(
            async {
                stdout_task
                    .await
                    .map_err(|_| "Could not read media tool output.".to_string())?
            },
            async {
                stderr_task
                    .await
                    .map_err(|_| "Could not read media tool output.".to_string())?
            },
            wait_for_exit(child.clone()),
        )?;
        Ok::<_, String>(CommandOutput {
            success: status.success(),
            stdout,
            stderr,
        })
    })
    .await;
    drop(registration);
    match result {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(error)) => Err(error),
        Err(_) => Err("Media preparation exceeded its time limit.".into()),
    }
}

fn configure_command(command: &mut Command) {
    command.env_clear().env("LANG", "C");
    for key in ["PATH", "SYSTEMROOT", "WINDIR", "TEMP", "TMP", "TMPDIR"] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
}

fn registry() -> &'static Mutex<Registry> {
    ACTIVE_CHILDREN.get_or_init(|| Mutex::new(Registry::default()))
}

fn spawn_registered(command: &mut Command) -> Result<ChildRegistration> {
    let mut registry = registry()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if registry.shutting_down {
        return Err("Media preparation is stopping because the application is closing.".into());
    }
    let child = Arc::new(Mutex::new(
        crate::process_guard::spawn(command, true)
            .map_err(|e| format!("Could not run the selected media tool: {e}"))?,
    ));
    let id = registry.next_id;
    registry.next_id = registry.next_id.wrapping_add(1).max(1);
    registry.children.insert(id, child.clone());
    Ok(ChildRegistration { id, child })
}

async fn wait_for_exit(child: ManagedChild) -> Result<std::process::ExitStatus> {
    loop {
        if let Some(status) = child
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .try_wait()
            .map_err(|e| format!("Media tool did not exit cleanly: {e}"))?
        {
            return Ok(status);
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

fn kill_and_reap(child: &ManagedChild) -> Result<()> {
    let mut child = child
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if child
        .try_wait()
        .map_err(|e| format!("Could not inspect media tool: {e}"))?
        .is_some()
    {
        return Ok(());
    }
    child
        .kill()
        .map_err(|e| format!("Could not stop media tool: {e}"))?;
    child
        .wait()
        .map_err(|e| format!("Could not reap media tool: {e}"))?;
    Ok(())
}

fn read_capped_sync<R: Read>(mut reader: R) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|e| format!("Could not read media tool output: {e}"))?;
        if count == 0 {
            return Ok(output);
        }
        if output.len() + count > OUTPUT_LIMIT {
            return Err("Media tool produced too much diagnostic output.".into());
        }
        output.extend_from_slice(&buffer[..count]);
    }
}

#[derive(Debug)]
pub(crate) struct CommandOutput {
    pub(crate) success: bool,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

fn ensure_success(output: CommandOutput, fallback: &str) -> Result<()> {
    if output.success {
        return Ok(());
    }
    let detail = String::from_utf8_lossy(&output.stderr)
        .trim()
        .replace('\n', " ");
    Err(if detail.is_empty() {
        fallback.into()
    } else {
        format!("{fallback}: {detail}")
    })
}

fn read_output(path: &Path) -> Result<Vec<u8>> {
    let file = crate::regular_file::open_regular_file(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::InvalidInput {
            "Normalized media output is not a regular file.".to_string()
        } else {
            "Media preparation did not produce an output file.".to_string()
        }
    })?;
    read_file_limited(
        file,
        MAX_MEDIA_BYTES,
        "Normalized media is empty or exceeds 32 MB.",
    )
}

fn validate_prepared(source: &[u8], parts: &[MediaPart]) -> Result<()> {
    if parts.is_empty() || parts.len() > MAX_PARTS {
        return Err("Media preparation produced too many parts.".into());
    }
    let bytes = source.len() + parts.iter().map(|part| part.data.len()).sum::<usize>();
    if bytes > MAX_PREPARED_BYTES {
        return Err("Prepared media exceeds the 64 MB in-memory limit.".into());
    }
    Ok(())
}

fn timestamp(ms: u64) -> String {
    format!("{}:{:02}", ms / 60_000, (ms / 1_000) % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_a_mislabeled_file_before_decoder_execution() {
        let input = tempfile::NamedTempFile::with_suffix(".png").unwrap();
        std::fs::write(input.path(), b"not an image").unwrap();
        assert!(validate_signature(Kind::Image, input.path(), b"not an image").is_err());
    }

    #[test]
    fn fingerprint_changes_when_normalization_version_changes_input() {
        assert_ne!(fingerprint(b"a"), fingerprint(b"b"));
        assert_eq!(fingerprint(b"a"), fingerprint(b"a"));
    }

    #[test]
    fn bounded_reader_rejects_growth_beyond_its_limit() {
        let input = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(input.path(), b"four").unwrap();
        let error =
            read_file_limited(File::open(input.path()).unwrap(), 3, "too large").unwrap_err();
        assert_eq!(error, "too large");
        std::fs::write(input.path(), b"two").unwrap();
        assert_eq!(
            read_file_limited(File::open(input.path()).unwrap(), 3, "too large").unwrap(),
            b"two"
        );
    }

    #[test]
    fn rejects_more_than_allowed_prepared_memory() {
        let part = MediaPart {
            mime: "audio/wav".into(),
            data: vec![0; MAX_PREPARED_BYTES],
            start_ms: Some(0),
            end_ms: Some(1),
            label: "x".into(),
        };
        assert!(validate_prepared(&[1], &[part]).is_err());
    }

    #[test]
    fn accepts_real_fixture_signatures() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../evaluation/multimodal/fixtures");
        for name in ["red-circle.png", "budget.wav", "budget-video.mp4"] {
            let path = root.join(name);
            let kind = classify(&path).unwrap();
            validate_signature(kind, &path, &std::fs::read(&path).unwrap()).unwrap();
        }
    }

    #[test]
    fn resolves_included_decoder_only_when_ffmpeg_and_ffprobe_are_present() {
        let resources = tempfile::tempdir().unwrap();
        let decoder = resources.path().join("decoder");
        std::fs::create_dir(&decoder).unwrap();
        let ffmpeg = decoder.join(if cfg!(windows) {
            "ffmpeg.exe"
        } else {
            "ffmpeg"
        });
        let ffprobe = decoder.join(if cfg!(windows) {
            "ffprobe.exe"
        } else {
            "ffprobe"
        });
        std::fs::write(&ffmpeg, b"fixture").unwrap();
        assert!(included_tool(resources.path()).is_none());
        std::fs::write(&ffprobe, b"fixture").unwrap();
        assert_eq!(
            included_tool(resources.path()),
            Some(std::fs::canonicalize(ffmpeg).unwrap())
        );
    }

    #[tokio::test]
    async fn normalizes_thin_images_and_video_to_nonzero_bounded_dimensions() {
        let Some(ffmpeg) = std::env::var_os("CACHELLA_FFMPEG").map(PathBuf::from) else {
            return;
        };
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../evaluation/resources/boundaries");
        let ffprobe = sibling_probe(&ffmpeg).unwrap();
        for (name, width, height) in [
            ("side8192.png", 512, 1),
            ("portrait8192.png", 1, 512),
            ("wide-video.mp4", 720, 2),
            ("tall-video.mp4", 2, 720),
            ("odd-video.mp4", 512, 256),
        ] {
            let prepared = prepare(&ffmpeg, &root.join(name)).await.unwrap();
            let scratch = Workspace::new().unwrap();
            let output = scratch.path().join(name);
            std::fs::write(&output, &prepared.source).unwrap();
            let info = probe(&ffprobe, &output, classify(&output).unwrap())
                .await
                .unwrap();
            let visual = info
                .streams
                .iter()
                .find(|s| s.codec_type.as_deref() == Some("video"))
                .unwrap();
            assert_eq!(
                (visual.width, visual.height),
                (Some(width), Some(height)),
                "{name}"
            );
            for part in &prepared.parts {
                if part.mime == "image/png" {
                    assert_eq!(&part.data[12..16], b"IHDR");
                    let w = u32::from_be_bytes(part.data[16..20].try_into().unwrap());
                    let h = u32::from_be_bytes(part.data[20..24].try_into().unwrap());
                    assert!((1..=512).contains(&w) && (1..=512).contains(&h), "{name}");
                }
            }
        }
    }

    #[tokio::test]
    async fn prepares_real_local_multimodal_fixtures_when_ffmpeg_is_installed() {
        let ffmpeg = std::env::var_os("CACHELLA_FFMPEG")
            .map(PathBuf::from)
            .or_else(|| {
                let local = PathBuf::from("/opt/homebrew/bin/ffmpeg");
                local.exists().then_some(local)
            });
        let Some(ffmpeg) = ffmpeg else {
            return;
        };
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../evaluation/multimodal/fixtures");
        let image = prepare(&ffmpeg, &root.join("red-circle.png"))
            .await
            .unwrap();
        assert_eq!(image.kind, "image");
        assert_eq!(image.parts.len(), 1);
        let audio = prepare(&ffmpeg, &root.join("budget.wav")).await.unwrap();
        assert_eq!(audio.kind, "audio");
        assert_eq!(audio.parts.len(), 1);
        assert_eq!(audio.parts[0].mime, "audio/wav");
        let video = prepare(&ffmpeg, &root.join("budget-video.mp4"))
            .await
            .unwrap();
        assert_eq!(video.kind, "video");
        assert!(video.parts.iter().any(|part| part.mime == "image/png"));
        assert!(video.parts.iter().any(|part| part.mime == "audio/wav"));
        assert!(video.source.len() <= MAX_MEDIA_BYTES);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn caps_decoder_output_and_terminates_the_child() {
        use std::os::unix::fs::PermissionsExt;

        let scratch = tempfile::tempdir().unwrap();
        let script = scratch.path().join("noisy-tool");
        let pid_file = scratch.path().join("child.pid");
        std::fs::write(
            &script,
            b"#!/bin/sh\necho $$ > \"$1\"\nwhile :; do printf '1234567890'; done\n",
        )
        .unwrap();
        let mut permissions = std::fs::metadata(&script).unwrap().permissions();
        permissions.set_mode(0o700);
        std::fs::set_permissions(&script, permissions).unwrap();
        let error = run_strings(
            &script,
            vec![pid_file.to_string_lossy().into_owned()],
            Duration::from_secs(3),
        )
        .await
        .unwrap_err();
        assert!(error.contains("too much diagnostic output"));
        let pid = std::fs::read_to_string(pid_file).unwrap();
        assert!(process_is_gone(pid.trim()));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn aborting_a_running_tool_reaps_its_own_child() {
        use std::os::unix::fs::PermissionsExt;

        let scratch = tempfile::tempdir().unwrap();
        let script = scratch.path().join("waiting-tool");
        let pid_file = scratch.path().join("child.pid");
        std::fs::write(
            &script,
            b"#!/bin/sh\necho $$ > \"$1\"\nwhile :; do sleep 1; done\n",
        )
        .unwrap();
        let mut permissions = std::fs::metadata(&script).unwrap().permissions();
        permissions.set_mode(0o700);
        std::fs::set_permissions(&script, permissions).unwrap();
        let script_for_task = script.clone();
        let pid_argument = pid_file.to_string_lossy().into_owned();
        let task = tokio::spawn(async move {
            run_strings(
                &script_for_task,
                vec![pid_argument],
                Duration::from_secs(30),
            )
            .await
        });
        // Concurrent lifecycle tests also spawn guarded children. Wait for the
        // observable readiness marker with a bounded deadline before aborting.
        tokio::time::timeout(Duration::from_secs(5), async {
            while !pid_file.exists() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("child wrote its PID within five seconds");
        let pid = std::fs::read_to_string(&pid_file).expect("child wrote its PID");
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert!(process_is_gone(pid.trim()));
    }

    #[cfg(unix)]
    fn process_is_gone(pid: &str) -> bool {
        let output = std::process::Command::new("ps")
            .args(["-p", pid, "-o", "pid="])
            .output()
            .unwrap();
        String::from_utf8_lossy(&output.stdout).trim().is_empty()
    }
}
