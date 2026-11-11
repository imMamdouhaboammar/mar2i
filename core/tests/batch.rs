use mar2i_core::policy::Mode;
use mar2i_core::queue::BatchQueue;
use mar2i_core::report::{report_csv, summarize};
use mar2i_core::worker::{modified_ns, run_batch, Control, ResourceProfile};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

fn binary(name: &str) -> PathBuf {
    std::env::var(format!("MAR2I_TEST_{}", name.to_uppercase()))
        .map(PathBuf::from).unwrap_or_else(|_| PathBuf::from(name))
}

fn synthetic(path: &Path, codec: &str) {
    let mut command = Command::new(binary("ffmpeg"));
    command.args(["-hide_banner", "-loglevel", "error",
        "-f", "lavfi", "-i", "testsrc2=size=160x96:rate=24",
        "-t", "0.5", "-c:v", codec, "-y"]);
    if codec == "rawvideo" {
        command.args(["-pix_fmt", "yuv420p"]);
    } else {
        command.args(["-pix_fmt", "yuv420p"]);
    }
    assert!(command.arg(path).status().unwrap().success());
}

#[test]
fn real_two_file_batch_is_durable_and_reports_verified_savings() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("source");
    let output = tmp.path().join("dest");
    fs::create_dir_all(input.join("مجلد")).unwrap();
    fs::create_dir_all(&output).unwrap();
    let raw = input.join("مجلد/خام.avi");
    let corrupt = input.join("bad.mp4");
    synthetic(&raw, "rawvideo");
    fs::write(&corrupt, b"broken").unwrap();
    let db = tmp.path().join("batch.sqlite");
    let queue = BatchQueue::open(&db, &input, &output, Mode::Archival).unwrap();
    for (relative, path) in [("مجلد/خام.avi", &raw), ("bad.mp4", &corrupt)] {
        queue.enqueue(relative, path, fs::metadata(path).unwrap().len(), modified_ns(path).unwrap()).unwrap();
    }
    run_batch(&queue, &binary("ffmpeg"), &binary("ffprobe"), &output, Mode::Archival, ResourceProfile::Balanced, &Control::default()).unwrap();
    let jobs = queue.list().unwrap();
    assert_eq!(jobs.len(), 2);
    let summary = summarize(&jobs);
    assert_eq!(summary.completed, 1);
    assert_eq!(summary.failed, 1);
    assert!(summary.verified_savings_bytes > 0);
    assert!(output.join("مجلد/خام.avi.mar2i.mkv").is_file());
    assert!(report_csv(&jobs).contains("مجلد/خام.avi"));
    queue.recover().unwrap();
    assert!(queue.claim_next().unwrap().is_none());
}

#[test]
fn changed_source_is_rejected_before_any_encode() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("source");
    let output = tmp.path().join("dest");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();
    let source = input.join("film.mov");
    synthetic(&source, "prores_ks");
    let queue = BatchQueue::open(&tmp.path().join("batch.sqlite"), &input, &output, Mode::Publishing).unwrap();
    queue.enqueue("film.mov", &source, fs::metadata(&source).unwrap().len(), modified_ns(&source).unwrap()).unwrap();
    fs::write(&source, b"changed source").unwrap();
    run_batch(&queue, &binary("ffmpeg"), &binary("ffprobe"), &output, Mode::Publishing, ResourceProfile::Low, &Control::default()).unwrap();
    assert_eq!(queue.list().unwrap()[0].status, "failed");
    assert!(!output.join("film.mov.mar2i.mp4").exists());
}
