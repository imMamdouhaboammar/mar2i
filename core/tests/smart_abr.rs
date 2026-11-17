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

fn generate_sample(path: &Path) {
    let mut cmd = Command::new(binary("ffmpeg"));
    cmd.args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-f",
        "lavfi",
        "-i",
        "testsrc2=size=640x360:rate=30",
        "-f",
        "lavfi",
        "-i",
        "sine=frequency=1000:sample_rate=44100",
        "-t",
        "1.0",
        "-c:v",
        "libx264",
        "-preset",
        "ultrafast",
        "-b:v",
        "4000k",
        "-c:a",
        "aac",
        "-b:a",
        "128k",
        "-pix_fmt",
        "yuv420p",
        "-y",
    ]);
    assert!(cmd.arg(path).status().unwrap().success());
}

#[test]
fn smart_abr_two_pass_compresses_and_faststarts_mp4() {
    let ffmpeg = binary("ffmpeg");
    let ffprobe = binary("ffprobe");
    let available = probe_available_encoders(&ffmpeg);
    if !available.iter().any(|e| e == "libx264") {
        eprintln!("libx264 not available in test environment; skipping");
        return;
    }

    let tmp = tempdir().unwrap();
    let input = tmp.path().join("input");
    let output = tmp.path().join("output");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();
    let original = input.join("sample.mp4");
    generate_sample(&original);
    let original_bytes = fs::read(&original).unwrap();
    let original_size = original_bytes.len() as u64;

    let outcome = run_one_with_options(
        &ffmpeg,
        &ffprobe,
        &original,
        Path::new("sample.mp4"),
        &output,
        RunOptions {
            mode: Mode::Publishing,
            preference: CodecPreference::SmartAbr,
            available_encoders: Some(&available),
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
        panic!("expected accepted SmartAbr 2-pass outcome");
    };

    assert_eq!(before, original_size);
    assert!(published.is_file());
    assert!(
        after < before,
        "after ({after}) should be smaller than before ({before})"
    );
    assert!(
        quality.contains("SSIM"),
        "quality should report SSIM metric: {quality}"
    );

    // Verify no stray passlog files or directories in output or parent
    let mut stray_passlogs = 0;
    for entry in fs::read_dir(&output).unwrap().flatten() {
        if entry.file_name().to_string_lossy().contains("passlog") {
            stray_passlogs += 1;
        }
    }
    assert_eq!(
        stray_passlogs, 0,
        "passlog directory must be completely cleaned up"
    );

    // Verify source video was NEVER modified
    assert_eq!(
        fs::read(&original).unwrap(),
        original_bytes,
        "source file must remain immutable"
    );

    // Verify moov atom is placed early (faststart)
    let published_bytes = fs::read(&published).unwrap();
    assert!(published_bytes.len() >= 32);
    // In MP4 with faststart, moov typically appears before mdat (within the first 4KB)
    let search_window = &published_bytes[..published_bytes.len().min(4096)];
    assert!(
        search_window.windows(4).any(|w| w == b"moov"),
        "published MP4 should have moov atom near the start for faststart streaming"
    );
}

#[test]
fn benchmark_sample_mp4_matches_telemetry_when_configured() {
    let sample_env = std::env::var("MAR2I_BENCHMARK_SAMPLE_PATH").ok();
    let Some(sample_str) = sample_env else {
        return;
    };
    let sample_path = Path::new(&sample_str);
    if !sample_path.is_file() {
        return;
    }

    let ffmpeg = binary("ffmpeg");
    let ffprobe = binary("ffprobe");
    let available = probe_available_encoders(&ffmpeg);

    let tmp = tempdir().unwrap();
    let output = tmp.path().join("output");
    fs::create_dir_all(&output).unwrap();

    let outcome = run_one_with_options(
        &ffmpeg,
        &ffprobe,
        sample_path,
        Path::new("sample.mp4"),
        &output,
        RunOptions {
            mode: Mode::Publishing,
            preference: CodecPreference::SmartAbr,
            available_encoders: Some(&available),
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
        panic!("expected accepted SmartAbr outcome on benchmark sample");
    };

    let ratio = after as f64 / before as f64;
    assert!(
        (0.40..=0.60).contains(&ratio),
        "ratio {ratio:.4} should be around 0.50 (50% size reduction)"
    );
    assert!(quality.contains("SSIM"), "quality should report SSIM");
    assert!(published.is_file());
}
