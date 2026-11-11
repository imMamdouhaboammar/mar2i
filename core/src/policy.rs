#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Publishing,
    Archival,
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

#[derive(Debug, Clone)]
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
    Copy { reason: String },
    Encode { codec: &'static str, container: &'static str, crf: u8 },
    Skip { reason: String },
}

pub fn choose_plan(profile: &MediaProfile, mode: Mode) -> Decision {
    if profile.video_count != 1 || profile.width == 0 || profile.height == 0 {
        return Decision::Skip { reason: "requires exactly one decodable video stream".into() };
    }

    let media_class = profile.classify();
    if media_class == MediaClass::Unsupported {
        return Decision::Skip { reason: format!("unsupported video codec: {}", profile.codec) };
    }

    match mode {
        Mode::Archival => match media_class {
            MediaClass::Uncompressed => {
                if profile.variable_frame_rate {
                    return Decision::Copy { reason: "unusual source timing; keeping original for archival fidelity".into() };
                }
                Decision::Encode { codec: "ffv1", container: "mkv", crf: 0 }
            }
            _ => Decision::Copy {
                reason: "preserve existing encoded source without claiming improved fidelity".into(),
            },
        },
        Mode::Publishing => {
            if profile.is_hdr() {
                return Decision::Skip { reason: "HDR needs an approved color-preserving publishing profile".into() };
            }
            if profile.subtitle_count != 0 {
                return Decision::Skip { reason: "subtitle streams need an explicitly supported publication container".into() };
            }
            if profile.audio_codecs.iter().any(|codec| codec != "aac") {
                return Decision::Skip { reason: "audio codec needs an explicit preservation or transcode contract".into() };
            }
            if profile.variable_frame_rate {
                return Decision::Copy { reason: "variable frame timing retained by copying original".into() };
            }
            if media_class == MediaClass::Modern {
                return Decision::Copy { reason: "efficient source: compatibility must be checked before replacing".into() };
            }
            if profile.codec == "h264"
                && profile.bits_per_pixel_per_frame().is_some_and(|bpp| bpp <= 0.14)
            {
                return Decision::Copy { reason: "already-efficient H.264 source".into() };
            }
            let crf = if profile.pix_fmt.contains("10") || profile.pix_fmt.contains("12") {
                18
            } else if profile.width >= 3840 {
                20
            } else {
                21
            };
            Decision::Encode { codec: "libx264", container: "mp4", crf }
        }
    }
}
