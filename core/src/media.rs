use crate::policy::MediaProfile;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct VideoDetails {
    pub profile: Option<String>,
    pub bit_depth: Option<u8>,
    pub sample_aspect_ratio: Option<String>,
    pub display_aspect_ratio: Option<String>,
    pub color_range: Option<String>,
    pub color_space: Option<String>,
    pub color_primaries: Option<String>,
    pub color_transfer: Option<String>,
    pub interlaced: bool,
    pub time_base: Option<String>,
    pub start_time: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AudioStreamInfo {
    pub codec: String,
    pub channels: u32,
    pub channel_layout: Option<String>,
    pub sample_rate: Option<u32>,
    pub language: Option<String>,
    pub bit_rate: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SubtitleStreamInfo {
    pub codec: String,
    pub language: Option<String>,
    pub is_default: bool,
    pub is_forced: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectedMedia {
    pub profile: MediaProfile,
    pub duration: f64,
    pub input_bytes: u64,
    pub audio_count: usize,
    pub subtitle_count: usize,
    pub chapter_count: usize,
    pub rotation: Option<i32>,
    pub original_format: String,
    pub video_details: Option<VideoDetails>,
    #[serde(default)]
    pub audio_streams: Vec<AudioStreamInfo>,
    #[serde(default)]
    pub subtitle_streams: Vec<SubtitleStreamInfo>,
    pub start_time: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct ProbeResult {
    format: Option<ProbeFormat>,
    #[serde(default)]
    streams: Vec<ProbeStream>,
    #[serde(default)]
    chapters: Vec<Value>,
}

#[derive(Debug, Deserialize)]
struct ProbeFormat {
    #[serde(default)]
    format_name: String,
    duration: Option<String>,
    bit_rate: Option<String>,
    start_time: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProbeStream {
    codec_type: Option<String>,
    codec_name: Option<String>,
    profile: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    sample_aspect_ratio: Option<String>,
    display_aspect_ratio: Option<String>,
    r_frame_rate: Option<String>,
    avg_frame_rate: Option<String>,
    bit_rate: Option<String>,
    bits_per_raw_sample: Option<String>,
    pix_fmt: Option<String>,
    color_range: Option<String>,
    color_space: Option<String>,
    color_transfer: Option<String>,
    color_primaries: Option<String>,
    field_order: Option<String>,
    time_base: Option<String>,
    start_time: Option<String>,
    channels: Option<u32>,
    channel_layout: Option<String>,
    sample_rate: Option<String>,
    duration: Option<String>,
    disposition: Option<Value>,
    tags: Option<Value>,
    side_data_list: Option<Vec<Value>>,
}

fn bit_depth(stream: &ProbeStream) -> Option<u8> {
    if let Some(raw) = stream
        .bits_per_raw_sample
        .as_deref()
        .and_then(|b| b.parse::<u8>().ok())
        .filter(|&r| r > 0)
    {
        return Some(raw);
    }
    if let Some(pix) = stream.pix_fmt.as_deref() {
        if pix.contains("16") {
            return Some(16);
        }
        if pix.contains("12") {
            return Some(12);
        }
        if pix.contains("10") {
            return Some(10);
        }
        if pix.starts_with("yuv420p")
            || pix.starts_with("yuv422p")
            || pix.starts_with("yuv444p")
            || pix.starts_with("rgb")
            || pix.starts_with("bgr")
        {
            return Some(8);
        }
    }
    None
}

fn is_interlaced(stream: &ProbeStream) -> bool {
    matches!(
        stream.field_order.as_deref(),
        Some("tt" | "bb" | "tb" | "bt" | "interlaced")
    )
}

fn rational_rate(value: Option<&str>) -> Option<f64> {
    let text = value?;
    let (num, denominator) = text.split_once('/')?;
    let top = num.parse::<f64>().ok()?;
    let bottom = denominator.parse::<f64>().ok()?;
    if bottom == 0.0 {
        return None;
    }
    let value = top / bottom;
    (value.is_finite() && value > 0.0).then_some(value)
}

fn rotation(stream: &ProbeStream) -> Option<i32> {
    if let Some(value) = stream
        .tags
        .as_ref()
        .and_then(|tags| tags.get("rotate"))
        .and_then(Value::as_str)
        .and_then(|raw| raw.parse::<i32>().ok())
    {
        return Some(value);
    }
    stream.side_data_list.as_ref()?.iter().find_map(|side| {
        side.get("rotation")
            .and_then(|value| {
                value
                    .as_i64()
                    .or_else(|| value.as_str()?.parse::<i64>().ok())
            })
            .and_then(|value| i32::try_from(value).ok())
    })
}

pub fn inspect(ffprobe: &Path, path: &Path) -> Result<InspectedMedia> {
    let metadata =
        fs::symlink_metadata(path).with_context(|| format!("cannot stat {}", path.display()))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        bail!("source must be a regular file, not a symbolic link");
    }

    let mut command = Command::new(ffprobe);
    command
        .args([
            "-hide_banner",
            "-v",
            "error",
            "-show_format",
            "-show_streams",
            "-show_chapters",
            "-of",
            "json",
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .arg(path);

    let mut child = command
        .spawn()
        .with_context(|| format!("cannot launch ffprobe: {}", ffprobe.display()))?;

    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(10);
    let poll = std::time::Duration::from_millis(50);
    let mut finished = false;

    while start.elapsed() < timeout {
        match child.try_wait() {
            Ok(Some(_)) => {
                finished = true;
                break;
            }
            Ok(None) => std::thread::sleep(poll),
            Err(e) => bail!("error waiting for ffprobe: {e}"),
        }
    }

    if !finished {
        let _ = child.kill();
        let _ = child.wait();
        bail!("ffprobe inspection timed out after 10s for {}", path.display());
    }

    let result = child
        .wait_with_output()
        .context("failed to collect ffprobe output")?;

    if !result.status.success() {
        bail!(
            "ffprobe failed: {}",
            String::from_utf8_lossy(&result.stderr).trim()
        );
    }
    let parsed: ProbeResult =
        serde_json::from_slice(&result.stdout).context("invalid ffprobe JSON")?;
    let format = parsed
        .format
        .context("ffprobe returned no container information")?;
    let videos: Vec<&ProbeStream> = parsed
        .streams
        .iter()
        .filter(|stream| stream.codec_type.as_deref() == Some("video"))
        .collect();
    let video = videos.first().context("file has no video stream")?;
    let audios: Vec<&ProbeStream> = parsed
        .streams
        .iter()
        .filter(|stream| stream.codec_type.as_deref() == Some("audio"))
        .collect();
    let subtitles = parsed
        .streams
        .iter()
        .filter(|stream| stream.codec_type.as_deref() == Some("subtitle"))
        .count();

    let avg = rational_rate(video.avg_frame_rate.as_deref());
    let nominal = rational_rate(video.r_frame_rate.as_deref());
    let duration = format
        .duration
        .as_deref()
        .or(video.duration.as_deref())
        .and_then(|value| value.parse::<f64>().ok())
        .context("video duration is missing or unsupported")?;
    if !duration.is_finite() || duration <= 0.0 {
        bail!("invalid or zero video duration");
    }

    let profile = MediaProfile {
        codec: video.codec_name.clone().unwrap_or_default(),
        container: format.format_name.clone(),
        width: video.width.unwrap_or(0),
        height: video.height.unwrap_or(0),
        frame_rate: avg.or(nominal).unwrap_or(0.0),
        // Frame-rate metadata can flag VFR, but only packet timestamps can prove CFR.
        variable_frame_rate: match (avg, nominal) {
            (Some(a), Some(b)) => (a - b).abs() > 0.01,
            _ => true,
        },
        video_bitrate: video
            .bit_rate
            .as_deref()
            .or(format.bit_rate.as_deref())
            .and_then(|rate| rate.parse().ok()),
        pix_fmt: video.pix_fmt.clone().unwrap_or_default(),
        color_transfer: video.color_transfer.clone(),
        audio_codecs: audios
            .iter()
            .map(|stream| stream.codec_name.clone().unwrap_or_default())
            .collect(),
        subtitle_count: subtitles,
        video_count: videos.len(),
    };

    let video_details = VideoDetails {
        profile: video.profile.clone(),
        bit_depth: bit_depth(video),
        sample_aspect_ratio: video.sample_aspect_ratio.clone(),
        display_aspect_ratio: video.display_aspect_ratio.clone(),
        color_range: video.color_range.clone(),
        color_space: video.color_space.clone(),
        color_primaries: video.color_primaries.clone(),
        color_transfer: video.color_transfer.clone(),
        interlaced: is_interlaced(video),
        time_base: video.time_base.clone(),
        start_time: video.start_time.clone(),
    };

    let audio_streams: Vec<AudioStreamInfo> = audios
        .iter()
        .map(|stream| AudioStreamInfo {
            codec: stream.codec_name.clone().unwrap_or_default(),
            channels: stream.channels.unwrap_or(0),
            channel_layout: stream.channel_layout.clone(),
            sample_rate: stream.sample_rate.as_deref().and_then(|r| r.parse().ok()),
            language: stream
                .tags
                .as_ref()
                .and_then(|t| t.get("language"))
                .and_then(Value::as_str)
                .map(|s| s.to_string()),
            bit_rate: stream.bit_rate.as_deref().and_then(|b| b.parse().ok()),
        })
        .collect();

    let subtitle_streams: Vec<SubtitleStreamInfo> = parsed
        .streams
        .iter()
        .filter(|stream| stream.codec_type.as_deref() == Some("subtitle"))
        .map(|stream| {
            let is_default = stream
                .disposition
                .as_ref()
                .and_then(|d| d.get("default"))
                .and_then(|v| v.as_i64())
                .is_some_and(|v| v == 1);
            let is_forced = stream
                .disposition
                .as_ref()
                .and_then(|d| d.get("forced"))
                .and_then(|v| v.as_i64())
                .is_some_and(|v| v == 1);
            SubtitleStreamInfo {
                codec: stream.codec_name.clone().unwrap_or_default(),
                language: stream
                    .tags
                    .as_ref()
                    .and_then(|t| t.get("language"))
                    .and_then(Value::as_str)
                    .map(|s| s.to_string()),
                is_default,
                is_forced,
            }
        })
        .collect();

    let start_time = format
        .start_time
        .as_deref()
        .or(video.start_time.as_deref())
        .and_then(|s| s.parse::<f64>().ok());

    Ok(InspectedMedia {
        profile,
        duration,
        input_bytes: metadata.len(),
        audio_count: audios.len(),
        subtitle_count: subtitles,
        chapter_count: parsed.chapters.len(),
        rotation: rotation(video),
        original_format: format.format_name,
        video_details: Some(video_details),
        audio_streams,
        subtitle_streams,
        start_time,
    })
}
