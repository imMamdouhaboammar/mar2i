use mar2i_core::policy::{Decision, MediaClass, MediaProfile, Mode, choose_plan};

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
fn compressed_h264_below_size_threshold_is_copied_not_reencoded() {
    let mut p = example("h264");
    p.video_bitrate = Some(2_000_000);
    p.pix_fmt = "yuv420p".into();
    assert!(matches!(
        choose_plan(&p, Mode::Publishing),
        Decision::Copy { .. }
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
fn variable_frame_rate_is_not_forced_to_cfr() {
    let mut p = example("h264");
    p.variable_frame_rate = true;
    assert!(matches!(
        choose_plan(&p, Mode::Publishing),
        Decision::Copy { .. }
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
