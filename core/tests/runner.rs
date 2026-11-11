use mar2i_core::policy::Mode;
use mar2i_core::runner::{run_one, RunOutcome};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

fn binary(name: &str) -> PathBuf {
    std::env::var(format!("MAR2I_TEST_{}", name.to_uppercase()))
        .map(PathBuf::from).unwrap_or_else(|_| PathBuf::from(name))
}

fn generate(path: &Path, codec: &str) {
    let mut cmd = Command::new(binary("ffmpeg"));
    cmd.args(["-hide_banner", "-loglevel", "error", "-f", "lavfi",
        "-i", "testsrc2=size=320x180:rate=24", "-t", "1",
        "-c:v", codec, "-y"]);
    if codec == "rawvideo" {
        cmd.args(["-pix_fmt", "yuv420p"]);
    } else if codec == "prores_ks" {
        cmd.args(["-profile:v", "3"]);
    } else {
        cmd.args(["-crf", "19", "-pix_fmt", "yuv420p"]);
    }
    assert!(cmd.arg(path).status().unwrap().success());
}

#[test]
fn archives_real_raw_frames_to_verified_ffv1_and_preserves_original() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("input");
    let output = tmp.path().join("output");
    fs::create_dir_all(input.join("أرشيف")).unwrap();
    fs::create_dir_all(&output).unwrap();
    let relative = Path::new("أرشيف/خام.avi");
    let original = input.join(relative);
    generate(&original, "rawvideo");
    let before = fs::read(&original).unwrap();
    let outcome = run_one(&binary("ffmpeg"), &binary("ffprobe"), &original, relative, &output, Mode::Archival).unwrap();
    let RunOutcome::Accepted { output: published, before: original_size, after, quality } = outcome else {
        panic!("expected accepted lossless archive");
    };
    assert!(published.starts_with(fs::canonicalize(output.join("أرشيف")).unwrap()));
    assert!(after < original_size);
    assert_eq!(quality, "decoded-frame hashes identical");
    assert_eq!(fs::read(&original).unwrap(), before);
    assert!(published.is_file());
}

#[test]
fn skips_overwriting_existing_destination_even_when_file_is_compressible() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("source");
    let output = tmp.path().join("output");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();
    let original = input.join("raw.avi");
    generate(&original, "rawvideo");
    let existing = output.join("raw.avi.mar2i.mkv");
    fs::write(&existing, b"important output from another run").unwrap();
    assert!(run_one(&binary("ffmpeg"), &binary("ffprobe"), &original, Path::new("raw.avi"), &output, Mode::Archival).is_err());
    assert_eq!(fs::read(&existing).unwrap(), b"important output from another run");
}

#[test]
fn corrupted_input_is_not_published() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("corrupt.mp4");
    let output = tmp.path().join("output");
    fs::create_dir_all(&output).unwrap();
    fs::write(&input, b"not a video").unwrap();
    assert!(run_one(&binary("ffmpeg"), &binary("ffprobe"), &input, Path::new("corrupt.mp4"), &output, Mode::Publishing).is_err());
    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
}

#[test]
fn publishing_compresses_high_bitrate_mezzanine_source_when_quality_passes() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("source");
    let output = tmp.path().join("output");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();
    let original = input.join("production.mov");
    generate(&original, "prores_ks");
    let before = fs::metadata(&original).unwrap().len();
    let result = run_one(&binary("ffmpeg"), &binary("ffprobe"), &original, Path::new("production.mov"), &output, Mode::Publishing).unwrap();
    match result {
        RunOutcome::Accepted { output: published, after, quality, .. } => {
            assert!(after < before);
            assert!(published.exists());
            assert!(quality.starts_with("SSIM="));
        }
        other => panic!("expected compressed output, got {other:?}"),
    }
}
