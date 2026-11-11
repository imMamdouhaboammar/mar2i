use crate::policy::MediaProfile;
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct InspectedMedia {
    pub profile: MediaProfile,
    pub duration: f64,
    pub input_bytes: u64,
    pub audio_count: usize,
    pub subtitle_count: usize,
    pub chapter_count: usize,
    pub rotation: Option<i32>,
    pub original_format: String,
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
}

#[derive(Debug, Deserialize)]
struct ProbeStream {
    codec_type: Option<String>,
    codec_name: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    r_frame_rate: Option<String>,
    avg_frame_rate: Option<String>,
    bit_rate: Option<String>,
    pix_fmt: Option<String>,
    color_transfer: Option<String>,
    duration: Option<String>,
    tags: Option<Value>,
    side_data_list: Option<Vec<Value>>,
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
    if let Some(value) = stream.tags.as_ref()
        .and_then(|tags| tags.get("rotate"))
        .and_then(Value::as_str)
        .and_then(|raw| raw.parse::<i32>().ok())
    {
        return Some(value);
    }
    stream.side_data_list.as_ref()?.iter().find_map(|side| {
        side.get("rotation")
            .and_then(|value| value.as_i64().or_else(|| value.as_str()?.parse::<i64>().ok()))
            .and_then(|value| i32::try_from(value).ok())
    })
}

pub fn inspect(ffprobe: &Path, path: &Path) -> Result<InspectedMedia> {
    let metadata = fs::symlink_metadata(path).with_context(|| format!("cannot stat {}", path.display()))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        bail!("source must be a regular file, not a symbolic link");
    }

    let result = Command::new(ffprobe)
        .args(["-v", "error", "-show_format", "-show_streams", "-show_chapters", "-of", "json"])
        .arg(path)
        .output()
        .with_context(|| format!("cannot launch ffprobe: {}", ffprobe.display()))?;
    if !result.status.success() {
        bail!("ffprobe failed: {}", String::from_utf8_lossy(&result.stderr).trim());
    }
    let parsed: ProbeResult = serde_json::from_slice(&result.stdout).context("invalid ffprobe JSON")?;
    let format = parsed.format.context("ffprobe returned no container information")?;
    let videos: Vec<&ProbeStream> = parsed.streams.iter()
        .filter(|stream| stream.codec_type.as_deref() == Some("video")).collect();
    let video = videos.first().context("file has no video stream")?;
    let audios: Vec<&ProbeStream> = parsed.streams.iter()
        .filter(|stream| stream.codec_type.as_deref() == Some("audio")).collect();
    let subtitles = parsed.streams.iter()
        .filter(|stream| stream.codec_type.as_deref() == Some("subtitle")).count();

    let avg = rational_rate(video.avg_frame_rate.as_deref());
    let nominal = rational_rate(video.r_frame_rate.as_deref());
    let duration = format.duration.as_deref()
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
        video_bitrate: video.bit_rate.as_deref()
            .or(format.bit_rate.as_deref())
            .and_then(|rate| rate.parse().ok()),
        pix_fmt: video.pix_fmt.clone().unwrap_or_default(),
        color_transfer: video.color_transfer.clone(),
        audio_codecs: audios.iter().map(|stream| stream.codec_name.clone().unwrap_or_default()).collect(),
        subtitle_count: subtitles,
        video_count: videos.len(),
    };

    Ok(InspectedMedia {
        profile,
        duration,
        input_bytes: metadata.len(),
        audio_count: audios.len(),
        subtitle_count: subtitles,
        chapter_count: parsed.chapters.len(),
        rotation: rotation(video),
        original_format: format.format_name,
    })
}
