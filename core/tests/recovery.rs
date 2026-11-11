use mar2i_core::policy::Mode;
use mar2i_core::queue::{BatchQueue, JobCompletion};
use mar2i_core::runner::PartialOutputRegistry;
use mar2i_core::worker::{Control, ResourceProfile, modified_ns, run_batch};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

fn binary(name: &str) -> PathBuf {
    std::env::var(format!("MAR2I_TEST_{}", name.to_uppercase()))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(name))
}

fn file_sha256(path: &Path) -> Vec<u8> {
    let data = fs::read(path).expect("read file for hashing");
    let mut hasher = Sha256::new();
    hasher.update(&data);
    hasher.finalize().to_vec()
}

fn create_synthetic_video(path: &Path, duration_secs: f32) {
    let mut command = Command::new(binary("ffmpeg"));
    command.args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-f",
        "lavfi",
        "-i",
        "testsrc2=size=320x240:rate=25",
        "-t",
        &duration_secs.to_string(),
        "-c:v",
        "rawvideo",
        "-pix_fmt",
        "yuv420p",
        "-y",
    ]);
    assert!(command.arg(path).status().unwrap().success());
}

#[test]
fn recovery_preserves_unregistered_files_even_when_they_match_temp_prefix() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("source");
    let output = tmp.path().join("dest");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();

    let unrelated = output.join(".mar2i-partial-personal-recording.mkv");
    let original = b"user-owned file, not an application temporary output";
    fs::write(&unrelated, original).unwrap();

    let queue = BatchQueue::open(
        &tmp.path().join("batch.sqlite"),
        &input,
        &output,
        Mode::Archival,
    )
    .unwrap();
    assert_eq!(queue.recover().unwrap(), 0);
    assert_eq!(fs::read(&unrelated).unwrap(), original);
}

#[test]
fn registration_rejects_temporary_paths_outside_the_batch_destination() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("source");
    let output = tmp.path().join("dest");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();

    let unrelated = tmp.path().join(".mar2i-partial-private.mkv");
    fs::write(&unrelated, b"preserve").unwrap();
    let queue = BatchQueue::open(
        &tmp.path().join("batch.sqlite"),
        &input,
        &output,
        Mode::Archival,
    )
    .unwrap();
    assert!(queue.register_partial(&unrelated).is_err());
    queue.recover().unwrap();
    assert_eq!(fs::read(&unrelated).unwrap(), b"preserve");
}

#[cfg(unix)]
#[test]
fn recovery_never_follows_a_replaced_registered_symlink() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("source");
    let output = tmp.path().join("dest");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();

    let protected = tmp.path().join("protected.mp4");
    fs::write(&protected, b"source must survive").unwrap();
    let partial = output.join(".mar2i-partial-owned.mkv");
    fs::write(&partial, b"temporary").unwrap();

    let queue = BatchQueue::open(
        &tmp.path().join("batch.sqlite"),
        &input,
        &output,
        Mode::Archival,
    )
    .unwrap();
    queue.register_partial(&partial).unwrap();
    fs::remove_file(&partial).unwrap();
    std::os::unix::fs::symlink(&protected, &partial).unwrap();

    assert!(queue.recover().is_err());
    assert_eq!(fs::read(&protected).unwrap(), b"source must survive");
    assert!(
        fs::symlink_metadata(&partial)
            .unwrap()
            .file_type()
            .is_symlink()
    );
}

#[test]
fn recover_cleans_orphaned_partial_temp_files_from_crashed_run() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("source");
    let output = tmp.path().join("dest");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(output.join("nested")).unwrap();

    let video = input.join("video.avi");
    create_synthetic_video(&video, 1.0);

    let db = tmp.path().join("batch.sqlite");
    let queue = BatchQueue::open(&db, &input, &output, Mode::Archival).unwrap();
    queue
        .enqueue(
            "video.avi",
            &video,
            fs::metadata(&video).unwrap().len(),
            modified_ns(&video).unwrap(),
        )
        .unwrap();

    // Claim the job so it transitions to 'processing'
    let claimed = queue.claim_next().unwrap().unwrap();
    assert_eq!(claimed.status, "processing");

    // Simulate orphaned partial temp files left on disk from a crashed process
    let orphan1 = output.join(".mar2i-partial-12345.mkv");
    let orphan2 = output.join("nested/.mar2i-partial-67890.mp4");
    fs::write(&orphan1, b"incomplete video stream data 1").unwrap();
    fs::write(&orphan2, b"incomplete video stream data 2").unwrap();

    assert!(orphan1.is_file());
    assert!(orphan2.is_file());
    queue.register_partial(&orphan1).unwrap();
    queue.register_partial(&orphan2).unwrap();

    // Run recovery
    let recovered_count = queue.recover().unwrap();
    assert_eq!(recovered_count, 1);

    // 1. Partial files must be deleted
    assert!(
        !orphan1.exists(),
        "orphaned partial file 1 was not cleaned up"
    );
    assert!(
        !orphan2.exists(),
        "orphaned partial file 2 was not cleaned up"
    );

    // 2. Job in SQLite must be marked interrupted
    let jobs = queue.list().unwrap();
    assert_eq!(jobs[0].status, "interrupted");
    assert!(jobs[0].reason.as_deref().unwrap_or("").contains("stopped"));

    // 3. Interrupted job can be reclaimed for retry
    let reclaimed = queue.claim_next().unwrap();
    assert!(reclaimed.is_some());
    assert_eq!(reclaimed.unwrap().id, claimed.id);
}

#[test]
fn interrupted_jobs_restart_from_beginning_and_complete() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("source");
    let output = tmp.path().join("dest");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();

    let raw = input.join("interrupted.avi");
    create_synthetic_video(&raw, 1.0);
    let original_hash = file_sha256(&raw);

    let db = tmp.path().join("batch.sqlite");
    let queue = BatchQueue::open(&db, &input, &output, Mode::Archival).unwrap();
    queue
        .enqueue(
            "interrupted.avi",
            &raw,
            fs::metadata(&raw).unwrap().len(),
            modified_ns(&raw).unwrap(),
        )
        .unwrap();

    // Simulate job was claimed and process terminated before completion
    let claimed = queue.claim_next().unwrap().unwrap();
    assert_eq!(claimed.status, "processing");

    // Recovery marks it interrupted
    queue.recover().unwrap();
    assert_eq!(queue.list().unwrap()[0].status, "interrupted");

    // Restart batch processing
    let control = Control::default();
    run_batch(
        &queue,
        &binary("ffmpeg"),
        &binary("ffprobe"),
        &output,
        Mode::Archival,
        ResourceProfile::Low,
        &control,
    )
    .unwrap();

    let jobs = queue.list().unwrap();
    assert_eq!(jobs[0].status, "completed");
    assert!(output.join("interrupted.avi.mar2i.mkv").is_file());
    assert_eq!(file_sha256(&raw), original_hash);
}

#[test]
fn completed_jobs_are_not_reprocessed_on_restart() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("source");
    let output = tmp.path().join("dest");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();

    let vid1 = input.join("vid1.avi");
    let vid2 = input.join("vid2.avi");
    create_synthetic_video(&vid1, 0.5);
    create_synthetic_video(&vid2, 0.5);

    let db = tmp.path().join("batch.sqlite");
    let queue = BatchQueue::open(&db, &input, &output, Mode::Archival).unwrap();
    queue
        .enqueue(
            "vid1.avi",
            &vid1,
            fs::metadata(&vid1).unwrap().len(),
            modified_ns(&vid1).unwrap(),
        )
        .unwrap();
    queue
        .enqueue(
            "vid2.avi",
            &vid2,
            fs::metadata(&vid2).unwrap().len(),
            modified_ns(&vid2).unwrap(),
        )
        .unwrap();

    // Complete vid1 manually
    let claimed1 = queue.claim_next().unwrap().unwrap();
    let out1 = output.join("vid1.avi.mar2i.mkv");
    fs::write(&out1, b"verified completed output").unwrap();
    queue
        .finish(
            claimed1.id,
            JobCompletion {
                status: "completed",
                output: Some(&out1),
                bytes: Some(25),
                quality: Some("lossless"),
                reason: None,
                elapsed_ms: 150,
            },
        )
        .unwrap();

    // Now restart the batch
    queue.recover().unwrap();
    let control = Control::default();
    run_batch(
        &queue,
        &binary("ffmpeg"),
        &binary("ffprobe"),
        &output,
        Mode::Archival,
        ResourceProfile::Low,
        &control,
    )
    .unwrap();

    // Vid1 content must NOT have been overwritten
    assert_eq!(fs::read(&out1).unwrap(), b"verified completed output");
    let jobs = queue.list().unwrap();
    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].status, "completed");
    assert_eq!(jobs[0].output_bytes, Some(25));
    assert_eq!(jobs[1].status, "completed");
}

#[test]
fn failed_jobs_remain_failed_and_are_not_silently_retried() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("source");
    let output = tmp.path().join("dest");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();

    let bad = input.join("corrupted.mp4");
    fs::write(&bad, b"corrupted garbage").unwrap();

    let db = tmp.path().join("batch.sqlite");
    let queue = BatchQueue::open(&db, &input, &output, Mode::Publishing).unwrap();
    queue
        .enqueue(
            "corrupted.mp4",
            &bad,
            fs::metadata(&bad).unwrap().len(),
            modified_ns(&bad).unwrap(),
        )
        .unwrap();

    let control = Control::default();
    run_batch(
        &queue,
        &binary("ffmpeg"),
        &binary("ffprobe"),
        &output,
        Mode::Publishing,
        ResourceProfile::Low,
        &control,
    )
    .unwrap();

    let jobs = queue.list().unwrap();
    assert_eq!(jobs[0].status, "failed");

    // Subsequent recovery and claim
    queue.recover().unwrap();
    assert!(
        queue.claim_next().unwrap().is_none(),
        "failed job should not be reclaimed"
    );
}

#[test]
fn removed_source_file_fails_cleanly_on_recovery_retry() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("source");
    let output = tmp.path().join("dest");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();

    let raw = input.join("disappearing.avi");
    create_synthetic_video(&raw, 0.5);

    let db = tmp.path().join("batch.sqlite");
    let queue = BatchQueue::open(&db, &input, &output, Mode::Archival).unwrap();
    queue
        .enqueue(
            "disappearing.avi",
            &raw,
            fs::metadata(&raw).unwrap().len(),
            modified_ns(&raw).unwrap(),
        )
        .unwrap();

    // Job is claimed, then interrupted
    let _claimed = queue.claim_next().unwrap().unwrap();
    queue.recover().unwrap();

    // Now delete the source file before retry
    fs::remove_file(&raw).unwrap();

    let control = Control::default();
    run_batch(
        &queue,
        &binary("ffmpeg"),
        &binary("ffprobe"),
        &output,
        Mode::Archival,
        ResourceProfile::Low,
        &control,
    )
    .unwrap();

    let jobs = queue.list().unwrap();
    assert_eq!(jobs[0].status, "failed");
    assert!(
        jobs[0]
            .reason
            .as_deref()
            .unwrap_or("")
            .contains("No such file")
    );
}

#[test]
fn modified_source_file_is_rejected_on_recovery_retry() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("source");
    let output = tmp.path().join("dest");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();

    let raw = input.join("tampered.avi");
    create_synthetic_video(&raw, 0.5);

    let db = tmp.path().join("batch.sqlite");
    let queue = BatchQueue::open(&db, &input, &output, Mode::Archival).unwrap();
    queue
        .enqueue(
            "tampered.avi",
            &raw,
            fs::metadata(&raw).unwrap().len(),
            modified_ns(&raw).unwrap(),
        )
        .unwrap();

    let _claimed = queue.claim_next().unwrap().unwrap();
    queue.recover().unwrap();

    // Modify the source file before retry
    fs::write(&raw, b"completely different content now").unwrap();

    let control = Control::default();
    run_batch(
        &queue,
        &binary("ffmpeg"),
        &binary("ffprobe"),
        &output,
        Mode::Archival,
        ResourceProfile::Low,
        &control,
    )
    .unwrap();

    let jobs = queue.list().unwrap();
    assert_eq!(jobs[0].status, "failed");
    assert!(
        jobs[0]
            .reason
            .as_deref()
            .unwrap_or("")
            .contains("changed since discovery")
    );
}

#[test]
fn repeated_recover_calls_are_idempotent() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("source");
    let output = tmp.path().join("dest");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();

    let raw = input.join("item.avi");
    create_synthetic_video(&raw, 0.5);

    let db = tmp.path().join("batch.sqlite");
    let queue = BatchQueue::open(&db, &input, &output, Mode::Archival).unwrap();
    queue
        .enqueue(
            "item.avi",
            &raw,
            fs::metadata(&raw).unwrap().len(),
            modified_ns(&raw).unwrap(),
        )
        .unwrap();

    let _claimed = queue.claim_next().unwrap().unwrap();

    let first_recover = queue.recover().unwrap();
    assert_eq!(first_recover, 1);

    let second_recover = queue.recover().unwrap();
    assert_eq!(second_recover, 0);

    let third_recover = queue.recover().unwrap();
    assert_eq!(third_recover, 0);

    let jobs = queue.list().unwrap();
    assert_eq!(jobs[0].status, "interrupted");
}
