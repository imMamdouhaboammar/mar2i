use mar2i_core::policy::Mode;
use mar2i_core::queue::BatchQueue;
use mar2i_core::worker::{worker_limit, run_batch, Control, ResourceProfile};
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn worker_count_is_cpu_and_memory_bounded() {
    let gib = 1024 * 1024 * 1024;
    assert_eq!(worker_limit(ResourceProfile::Low, 16, 64*gib), 1);
    assert_eq!(worker_limit(ResourceProfile::Balanced, 16, 64*gib), 2);
    assert_eq!(worker_limit(ResourceProfile::Maximum, 16, 64*gib), 4);
    assert_eq!(worker_limit(ResourceProfile::Maximum, 2, 64*gib), 1);
    assert_eq!(worker_limit(ResourceProfile::Maximum, 16, 2*gib), 1);
    assert_eq!(worker_limit(ResourceProfile::Maximum, 0, 0), 1);
}

#[test]
fn pre_cancelled_batch_does_not_claim_or_modify_pending_jobs() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("input");
    let output = tmp.path().join("output");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();
    let file = input.join("raw.avi");
    fs::write(&file, b"fake").unwrap();
    let queue = BatchQueue::open(&tmp.path().join("cancel.sqlite"), &input, &output, Mode::Archival).unwrap();
    queue.enqueue("raw.avi", &file, 4, 100).unwrap();
    let control = Control::default();
    control.cancel();
    run_batch(&queue, &PathBuf::from("nonexistent-ffmpeg"), &PathBuf::from("nonexistent-ffprobe"),
        &output, Mode::Archival, ResourceProfile::Low, &control).unwrap();
    assert_eq!(queue.list().unwrap()[0].status, "queued");
}
