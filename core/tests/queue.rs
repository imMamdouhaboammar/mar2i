use mar2i_core::policy::Mode;
use mar2i_core::queue::{BatchQueue, JobCompletion};
use mar2i_core::worker::{Control, ResourceProfile, run_batch};
use std::fs::{self, File};
use std::path::Path;
use tempfile::tempdir;

#[test]
fn another_owner_prevents_recovery_from_interrupting_an_active_job() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("input");
    let output = tmp.path().join("output");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();
    let db = tmp.path().join("batch.sqlite");
    let queue = BatchQueue::open(&db, &input, &output, Mode::Archival).unwrap();
    let source = input.join("source.mp4");
    fs::write(&source, b"media").unwrap();
    queue.enqueue("source.mp4", &source, 5, 123).unwrap();
    queue.claim_next().unwrap().unwrap();

    // Separate OS file handle simulates a second process holding the processing lease.
    let lock = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(db.with_extension("sqlite.lock"))
        .unwrap();
    lock.try_lock().unwrap();
    assert!(queue.recover().is_err());
    assert!(
        run_batch(
            &queue,
            Path::new("ffmpeg"),
            Path::new("ffprobe"),
            &output,
            Mode::Archival,
            ResourceProfile::Low,
            &Control::default(),
        )
        .is_err(),
        "a second process must not begin work while the lease is owned"
    );
    assert_eq!(queue.list().unwrap()[0].status, "processing");
    let status_reader = BatchQueue::open(&db, &input, &output, Mode::Archival).unwrap();
    assert_eq!(
        status_reader.list().unwrap()[0].status,
        "processing",
        "status polling must remain available while processing is locked"
    );

    drop(lock);
    assert_eq!(queue.recover().unwrap(), 1);
}

#[test]
fn repeated_discovery_is_idempotent_and_claims_each_job_at_most_once() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("input");
    let output = tmp.path().join("output");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();
    let db = tmp.path().join("batch.sqlite");
    let queue = BatchQueue::open(&db, &input, &output, Mode::Publishing).unwrap();
    let source = input.join("فيديو.mp4");
    fs::write(&source, b"placeholder").unwrap();
    queue.enqueue("فيديو.mp4", &source, 11, 123).unwrap();
    queue.enqueue("فيديو.mp4", &source, 11, 123).unwrap();
    assert_eq!(queue.list().unwrap().len(), 1);
    assert!(queue.claim_next().unwrap().is_some());
    assert!(queue.claim_next().unwrap().is_none());
}

#[test]
fn interrupted_processing_can_be_reclaimed_after_recovery() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("input");
    let output = tmp.path().join("output");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();
    let queue = BatchQueue::open(
        &tmp.path().join("recover.sqlite"),
        &input,
        &output,
        Mode::Archival,
    )
    .unwrap();
    let source = input.join("raw.avi");
    fs::write(&source, b"raw").unwrap();
    queue.enqueue("raw.avi", &source, 3, 123).unwrap();
    let claimed = queue.claim_next().unwrap().unwrap();
    assert_eq!(claimed.status, "processing");
    assert_eq!(queue.recover().unwrap(), 1);
    let again = queue.claim_next().unwrap().unwrap();
    assert_eq!(again.id, claimed.id);
}

#[test]
fn completed_job_is_never_reclaimed_after_restart_or_rescan() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("input");
    let output = tmp.path().join("output");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();
    let db = tmp.path().join("completed.sqlite");
    let source = input.join("done.mp4");
    fs::write(&source, b"original").unwrap();
    let queue = BatchQueue::open(&db, &input, &output, Mode::Publishing).unwrap();
    queue.enqueue("done.mp4", &source, 8, 555).unwrap();
    let claimed = queue.claim_next().unwrap().unwrap();
    let out = output.join("done.mp4.mar2i.mp4");
    fs::write(&out, b"smaller").unwrap();
    queue
        .finish(
            claimed.id,
            JobCompletion {
                status: "completed",
                output: Some(&out),
                bytes: Some(7),
                quality: Some("SSIM=.99"),
                reason: None,
                elapsed_ms: 100,
            },
        )
        .unwrap();
    let restarted = BatchQueue::open(&db, &input, &output, Mode::Publishing).unwrap();
    restarted.recover().unwrap();
    restarted.enqueue("done.mp4", &source, 8, 555).unwrap();
    assert!(restarted.claim_next().unwrap().is_none());
    assert_eq!(restarted.list().unwrap()[0].output_bytes, Some(7));
}

#[test]
fn mismatched_batch_roots_or_mode_cannot_reuse_state() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("input");
    let output = tmp.path().join("output");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();
    let db = tmp.path().join("state.sqlite");
    BatchQueue::open(&db, &input, &output, Mode::Publishing).unwrap();
    assert!(BatchQueue::open(&db, &input, &output, Mode::Archival).is_err());
}

#[test]
fn invalid_completion_cannot_change_a_claimed_job() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("input");
    let output = tmp.path().join("output");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();
    let source = input.join("source.mp4");
    fs::write(&source, b"video").unwrap();
    let queue = BatchQueue::open(
        &tmp.path().join("batch.sqlite"),
        &input,
        &output,
        Mode::Publishing,
    )
    .unwrap();
    queue.enqueue("source.mp4", &source, 5, 123).unwrap();
    let job = queue.claim_next().unwrap().unwrap();

    for (status, bytes, elapsed_ms) in [
        ("queued", None, 100),
        ("completed", Some(u64::MAX), 100),
        ("completed", Some(5), u64::MAX),
    ] {
        assert!(
            queue
                .finish(
                    job.id,
                    JobCompletion {
                        status,
                        output: None,
                        bytes,
                        quality: None,
                        reason: None,
                        elapsed_ms,
                    }
                )
                .is_err()
        );
        assert_eq!(queue.list().unwrap()[0].status, "processing");
    }
}
