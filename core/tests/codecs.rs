use mar2i_core::hardware::probe_available_encoders;
use mar2i_core::policy::{CodecPreference, Mode};
use mar2i_core::runner::{RunOptions, RunOutcome, run_one_with_options};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

fn binary(name: &str) -> PathBuf {
    std::env::var(format!("MAR2I_TEST_{}", name.to_uppercase()))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(name))
}

fn generate(path: &Path) {
    let mut cmd = Command::new(binary("ffmpeg"));
    cmd.args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-f",
        "lavfi",
        "-i",
        "testsrc2=size=320x180:rate=24",
        "-t",
        "0.5",
        "-c:v",
        "rawvideo",
        "-pix_fmt",
        "yuv420p",
        "-y",
    ]);
    assert!(cmd.arg(path).status().unwrap().success());
}

#[test]
fn real_synthetic_encoding_to_hevc_produces_verified_mp4_and_preserves_source() {
    let ffmpeg = binary("ffmpeg");
    let ffprobe = binary("ffprobe");
    let available = probe_available_encoders(&ffmpeg);
    if !available.iter().any(|e| e == "libx265") {
        eprintln!("libx265 not available in this test environment; skipping transcode");
        return;
    }

    let tmp = tempdir().unwrap();
    let input = tmp.path().join("input");
    let output = tmp.path().join("output");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();
    let original = input.join("source.avi");
    generate(&original);
    let original_bytes = fs::read(&original).unwrap();

    let outcome = run_one_with_options(
        &ffmpeg,
        &ffprobe,
        &original,
        Path::new("source.avi"),
        &output,
        RunOptions {
            mode: Mode::Publishing,
            preference: CodecPreference::Hevc,
            available_encoders: Some(&available),
            ..Default::default()
        },
        None,
    )
    .unwrap();

    let RunOutcome::Accepted {
        output: published,
        before,
        after,
        quality,
    } = outcome
    else {
        panic!("expected accepted HEVC encoding outcome");
    };

    assert!(published.is_file());
    assert!(published.to_string_lossy().ends_with(".mar2i.mp4"));
    assert_eq!(before, original_bytes.len() as u64);
    assert!(after < before);
    assert!(quality.contains("SSIM="));
    // Verify source is pristine and unchanged
    assert_eq!(fs::read(&original).unwrap(), original_bytes);
}

#[test]
fn real_synthetic_encoding_to_vp9_produces_verified_webm_and_preserves_source() {
    let ffmpeg = binary("ffmpeg");
    let ffprobe = binary("ffprobe");
    let available = probe_available_encoders(&ffmpeg);
    if !available.iter().any(|e| e == "libvpx-vp9") {
        eprintln!("libvpx-vp9 not available in this test environment; skipping transcode");
        return;
    }

    let tmp = tempdir().unwrap();
    let input = tmp.path().join("input");
    let output = tmp.path().join("output");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();
    let original = input.join("clip.avi");
    generate(&original);
    let original_bytes = fs::read(&original).unwrap();

    let outcome = run_one_with_options(
        &ffmpeg,
        &ffprobe,
        &original,
        Path::new("clip.avi"),
        &output,
        RunOptions {
            mode: Mode::Publishing,
            preference: CodecPreference::Vp9,
            available_encoders: Some(&available),
            ..Default::default()
        },
        None,
    )
    .unwrap();

    let RunOutcome::Accepted {
        output: published,
        before,
        after,
        quality,
    } = outcome
    else {
        panic!("expected accepted VP9 encoding outcome, got: {outcome:?}");
    };

    assert!(published.is_file());
    assert!(published.to_string_lossy().ends_with(".mar2i.webm"));
    assert_eq!(before, original_bytes.len() as u64);
    assert!(after < before);
    assert!(quality.contains("SSIM="));
    // Verify source is pristine and unchanged
    assert_eq!(fs::read(&original).unwrap(), original_bytes);
}

#[test]
fn real_synthetic_encoding_to_av1_produces_verified_mp4_when_encoder_available() {
    let ffmpeg = binary("ffmpeg");
    let ffprobe = binary("ffprobe");
    let available = probe_available_encoders(&ffmpeg);
    if !available.iter().any(|e| e == "libsvtav1") {
        eprintln!("libsvtav1 not available in this test environment; skipping test");
        return;
    }

    let tmp = tempdir().unwrap();
    let input = tmp.path().join("input");
    let output = tmp.path().join("output");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();
    let original = input.join("av1_test.avi");
    generate(&original);
    let original_bytes = fs::read(&original).unwrap();

    let outcome = run_one_with_options(
        &ffmpeg,
        &ffprobe,
        &original,
        Path::new("av1_test.avi"),
        &output,
        RunOptions {
            mode: Mode::Publishing,
            preference: CodecPreference::Av1,
            available_encoders: Some(&available),
            ..Default::default()
        },
        None,
    )
    .unwrap();

    let RunOutcome::Accepted {
        output: published,
        before,
        after,
        quality,
    } = outcome
    else {
        panic!("expected accepted AV1 encoding outcome");
    };

    assert!(published.is_file());
    assert!(published.to_string_lossy().ends_with(".mar2i.mp4"));
    assert_eq!(before, original_bytes.len() as u64);
    assert!(after < before);
    assert!(quality.contains("SSIM="));
    assert_eq!(fs::read(&original).unwrap(), original_bytes);
}
