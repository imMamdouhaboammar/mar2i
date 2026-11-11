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
    };

    let est_hevc = analytical_estimate(&media, "hevc", 22)?;
    assert!(est_hevc.estimated_output_bytes < media.input_bytes, "HEVC CRF 22 must compress 150Mbps ProRes");
    assert!(est_hevc.projected_savings_bytes > 0);
    assert!(est_hevc.projected_savings_percent >= 25);

    let est_av1 = analytical_estimate(&media, "av1", 28)?;
    assert!(est_av1.estimated_output_bytes < est_hevc.estimated_output_bytes, "AV1 CRF 28 should compress further than HEVC 22");
    assert!(est_av1.projected_savings_percent >= 40);

    Ok(())
}
