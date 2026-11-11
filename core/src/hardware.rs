//! Hardware acceleration detection and capability probing.
//! Probes GPU-backed hardware encoders at runtime and enforces strict policy boundaries.
//!
//! Policy Invariant:
//! Hardware acceleration is PERMITTED for Publishing mode to accelerate throughput,
//! but STRICTLY FORBIDDEN for Archival mode where FFV1 lossless mathematical bit-preservation
//! is mandatory.

use crate::policy::Mode;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HardwareApi {
    VideoToolbox,
    Nvenc,
    Qsv,
    Amf,
    Vaapi,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardwareEncoder {
    pub codec: String,
    pub encoder: String,
    pub api: HardwareApi,
    pub verified: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardwareReport {
    pub available_encoders: Vec<HardwareEncoder>,
    pub recommended_h264: Option<String>,
    pub recommended_hevc: Option<String>,
}

impl HardwareReport {
    /// Returns the active encoder string for the given target codec and mode.
    /// In Archival mode, hardware acceleration is NEVER used.
    pub fn select_encoder<'a>(
        &'a self,
        target_codec: &'a str,
        mode: Mode,
        use_hw: bool,
    ) -> &'a str {
        if mode == Mode::Archival || !use_hw {
            return default_cpu_encoder(target_codec);
        }

        match target_codec {
            "h264" => self.recommended_h264.as_deref().unwrap_or("libx264"),
            "hevc" => self.recommended_hevc.as_deref().unwrap_or("libx265"),
            _ => default_cpu_encoder(target_codec),
        }
    }
}

pub fn default_cpu_encoder(codec: &str) -> &'static str {
    match codec {
        "h264" => "libx264",
        "hevc" => "libx265",
        "av1" => "libsvtav1",
        "vp9" => "libvpx-vp9",
        "ffv1" => "ffv1",
        _ => "copy",
    }
}

pub fn candidate_encoders() -> Vec<HardwareEncoder> {
    let mut candidates = Vec::new();

    #[cfg(target_os = "macos")]
    {
        candidates.push(HardwareEncoder {
            codec: "h264".into(),
            encoder: "h264_videotoolbox".into(),
            api: HardwareApi::VideoToolbox,
            verified: false,
        });
        candidates.push(HardwareEncoder {
            codec: "hevc".into(),
            encoder: "hevc_videotoolbox".into(),
            api: HardwareApi::VideoToolbox,
            verified: false,
        });
    }

    #[cfg(windows)]
    {
        // NVIDIA
        candidates.push(HardwareEncoder {
            codec: "h264".into(),
            encoder: "h264_nvenc".into(),
            api: HardwareApi::Nvenc,
            verified: false,
        });
        candidates.push(HardwareEncoder {
            codec: "hevc".into(),
            encoder: "hevc_nvenc".into(),
            api: HardwareApi::Nvenc,
            verified: false,
        });
        // Intel QSV
        candidates.push(HardwareEncoder {
            codec: "h264".into(),
            encoder: "h264_qsv".into(),
            api: HardwareApi::Qsv,
            verified: false,
        });
        candidates.push(HardwareEncoder {
            codec: "hevc".into(),
            encoder: "hevc_qsv".into(),
            api: HardwareApi::Qsv,
            verified: false,
        });
        // AMD AMF
        candidates.push(HardwareEncoder {
            codec: "h264".into(),
            encoder: "h264_amf".into(),
            api: HardwareApi::Amf,
            verified: false,
        });
        candidates.push(HardwareEncoder {
            codec: "hevc".into(),
            encoder: "hevc_amf".into(),
            api: HardwareApi::Amf,
            verified: false,
        });
    }

    #[cfg(target_os = "linux")]
    {
        candidates.push(HardwareEncoder {
            codec: "h264".into(),
            encoder: "h264_nvenc".into(),
            api: HardwareApi::Nvenc,
            verified: false,
        });
        candidates.push(HardwareEncoder {
            codec: "hevc".into(),
            encoder: "hevc_nvenc".into(),
            api: HardwareApi::Nvenc,
            verified: false,
        });
        candidates.push(HardwareEncoder {
            codec: "h264".into(),
            encoder: "h264_vaapi".into(),
            api: HardwareApi::Vaapi,
            verified: false,
        });
        candidates.push(HardwareEncoder {
            codec: "hevc".into(),
            encoder: "hevc_vaapi".into(),
            api: HardwareApi::Vaapi,
            verified: false,
        });
    }

    candidates
}

/// Probes whether a specific hardware encoder is supported by executing a 1-frame null transcode.
pub fn probe_encoder(ffmpeg: &Path, encoder_name: &str) -> bool {
    let output = Command::new(ffmpeg)
        .args([
            "-hide_banner",
            "-nostdin",
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=16x16:d=0.1",
            "-c:v",
            encoder_name,
            "-f",
            "null",
            "-",
        ])
        .output();

    match output {
        Ok(res) => res.status.success(),
        Err(_) => false,
    }
}

/// Discovers all available hardware encoders on the host machine.
pub fn detect_hardware(ffmpeg: &Path) -> HardwareReport {
    let candidates = candidate_encoders();
    let mut verified = Vec::new();

    let mut recommended_h264 = None;
    let mut recommended_hevc = None;

    for mut candidate in candidates {
        if probe_encoder(ffmpeg, &candidate.encoder) {
            candidate.verified = true;
            if candidate.codec == "h264" && recommended_h264.is_none() {
                recommended_h264 = Some(candidate.encoder.clone());
            }
            if candidate.codec == "hevc" && recommended_hevc.is_none() {
                recommended_hevc = Some(candidate.encoder.clone());
            }
            verified.push(candidate);
        }
    }

    HardwareReport {
        available_encoders: verified,
        recommended_h264,
        recommended_hevc,
    }
}
