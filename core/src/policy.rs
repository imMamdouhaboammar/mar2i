use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CodecPreference {
    #[default]
    Auto,
    H264,
    Hevc,
    Av1,
    Vp9,
    Ffv1,
    #[serde(rename = "smart_abr")]
    SmartAbr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioPlan {
    Copy,
    TranscodeAac { bitrate_kbps: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Publishing,
    Archival,
}

pub fn validate_container_compatibility(codec: &str, container: &str) -> Result<(), &'static str> {
    match codec {
        "ffv1" => {
            if matches!(container, "mkv" | "avi") {
                Ok(())
            } else {
                Err("FFV1 is only supported in MKV or AVI containers")
            }
        }
        "libvpx-vp9" | "vp9" => {
            if matches!(container, "webm" | "mkv") {
                Ok(())
            } else {
                Err("VP9 is supported in WebM and MKV containers")
            }
        }
        "libx264" | "h264" => {
            if matches!(container, "mp4" | "mkv" | "mov") {
                Ok(())
            } else {
                Err("H.264 is supported in MP4, MKV, and MOV containers")
            }
        }
        "libx265" | "hevc" => {
            if matches!(container, "mp4" | "mkv" | "mov") {
                Ok(())
            } else {
                Err("HEVC is supported in MP4, MKV, and MOV containers")
            }
        }
        "libsvtav1" | "av1" => {
            if matches!(container, "mp4" | "mkv" | "webm") {
                Ok(())
            } else {
                Err("AV1 is supported in MP4, MKV, and WebM containers")
            }
        }
        _ => Ok(()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaClass {
    Uncompressed,
    Lossless,
    Mezzanine,
    Lossy,
    Modern,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaProfile {
    pub codec: String,
    pub container: String,
    pub width: u32,
    pub height: u32,
    pub frame_rate: f64,
    pub variable_frame_rate: bool,
    pub video_bitrate: Option<u64>,
    pub pix_fmt: String,
    pub color_transfer: Option<String>,
    pub audio_codecs: Vec<String>,
    pub subtitle_count: usize,
    pub video_count: usize,
}

impl MediaProfile {
    pub fn classify(&self) -> MediaClass {
        match self.codec.as_str() {
            "rawvideo" | "v210" | "v410" => MediaClass::Uncompressed,
            "ffv1" | "huffyuv" | "ffvhuff" | "utvideo" | "ljpeg" => MediaClass::Lossless,
            "prores" | "dnxhd" | "dnxhr" | "cfhd" => MediaClass::Mezzanine,
            "hevc" | "av1" | "vp9" => MediaClass::Modern,
            "h264" | "mpeg4" | "mpeg2video" | "vp8" | "mjpeg" | "wmv3" => MediaClass::Lossy,
            _ => MediaClass::Unsupported,
        }
    }

    fn is_hdr(&self) -> bool {
        matches!(
            self.color_transfer.as_deref(),
            Some("smpte2084" | "arib-std-b67" | "smpte428")
        )
    }

    fn bits_per_pixel_per_frame(&self) -> Option<f64> {
        let denominator = self.width as f64 * self.height as f64 * self.frame_rate;
        if denominator == 0.0 || !denominator.is_finite() {
            return None;
        }
        self.video_bitrate.map(|rate| rate as f64 / denominator)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Copy {
        reason: String,
    },
    Encode {
        codec: &'static str,
        container: &'static str,
        crf: u8,
    },
    EncodeAbr {
        codec: &'static str,
        container: &'static str,
        target_kbps: u32,
        preset: &'static str,
        min_ssim_bps: u16,
        audio_plan: AudioPlan,
    },
    Skip {
        reason: String,
    },
}

pub fn compute_target_abr_kbps(
    input_bytes: u64,
    duration_sec: f64,
    audio_bitrate_bps: u64,
    width: u32,
    height: u32,
    fps: f64,
    target_ratio: f64,
) -> Result<u32, &'static str> {
    if duration_sec <= 0.0 || !duration_sec.is_finite() {
        return Err("invalid duration");
    }
    if width == 0 || height == 0 || fps <= 0.0 {
        return Err("invalid video dimensions or frame rate");
    }

    let total_target_kbps = (input_bytes as f64 * 8.0 * target_ratio) / (duration_sec * 1000.0);
    let audio_kbps = if audio_bitrate_bps > 0 {
        (audio_bitrate_bps as f64 / 1000.0).clamp(64.0, 320.0)
    } else {
        0.0
    };

    let raw_video_kbps = ((total_target_kbps - audio_kbps - 18.5) / 50.0).round() * 50.0;
    let min_safe_kbps = (width as f64 * height as f64 * fps * 0.032) / 1000.0;

    let target_video_kbps = if raw_video_kbps < min_safe_kbps {
        min_safe_kbps.round() as u32
    } else {
        raw_video_kbps.max(300.0) as u32
    };

    Ok(target_video_kbps)
}

fn smart_abr_plan(profile: &MediaProfile) -> Decision {
    if profile.is_hdr() {
        return Decision::Skip {
            reason: "HDR needs an approved color-preserving publishing profile".into(),
        };
    }
    if profile.subtitle_count != 0 {
        return Decision::Skip {
            reason: "subtitle streams need an explicitly supported publication container".into(),
        };
    }
    let audio_plan = if profile.audio_codecs.iter().all(|c| c == "aac") {
        AudioPlan::Copy
    } else {
        AudioPlan::TranscodeAac { bitrate_kbps: 128 }
    };

    let min_safe_kbps =
        (profile.width as f64 * profile.height as f64 * profile.frame_rate * 0.032) / 1000.0;
    if let Some(video_bps) = profile.video_bitrate {
        let video_kbps = video_bps as f64 / 1000.0;
        if video_kbps <= min_safe_kbps {
            return Decision::Copy {
                reason: "already at minimum safe perceptual bitrate floor".into(),
            };
        }
    }

    let target_kbps = if let Some(video_bps) = profile.video_bitrate {
        let half_kbps = ((video_bps as f64 * 0.50 / 1000.0 - 18.5) / 50.0).round() * 50.0;
        if half_kbps < min_safe_kbps {
            min_safe_kbps.round() as u32
        } else {
            half_kbps.max(300.0) as u32
        }
    } else {
        2700
    };

    Decision::EncodeAbr {
        codec: "libx264",
        container: "mp4",
        target_kbps,
        preset: "fast",
        min_ssim_bps: 9450,
        audio_plan,
    }
}

pub fn choose_plan(profile: &MediaProfile, mode: Mode) -> Decision {
    choose_plan_with_preference(profile, mode, CodecPreference::Auto, None)
}

pub fn choose_plan_with_preference(
    profile: &MediaProfile,
    mode: Mode,
    preference: CodecPreference,
    available_encoders: Option<&[String]>,
) -> Decision {
    if profile.video_count != 1 || profile.width == 0 || profile.height == 0 {
        return Decision::Skip {
            reason: "requires exactly one decodable video stream".into(),
        };
    }

    let media_class = profile.classify();
    if media_class == MediaClass::Unsupported {
        return Decision::Skip {
            reason: format!("unsupported video codec: {}", profile.codec),
        };
    }

    match mode {
        Mode::Archival => {
            if matches!(
                preference,
                CodecPreference::H264
                    | CodecPreference::Hevc
                    | CodecPreference::Av1
                    | CodecPreference::Vp9
            ) {
                return Decision::Skip {
                    reason: "lossy codec rejected for archival mode; mathematical lossless preservation required"
                        .into(),
                };
            }
            match media_class {
                MediaClass::Uncompressed => {
                    if profile.variable_frame_rate {
                        return Decision::Copy {
                            reason: "unusual source timing; keeping original for archival fidelity"
                                .into(),
                        };
                    }
                    if let Some(available) = available_encoders
                        && !available.iter().any(|e| e == "ffv1")
                    {
                        return Decision::Skip {
                            reason:
                                "required encoder 'ffv1' is not available in ffmpeg installation"
                                    .into(),
                        };
                    }
                    Decision::Encode {
                        codec: "ffv1",
                        container: "mkv",
                        crf: 0,
                    }
                }
                _ => Decision::Copy {
                    reason: "preserve existing encoded source without claiming improved fidelity"
                        .into(),
                },
            }
        }
        Mode::Publishing => {
            if preference == CodecPreference::Ffv1 {
                return Decision::Skip {
                    reason:
                        "FFV1 lossless archival codec is not suitable for publishing/distribution"
                            .into(),
                };
            }
            if preference == CodecPreference::SmartAbr {
                return smart_abr_plan(profile);
            }
            if profile.is_hdr() {
                return Decision::Skip {
                    reason: "HDR needs an approved color-preserving publishing profile".into(),
                };
            }
            if profile.subtitle_count != 0 {
                return Decision::Skip {
                    reason: "subtitle streams need an explicitly supported publication container"
                        .into(),
                };
            }
            if preference == CodecPreference::Auto
                && profile.codec == "prores"
                && profile.audio_codecs.iter().any(|codec| codec != "aac")
            {
                return Decision::Skip {
                    reason: "audio codec needs an explicit preservation or transcode contract"
                        .into(),
                };
            }
            if preference == CodecPreference::Auto
                && profile.codec == "h264"
                && profile.variable_frame_rate
            {
                return Decision::Copy {
                    reason: "variable frame timing retained by copying original".into(),
                };
            }

            let (target_codec, container, crf) = match preference {
                CodecPreference::Auto => {
                    if media_class == MediaClass::Modern {
                        return Decision::Copy {
                            reason:
                                "efficient source: compatibility must be checked before replacing"
                                    .into(),
                        };
                    }
                    if profile.codec == "h264"
                        && let Some(bpp) = profile.bits_per_pixel_per_frame()
                    {
                        if bpp <= 0.042 {
                            return Decision::Copy {
                                reason: "already-efficient H.264 source".into(),
                            };
                        } else if bpp <= 0.14 {
                            return smart_abr_plan(profile);
                        }
                    }
                    let crf = if profile.pix_fmt.contains("10") || profile.pix_fmt.contains("12") {
                        18
                    } else if profile.width >= 3840 {
                        20
                    } else {
                        21
                    };
                    ("libx264", "mp4", crf)
                }
                CodecPreference::H264 => {
                    if profile.codec == "h264"
                        && profile
                            .bits_per_pixel_per_frame()
                            .is_some_and(|bpp| bpp <= 0.042)
                    {
                        return Decision::Copy {
                            reason: "already-efficient H.264 source".into(),
                        };
                    }
                    let crf = if profile.pix_fmt.contains("10") || profile.pix_fmt.contains("12") {
                        18
                    } else if profile.width >= 3840 {
                        20
                    } else {
                        21
                    };
                    ("libx264", "mp4", crf)
                }
                CodecPreference::Hevc => {
                    if profile.codec == "hevc" {
                        return Decision::Copy {
                            reason: "already-efficient HEVC source".into(),
                        };
                    }
                    let crf = if profile.pix_fmt.contains("10")
                        || profile.pix_fmt.contains("12")
                        || profile.width >= 3840
                    {
                        22
                    } else {
                        24
                    };
                    ("libx265", "mp4", crf)
                }
                CodecPreference::Av1 => {
                    if profile.codec == "av1" {
                        return Decision::Copy {
                            reason: "already-efficient AV1 source".into(),
                        };
                    }
                    let crf = if profile.width >= 3840 { 26 } else { 28 };
                    ("libsvtav1", "mp4", crf)
                }
                CodecPreference::Vp9 => {
                    if profile.codec == "vp9" {
                        return Decision::Copy {
                            reason: "already-efficient VP9 source".into(),
                        };
                    }
                    let crf = if profile.width >= 3840 { 22 } else { 24 };
                    ("libvpx-vp9", "webm", crf)
                }
                CodecPreference::Ffv1 => unreachable!(),
                CodecPreference::SmartAbr => unreachable!(),
            };

            if let Some(available) = available_encoders
                && !available.iter().any(|e| e == target_codec)
            {
                return Decision::Skip {
                    reason: format!(
                        "required encoder '{target_codec}' is not available in ffmpeg installation"
                    ),
                };
            }

            Decision::Encode {
                codec: target_codec,
                container,
                crf,
            }
        }
    }
}
