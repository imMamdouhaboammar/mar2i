//! Stratified Multi-Sample Bitrate & Size Prediction Engine.
//!
//! Experimental size forecasts from short encoded video samples.
//! Forecasts are approximate, exclude audio and other streams, and are not
//! quality equivalence or guaranteed savings. Failed sampling returns an error.

use crate::media::InspectedMedia;
use crate::policy::Mode;
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::Builder;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SizeEstimate {
    pub original_bytes: u64,
    pub estimated_output_bytes: u64,
    pub projected_savings_bytes: i64,
    pub projected_savings_percent: i32,
    pub sample_count: usize,
    pub confidence_level: String, // "empirical_sampled" | "analytical_modeled"
    pub estimated_duration_sec: f64,
}

/// Estimates the compressed size of a video using empirical stratified sampling.
pub fn estimate_video_size(
    ffmpeg: &Path,
    source: &Path,
    media: &InspectedMedia,
    target_codec: &str,
    crf: u8,
    mode: Mode,
) -> Result<SizeEstimate> {
    let original_bytes = media.input_bytes;

    // Only source-preserving archival copies have a known size before processing.
    // Predicting FFV1 savings from a universal percentage is not evidence.
    if mode == Mode::Archival {
        let est = match media.profile.classify() {
            crate::policy::MediaClass::Uncompressed => {
                bail!("archival FFV1 size forecast unavailable without measured encoding samples")
            }
            _ => original_bytes,
        };
        let savings = original_bytes as i64 - est as i64;
        let pct = if original_bytes > 0 {
            ((savings as f64 / original_bytes as f64) * 100.0).round() as i32
        } else {
            0
        };
        return Ok(SizeEstimate {
            original_bytes,
            estimated_output_bytes: est,
            projected_savings_bytes: savings,
            projected_savings_percent: pct,
            sample_count: 0,
            confidence_level: "archival_contract".into(),
            estimated_duration_sec: media.duration,
        });
    }

    // A missing encoder or failed sampling run must not become a made-up forecast.
    let total_duration = get_media_duration_seconds(media);
    if total_duration <= 0.0 || !ffmpeg.is_file() {
        bail!("empirical size estimate unavailable: video duration or FFmpeg missing")
    }
    let (sample_est, sample_count) =
        run_stratified_samples(ffmpeg, source, total_duration, target_codec, crf)?;
    let savings = original_bytes as i64 - sample_est as i64;
    let pct = if original_bytes > 0 {
        ((savings as f64 / original_bytes as f64) * 100.0).round() as i32
    } else {
        0
    };
    Ok(SizeEstimate {
        original_bytes,
        estimated_output_bytes: sample_est,
        projected_savings_bytes: savings,
        projected_savings_percent: pct,
        sample_count,
        confidence_level: "empirical_sampled".into(),
        estimated_duration_sec: total_duration,
    })
}

fn get_media_duration_seconds(media: &InspectedMedia) -> f64 {
    if media.duration > 0.0 {
        return media.duration;
    }
    if let Some(bitrate) = media.profile.video_bitrate
        && bitrate > 0
    {
        return (media.input_bytes * 8) as f64 / bitrate as f64;
    }
    0.0 // Unknown duration must remain unknown
}

fn run_stratified_samples(
    ffmpeg: &Path,
    source: &Path,
    total_duration: f64,
    codec: &str,
    crf: u8,
) -> Result<(u64, usize)> {
    let sample_points = if total_duration <= 10.0 {
        vec![0.0]
    } else {
        vec![
            total_duration * 0.15,
            total_duration * 0.50,
            total_duration * 0.85,
        ]
    };

    let sample_duration = 1.0f64;
    let mut total_sample_bytes = 0u64;
    let mut total_sample_sec = 0.0f64;
    let mut successful_samples = 0usize;

    let encoder = match codec {
        "h264" => "libx264",
        "hevc" => "libx265",
        "av1" => "libsvtav1",
        "vp9" => "libvpx-vp9",
        _ => "libx264",
    };

    let temp_dir = Builder::new().prefix("mar2i-est-").tempdir()?;

    for (i, &start_time) in sample_points.iter().enumerate() {
        let sample_out = temp_dir.path().join(format!("sample_{i}.mp4"));
        let status = Command::new(ffmpeg)
            .args([
                "-hide_banner",
                "-nostdin",
                "-ss",
                &format!("{:.2}", start_time),
                "-t",
                &format!("{:.2}", sample_duration),
                "-i",
            ])
            .arg(source)
            .args([
                "-c:v",
                encoder,
                "-crf",
                &crf.to_string(),
                "-preset",
                "veryfast",
                "-an",
                "-f",
                "mp4",
                "-y",
            ])
            .arg(&sample_out)
            .output();

        if let Ok(res) = status
            && res.status.success()
            && sample_out.is_file()
            && let Ok(meta) = fs::metadata(&sample_out)
        {
            total_sample_bytes += meta.len();
            total_sample_sec += sample_duration;
            successful_samples += 1;
        }
    }

    if total_sample_sec > 0.0 {
        let byte_rate_per_sec = total_sample_bytes as f64 / total_sample_sec;
        let projected = (byte_rate_per_sec * total_duration) as u64;
        return Ok((projected, successful_samples));
    }

    bail!("sampling yielded zero bytes")
}

pub fn analytical_estimate(media: &InspectedMedia, codec: &str, crf: u8) -> Result<SizeEstimate> {
    let original_bytes = media.input_bytes;

    // Normalized CRF factor: CRF 23 is standard reference (1.0)
    // Each 6 steps in CRF roughly halves or doubles bitrate (exponential scale)
    let crf_ratio = 2.0f64.powf((23.0 - crf as f64) / 6.0);

    // Demonstration-only assumptions; never call this from production forecasts
    let codec_efficiency = match codec {
        "av1" => 0.50,  // AV1 is ~50% more efficient than H.264 at same visual fidelity
        "hevc" => 0.65, // HEVC is ~35% more efficient than H.264
        "vp9" => 0.70,
        "h264" => 1.00,
        _ => 1.00,
    };

    let estimated = ((original_bytes as f64) * codec_efficiency * crf_ratio)
        .min(original_bytes as f64 * 1.5) as u64;
    let savings = original_bytes as i64 - estimated as i64;
    let pct = if original_bytes > 0 {
        ((savings as f64 / original_bytes as f64) * 100.0).round() as i32
    } else {
        0
    };

    Ok(SizeEstimate {
        original_bytes,
        estimated_output_bytes: estimated,
        projected_savings_bytes: savings,
        projected_savings_percent: pct,
        sample_count: 0,
        confidence_level: "analytical_modeled".into(),
        estimated_duration_sec: get_media_duration_seconds(media),
    })
}
