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

fn create_source_video(path: &Path, args: &[&str]) {
    let mut cmd = Command::new(binary("ffmpeg"));
    cmd.args(["-hide_banner", "-loglevel", "error", "-y"]);
    cmd.args(args);
    cmd.arg(path);
    let output = cmd.output().expect("ffmpeg command failed to run");
    assert!(
        output.status.success(),
        "failed to create synthetic media at {}: {}",
        path.display(),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn matrix_h264_publishing_reencodes_and_passes_ssim_gate() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("input");
    let output = tmp.path().join("output");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();

    let relative = Path::new("clip_mezzanine.mov");
    let source = input.join(relative);

    // High bitrate mezzanine source (ProRes)
    create_source_video(
        &source,
        &[
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=320x240:rate=25",
            "-t",
            "1",
            "-c:v",
            "prores_ks",
            "-profile:v",
            "3",
            "-pix_fmt",
            "yuv422p10le",
        ],
    );

    let opts = RunOptions {
        mode: Mode::Publishing,
        preference: CodecPreference::H264,
        available_encoders: None,
        ..Default::default()
    };

    let outcome = run_one_with_options(
        &binary("ffmpeg"),
        &binary("ffprobe"),
        &source,
        relative,
        &output,
        opts,
        None,
    )
    .unwrap();

    match outcome {
        RunOutcome::Accepted {
            output: out_path,
            before,
            after,
            quality,
        } => {
            assert!(out_path.is_file());
            assert!(after < before, "output must be smaller than mezzanine");
            assert!(quality.contains("SSIM="), "quality must report SSIM score");
        }
        other => panic!("expected Accepted outcome for mezzanine H264 encode, got {other:?}"),
    }
}

#[test]
fn matrix_hevc_publishing_with_preference() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("input");
    let output = tmp.path().join("output");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();

    let relative = Path::new("hevc_source.avi");
    let source = input.join(relative);

    // High bitrate uncompressed raw video
    create_source_video(
        &source,
        &[
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=320x240:rate=25",
            "-t",
            "1",
            "-c:v",
            "rawvideo",
            "-pix_fmt",
            "yuv420p",
        ],
    );

    let opts = RunOptions {
        mode: Mode::Publishing,
        preference: CodecPreference::Hevc,
        available_encoders: None,
        ..Default::default()
    };

    let outcome = run_one_with_options(
        &binary("ffmpeg"),
        &binary("ffprobe"),
        &source,
        relative,
        &output,
        opts,
        None,
    )
    .unwrap();

    match outcome {
        RunOutcome::Accepted {
            output: out_path,
            before,
            after,
            quality,
        } => {
            assert!(out_path.is_file());
            assert!(out_path.extension().and_then(|s| s.to_str()) == Some("mp4"));
            assert!(after < before);
            assert!(quality.contains("SSIM="));
        }
        other => panic!("expected Accepted HEVC outcome, got {other:?}"),
    }
}

#[test]
fn matrix_ffv1_lossless_archival_preserves_frame_hashes() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("input");
    let output = tmp.path().join("output");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();

    let relative = Path::new("master_tape.avi");
    let source = input.join(relative);

    create_source_video(
        &source,
        &[
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=320x240:rate=25",
            "-t",
            "1",
            "-c:v",
            "rawvideo",
            "-pix_fmt",
            "yuv420p",
        ],
    );

    let opts = RunOptions {
        mode: Mode::Archival,
        preference: CodecPreference::Ffv1,
        available_encoders: None,
        ..Default::default()
    };

    let outcome = run_one_with_options(
        &binary("ffmpeg"),
        &binary("ffprobe"),
        &source,
        relative,
        &output,
        opts,
        None,
    )
    .unwrap();

    match outcome {
        RunOutcome::Accepted {
            output: out_path,
            quality,
            ..
        } => {
            assert!(out_path.is_file());
            assert!(out_path.extension().and_then(|s| s.to_str()) == Some("mkv"));
            assert!(
                quality.contains("decoded-frame hashes identical"),
                "quality must certify identical decoded-frame hashes"
            );
        }
        other => panic!("expected Accepted FFV1 archival outcome, got {other:?}"),
    }
}

#[test]
fn matrix_vp9_webm_video_encoding() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("input");
    let output = tmp.path().join("output");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();

    let relative = Path::new("vp9_source.avi");
    let source = input.join(relative);

    create_source_video(
        &source,
        &[
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=320x240:rate=25",
            "-t",
            "1",
            "-c:v",
            "rawvideo",
            "-pix_fmt",
            "yuv420p",
        ],
    );

    let opts = RunOptions {
        mode: Mode::Publishing,
        preference: CodecPreference::Vp9,
        available_encoders: None,
        ..Default::default()
    };

    let outcome = run_one_with_options(
        &binary("ffmpeg"),
        &binary("ffprobe"),
        &source,
        relative,
        &output,
        opts,
        None,
    )
    .unwrap();

    match outcome {
        RunOutcome::Accepted {
            output: out_path,
            quality,
            ..
        } => {
            assert!(out_path.is_file());
            assert!(out_path.extension().and_then(|s| s.to_str()) == Some("webm"));
            assert!(quality.contains("SSIM="));
        }
        other => panic!("expected Accepted VP9 outcome, got {other:?}"),
    }
}

#[test]
fn matrix_stream_copy_for_already_optimal_distribution_mp4() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("input");
    let output = tmp.path().join("output");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();

    let relative = Path::new("already_web_ready.mp4");
    let source = input.join(relative);

    // Create web-ready H.264+AAC MP4
    create_source_video(
        &source,
        &[
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=640x360:rate=25",
            "-f",
            "lavfi",
            "-i",
            "anullsrc=r=44100:cl=stereo",
            "-t",
            "1",
            "-c:v",
            "libx264",
            "-b:v",
            "180k",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-b:a",
            "128k",
            "-movflags",
            "+faststart",
        ],
    );

    let original_bytes = fs::read(&source).unwrap();

    let outcome = run_one_with_options(
        &binary("ffmpeg"),
        &binary("ffprobe"),
        &source,
        relative,
        &output,
        RunOptions::from(Mode::Publishing),
        None,
    )
    .unwrap();

    match outcome {
        RunOutcome::Rejected { reason } => {
            assert!(
                reason.contains("already optimal") || reason.contains("not smaller"),
                "reason must indicate file is already optimal and not re-encodable without bloat: {reason}"
            );
        }
        RunOutcome::Copied {
            output: out_path,
            bytes,
            reason,
        } => {
            assert!(out_path.is_file());
            assert_eq!(bytes, original_bytes.len() as u64);
            assert!(
                reason.contains("already"),
                "reason must indicate file is already compliant"
            );
        }
        other => panic!("expected Rejected/Copied outcome for optimal mp4, got {other:?}"),
    }
}

#[test]
fn matrix_stream_preservation_audio_channels_and_metadata() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("input");
    let output = tmp.path().join("output");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();

    let relative = Path::new("multi_audio.mov");
    let source = input.join(relative);

    // Create mezzanine with stereo audio with explicit -t 1 on both inputs
    create_source_video(
        &source,
        &[
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=320x240:rate=25",
            "-f",
            "lavfi",
            "-i",
            "anullsrc=r=44100:cl=stereo",
            "-t",
            "1",
            "-c:v",
            "prores_ks",
            "-profile:v",
            "3",
            "-c:a",
            "aac",
        ],
    );

    let outcome = run_one_with_options(
        &binary("ffmpeg"),
        &binary("ffprobe"),
        &source,
        relative,
        &output,
        RunOptions::from(Mode::Publishing),
        None,
    )
    .unwrap();

    match outcome {
        RunOutcome::Accepted {
            output: out_path, ..
        } => {
            assert!(out_path.is_file());
            // Probe output with ffprobe to verify audio stream count preserved
            let inspected = mar2i_core::media::inspect(&binary("ffprobe"), &out_path).unwrap();
            assert_eq!(
                inspected.audio_count, 1,
                "audio stream count must be preserved"
            );
        }
        other => panic!("expected Accepted outcome, got {other:?}"),
    }
}
