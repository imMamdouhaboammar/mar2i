use mar2i_core::estimate::analytical_estimate;
use mar2i_core::media::InspectedMedia;
use mar2i_core::policy::MediaProfile;

#[test]
fn analytical_estimate_computes_logical_savings_by_codec_and_crf() -> anyhow::Result<()> {
    let media = InspectedMedia {
        profile: MediaProfile {
            codec: "prores".into(),
            container: "mov".into(),
            width: 3840,
            height: 2160,
            frame_rate: 60.0,
            variable_frame_rate: false,
            video_bitrate: Some(150_000_000), // 150 Mbps ProRes
            pix_fmt: "yuv422p10le".into(),
            color_transfer: None,
            audio_codecs: vec!["pcm_s24le".into()],
            subtitle_count: 0,
            video_count: 1,
        },
        duration: 60.0,
        input_bytes: 1_125_000_000, // ~1.125 GB
        audio_count: 1,
        subtitle_count: 0,
        chapter_count: 0,
        rotation: None,
        original_format: "mov".into(),
        video_details: None,
        audio_streams: vec![],
        subtitle_streams: vec![],
        start_time: None,
    };

    let est_hevc = analytical_estimate(&media, "hevc", 22)?;
    assert!(
        est_hevc.estimated_output_bytes < media.input_bytes,
        "HEVC CRF 22 must compress 150Mbps ProRes"
    );
    assert!(est_hevc.projected_savings_bytes > 0);
    assert!(est_hevc.projected_savings_percent >= 25);

    let est_av1 = analytical_estimate(&media, "av1", 28)?;
    assert!(
        est_av1.estimated_output_bytes < est_hevc.estimated_output_bytes,
        "AV1 CRF 28 should compress further than HEVC 22"
    );
    assert!(est_av1.projected_savings_percent >= 40);

    Ok(())
}

#[test]
fn missing_encoder_does_not_invent_measured_savings() {
    use mar2i_core::estimate::estimate_video_size;
    use mar2i_core::policy::Mode;
    use std::path::Path;

    let media = InspectedMedia {
        profile: MediaProfile {
            codec: "prores".into(),
            container: "mov".into(),
            width: 1920,
            height: 1080,
            frame_rate: 25.0,
            variable_frame_rate: false,
            video_bitrate: Some(80_000_000),
            pix_fmt: "yuv422p10le".into(),
            color_transfer: None,
            audio_codecs: vec![],
            subtitle_count: 0,
            video_count: 1,
        },
        duration: 20.0,
        input_bytes: 100_000_000,
        audio_count: 0,
        subtitle_count: 0,
        chapter_count: 0,
        rotation: None,
        original_format: "mov".into(),
        video_details: None,
        audio_streams: vec![],
        subtitle_streams: vec![],
        start_time: None,
    };

    let estimate = estimate_video_size(
        Path::new("/nonexistent-mar2i-ffmpeg"),
        Path::new("/nonexistent-mar2i-source.mov"),
        &media,
        "av1",
        23,
        Mode::Publishing,
    );
    assert!(
        estimate.is_err(),
        "a failed encoder probe must never turn into a fabricated forecast"
    );
}

#[test]
fn measured_short_clip_reports_actual_number_of_samples() -> anyhow::Result<()> {
    use mar2i_core::estimate::estimate_video_size;
    use mar2i_core::media::inspect;
    use mar2i_core::policy::Mode;
    use std::path::PathBuf;
    use std::process::Command;

    let Ok(ffmpeg) = std::env::var("MAR2I_TEST_FFMPEG") else {
        eprintln!("FFmpeg integration test requires MAR2I_TEST_FFMPEG");
        return Ok(());
    };
    let ffprobe = std::env::var("MAR2I_TEST_FFPROBE")?;
    let dir = tempfile::tempdir()?;
    let source = dir.path().join("short.mp4");
    let output = Command::new(&ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=64x64:rate=10",
            "-t",
            "2",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-an",
            "-y",
        ])
        .arg(&source)
        .output()?;
    assert!(output.status.success(), "failed to create synthetic source");

    let media = inspect(&PathBuf::from(&ffprobe), &source)?;
    let estimated = estimate_video_size(
        &PathBuf::from(ffmpeg),
        &source,
        &media,
        "h264",
        23,
        Mode::Publishing,
    )?;
    assert_eq!(
        estimated.sample_count, 1,
        "a short clip has one sampling window, not three"
    );
    assert_eq!(estimated.confidence_level, "empirical_sampled");
    Ok(())
}
