use mar2i_core::policy::{
    CodecPreference, Decision, MediaClass, MediaProfile, Mode, choose_plan,
    choose_plan_with_preference, validate_container_compatibility,
};

fn example(codec: &str) -> MediaProfile {
    MediaProfile {
        codec: codec.to_owned(),
        container: "mov".to_owned(),
        width: 1920,
        height: 1080,
        frame_rate: 25.0,
        variable_frame_rate: false,
        video_bitrate: Some(90_000_000),
        pix_fmt: "yuv422p10le".to_owned(),
        color_transfer: Some("bt709".into()),
        audio_codecs: vec!["aac".into()],
        subtitle_count: 0,
        video_count: 1,
    }
}

#[test]
fn mov_extension_does_not_imply_uncompressed() {
    assert_eq!(example("prores").classify(), MediaClass::Mezzanine);
}

#[test]
fn raw_video_gets_lossless_archive_plan() {
    let p = example("rawvideo");
    assert_eq!(p.classify(), MediaClass::Uncompressed);
    assert!(matches!(
        choose_plan(&p, Mode::Archival),
        Decision::Encode {
            codec: "ffv1",
            container: "mkv",
            ..
        }
    ));
}

#[test]
fn lossy_source_is_not_reframed_as_lossless_archive() {
    assert!(matches!(
        choose_plan(&example("prores"), Mode::Archival),
        Decision::Copy { .. }
    ));
}

#[test]
fn compressed_h264_moderate_bitrate_is_smart_abr_compressed() {
    let mut p = example("h264");
    p.video_bitrate = Some(2_000_000);
    p.pix_fmt = "yuv420p".into();
    let plan = choose_plan(&p, Mode::Publishing);
    assert!(matches!(
        plan,
        Decision::EncodeAbr { target_kbps, .. } if target_kbps > 0
    ));
}

#[test]
fn real_world_web_mp4_with_moderate_bitrate_is_smart_abr_encoded_in_auto_mode() {
    let mut p = example("h264");
    // 5.57 Mbps at 1080p30 = BPP 0.0895 (above 0.042 and below 0.14)
    p.video_bitrate = Some(5_570_000);
    p.frame_rate = 30.0;
    p.pix_fmt = "yuv420p".into();
    let plan = choose_plan(&p, Mode::Publishing);
    assert!(matches!(plan, Decision::EncodeAbr { target_kbps, .. } if target_kbps > 0));
}

#[test]
fn non_aac_audio_is_permitted_with_explicit_codec_preference() {
    let mut p = example("h264");
    p.audio_codecs = vec!["pcm_s16le".into()];
    let plan = choose_plan_with_preference(&p, Mode::Publishing, CodecPreference::H264, None);
    assert!(matches!(
        plan,
        Decision::Encode {
            codec: "libx264",
            ..
        }
    ));
}

#[test]
fn vfr_video_is_compressed_with_explicit_codec_preference() {
    let mut p = example("h264");
    p.variable_frame_rate = true;
    let plan = choose_plan_with_preference(&p, Mode::Publishing, CodecPreference::H264, None);
    assert!(matches!(
        plan,
        Decision::Encode {
            codec: "libx264",
            ..
        }
    ));
}

#[test]
fn high_bitrate_source_uses_compatible_publishing_codec() {
    let p = example("prores");
    assert!(matches!(
        choose_plan(&p, Mode::Publishing),
        Decision::Encode {
            codec: "libx264",
            container: "mp4",
            ..
        }
    ));
}

#[test]
fn hdr_source_is_rejected_until_color_pipeline_is_verified() {
    let mut p = example("prores");
    p.color_transfer = Some("smpte2084".into());
    assert!(matches!(
        choose_plan(&p, Mode::Publishing),
        Decision::Skip { .. }
    ));
}

#[test]
fn variable_frame_rate_is_compressed_preserving_timestamps() {
    let mut p = example("h264");
    p.variable_frame_rate = true;
    let plan = choose_plan(&p, Mode::Publishing);
    assert!(matches!(
        plan,
        Decision::Encode {
            codec: "libx264",
            ..
        } | Decision::EncodeAbr { .. }
    ));
}

#[test]
fn subtitles_are_never_silently_dropped() {
    let mut p = example("prores");
    p.subtitle_count = 1;
    assert!(matches!(
        choose_plan(&p, Mode::Publishing),
        Decision::Skip { .. }
    ));
}

#[test]
fn incompatible_audio_is_reported_instead_of_dropped() {
    let mut p = example("prores");
    p.audio_codecs = vec!["pcm_s24le".into()];
    assert!(matches!(
        choose_plan(&p, Mode::Publishing),
        Decision::Skip { .. }
    ));
}

#[test]
fn manual_codec_preferences_generate_correct_plans() {
    let p = example("prores");

    // HEVC
    let hevc_plan = choose_plan_with_preference(&p, Mode::Publishing, CodecPreference::Hevc, None);
    assert_eq!(
        hevc_plan,
        Decision::Encode {
            codec: "libx265",
            container: "mp4",
            crf: 22,
        }
    );

    // AV1
    let av1_plan = choose_plan_with_preference(&p, Mode::Publishing, CodecPreference::Av1, None);
    assert_eq!(
        av1_plan,
        Decision::Encode {
            codec: "libsvtav1",
            container: "mp4",
            crf: 28,
        }
    );

    // VP9
    let vp9_plan = choose_plan_with_preference(&p, Mode::Publishing, CodecPreference::Vp9, None);
    assert_eq!(
        vp9_plan,
        Decision::Encode {
            codec: "libvpx-vp9",
            container: "webm",
            crf: 24,
        }
    );
}

#[test]
fn lossy_codecs_are_strictly_rejected_in_archival_mode() {
    let p = example("rawvideo");
    for pref in [
        CodecPreference::H264,
        CodecPreference::Hevc,
        CodecPreference::Av1,
        CodecPreference::Vp9,
    ] {
        let plan = choose_plan_with_preference(&p, Mode::Archival, pref, None);
        assert!(
            matches!(plan, Decision::Skip { ref reason } if reason.contains("lossless")),
            "Archival mode must reject lossy preference {pref:?}"
        );
    }
}

#[test]
fn ffv1_is_rejected_in_publishing_mode() {
    let p = example("prores");
    let plan = choose_plan_with_preference(&p, Mode::Publishing, CodecPreference::Ffv1, None);
    assert!(matches!(plan, Decision::Skip { ref reason } if reason.contains("archival")));
}

#[test]
fn missing_runtime_encoder_skips_plan_with_informative_reason() {
    let p = example("prores");
    let available = vec!["libx264".to_string(), "ffv1".to_string()];
    let plan =
        choose_plan_with_preference(&p, Mode::Publishing, CodecPreference::Av1, Some(&available));
    assert_eq!(
        plan,
        Decision::Skip {
            reason: "required encoder 'libsvtav1' is not available in ffmpeg installation".into()
        }
    );
}

#[test]
fn container_compatibility_validation_rules() {
    assert!(validate_container_compatibility("ffv1", "mkv").is_ok());
    assert!(validate_container_compatibility("ffv1", "mp4").is_err());
    assert!(validate_container_compatibility("libvpx-vp9", "webm").is_ok());
    assert!(validate_container_compatibility("libvpx-vp9", "mkv").is_ok());
    assert!(validate_container_compatibility("libvpx-vp9", "mp4").is_err());
    assert!(validate_container_compatibility("libx264", "mp4").is_ok());
    assert!(validate_container_compatibility("libx264", "webm").is_err());
    assert!(validate_container_compatibility("libx265", "mp4").is_ok());
    assert!(validate_container_compatibility("libsvtav1", "mp4").is_ok());
    assert!(validate_container_compatibility("libsvtav1", "mkv").is_ok());
}
