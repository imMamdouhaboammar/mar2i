use mar2i_core::media::inspect;
use mar2i_core::policy::MediaClass;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

fn binary(name: &str) -> PathBuf {
    std::env::var(format!("MAR2I_TEST_{}", name.to_uppercase()))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(name))
}

#[test]
fn ffprobe_detects_actual_streams_in_synthetic_video() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("الاختبار.mp4");
    let result = Command::new(binary("ffmpeg"))
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=320x180:rate=24",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=48000",
            "-t",
            "1",
            "-c:v",
            "libx264",
            "-crf",
            "18",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-shortest",
            "-y",
        ])
        .arg(&input)
        .status()
        .unwrap();
    assert!(result.success());
    let media = inspect(&binary("ffprobe"), &input).unwrap();
    assert_eq!(media.profile.codec, "h264");
    assert_eq!(media.profile.width, 320);
    assert_eq!(media.profile.height, 180);
    assert_eq!(media.profile.audio_codecs, ["aac"]);
    assert_eq!(media.profile.classify(), MediaClass::Lossy);
    assert_eq!(media.audio_count, 1);
    assert!(media.duration >= 0.90 && media.duration <= 1.1);
    assert!(media.input_bytes > 0);
}
