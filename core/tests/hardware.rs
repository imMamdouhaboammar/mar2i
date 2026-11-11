use mar2i_core::hardware::{
    HardwareApi, HardwareEncoder, HardwareReport, candidate_encoders, default_cpu_encoder,
};
use mar2i_core::policy::Mode;

#[test]
fn candidate_encoders_are_platform_aware() {
    let candidates = candidate_encoders();
    #[cfg(target_os = "macos")]
    {
        assert!(candidates.iter().any(|c| c.encoder == "h264_videotoolbox"));
        assert!(candidates.iter().any(|c| c.encoder == "hevc_videotoolbox"));
    }
    #[cfg(windows)]
    {
        assert!(candidates.iter().any(|c| c.encoder == "h264_nvenc"));
    }
    #[cfg(target_os = "linux")]
    {
        assert!(candidates.iter().any(|c| c.encoder == "h264_nvenc"));
        assert!(candidates.iter().any(|c| c.encoder == "h264_vaapi"));
    }
}

#[test]
fn archival_mode_strictly_rejects_hardware_acceleration() {
    let report = HardwareReport {
        available_encoders: vec![HardwareEncoder {
            codec: "hevc".into(),
            encoder: "hevc_videotoolbox".into(),
            api: HardwareApi::VideoToolbox,
            verified: true,
        }],
        recommended_h264: Some("h264_videotoolbox".into()),
        recommended_hevc: Some("hevc_videotoolbox".into()),
    };

    // Even if use_hw is requested, Archival mode MUST return cpu encoder "ffv1"
    let selected = report.select_encoder("ffv1", Mode::Archival, true);
    assert_eq!(selected, "ffv1");

    let selected_hevc = report.select_encoder("hevc", Mode::Archival, true);
    assert_eq!(selected_hevc, "libx265");

    // Publishing mode permits hardware encoder
    let pub_selected = report.select_encoder("hevc", Mode::Publishing, true);
    assert_eq!(pub_selected, "hevc_videotoolbox");

    // When use_hw is false in Publishing mode, it falls back to CPU
    let cpu_pub = report.select_encoder("hevc", Mode::Publishing, false);
    assert_eq!(cpu_pub, "libx265");
}

#[test]
fn default_cpu_encoders_cover_all_supported_formats() {
    assert_eq!(default_cpu_encoder("h264"), "libx264");
    assert_eq!(default_cpu_encoder("hevc"), "libx265");
    assert_eq!(default_cpu_encoder("av1"), "libsvtav1");
    assert_eq!(default_cpu_encoder("ffv1"), "ffv1");
}
