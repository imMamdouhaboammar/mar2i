use mar2i_core::policy::Mode;
use mar2i_core::queue::BatchQueue;
use mar2i_core::worker::{Control, ResourceProfile, modified_ns, run_batch};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
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
        "testsrc2=size=320x240:rate=30",
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
fn cancel_during_active_encoding_cleans_temp_and_reaps_process() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("source");
    let output = tmp.path().join("dest");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();

    let raw = input.join("active_cancel_test.avi");
    create_synthetic_video(&raw, 4.0);
    let original_hash = file_sha256(&raw);

    let db = tmp.path().join("batch.sqlite");
    let queue = BatchQueue::open(&db, &input, &output, Mode::Archival).unwrap();
    queue
        .enqueue(
            "active_cancel_test.avi",
            &raw,
            fs::metadata(&raw).unwrap().len(),
            modified_ns(&raw).unwrap(),
        )
        .unwrap();

    let control = Arc::new(Control::default());
    let control_clone = Arc::clone(&control);
    let queue_clone = queue.clone();
    let output_clone = output.clone();

    let worker_thread = thread::spawn(move || {
        run_batch(
            &queue_clone,
            &binary("ffmpeg"),
            &binary("ffprobe"),
            &output_clone,
            Mode::Archival,
            ResourceProfile::Low,
            &control_clone,
        )
    });

    // Wait until the job is claimed or child process is registered
    let mut started = false;
    for _ in 0..100 {
        thread::sleep(Duration::from_millis(50));
        let jobs = queue.list().unwrap();
        if !jobs.is_empty() && jobs[0].status == "processing" {
            started = true;
            break;
        }
    }
    assert!(started, "worker should have started processing the job");

    // Cancel while active encoding is taking place
    control.cancel();

    // The worker thread must exit within bounded time (e.g. 5 seconds max)
    let thread_start = std::time::Instant::now();
    worker_thread
        .join()
        .expect("worker thread joined successfully")
        .expect("run_batch returned ok");
    let elapsed = thread_start.elapsed();
    assert!(
        elapsed < Duration::from_secs(5),
        "cancellation took too long: {elapsed:?}"
    );

    // 1. Original media must remain untouched
    assert_eq!(
        file_sha256(&raw),
        original_hash,
        "original source file was modified"
    );

    // 2. No completed output published
    let published = output.join("active_cancel_test.avi.mar2i.mkv");
    assert!(
        !published.exists(),
        "cancelled job published incomplete output"
    );

    // 3. No leftover temporary partial files in output directory
    for entry in fs::read_dir(&output).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_string();
        assert!(
            !name.starts_with(".mar2i-partial-"),
            "orphan temp file left behind: {name}"
        );
    }

    // 4. Job status in SQLite must be 'interrupted'
    let jobs = queue.list().unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(
        jobs[0].status, "interrupted",
        "job status should be interrupted after cancellation"
    );
    assert!(
        jobs[0]
            .reason
            .as_deref()
            .unwrap_or("")
            .contains("cancelled"),
        "reason should mention cancellation: {:?}",
        jobs[0].reason
    );

    // 5. Recovery can reclaim this interrupted job and it can finish on restart
    let _recovered = queue.recover().unwrap();
    // It's already 'interrupted', so recover count is 0 if it was already updated, or 1 if still processing.
    // In both cases, claim_next must be able to claim it!
    let claimed = queue.claim_next().unwrap();
    assert!(
        claimed.is_some(),
        "interrupted job must be claimable for retry"
    );
    assert_eq!(claimed.unwrap().relative, "active_cancel_test.avi");
}

#[test]
fn cancel_while_paused_wakes_and_exits_cleanly() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("source");
    let output = tmp.path().join("dest");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();

    let raw = input.join("paused_test.avi");
    create_synthetic_video(&raw, 0.5);

    let db = tmp.path().join("batch.sqlite");
    let queue = BatchQueue::open(&db, &input, &output, Mode::Archival).unwrap();
    queue
        .enqueue(
            "paused_test.avi",
            &raw,
            fs::metadata(&raw).unwrap().len(),
            modified_ns(&raw).unwrap(),
        )
        .unwrap();

    let control = Arc::new(Control::default());
    control.pause();

    let control_clone = Arc::clone(&control);
    let queue_clone = queue.clone();
    let output_clone = output.clone();

    let worker_thread = thread::spawn(move || {
        run_batch(
            &queue_clone,
            &binary("ffmpeg"),
            &binary("ffprobe"),
            &output_clone,
            Mode::Archival,
            ResourceProfile::Low,
            &control_clone,
        )
    });

    thread::sleep(Duration::from_millis(100));
    assert!(control.is_paused());

    // Cancel while paused
    control.cancel();

    // Must wake up immediately and exit
    worker_thread
        .join()
        .expect("thread joined")
        .expect("run_batch succeeded");

    let jobs = queue.list().unwrap();
    assert_eq!(jobs[0].status, "queued");
}

#[test]
fn repeated_cancellation_is_idempotent() {
    let control = Arc::new(Control::default());
    let mut handles = Vec::new();
    for _ in 0..10 {
        let c = Arc::clone(&control);
        handles.push(thread::spawn(move || {
            c.cancel();
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    assert!(control.is_cancelled());
}

#[test]
fn cancel_with_multiple_workers_active_cleans_and_records_correctly() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("source");
    let output = tmp.path().join("dest");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();

    let files = ["multi_1.avi", "multi_2.avi", "multi_3.avi", "multi_4.avi"];
    let mut original_hashes = Vec::new();
    for name in &files {
        let path = input.join(name);
        create_synthetic_video(&path, 4.0);
        original_hashes.push(file_sha256(&path));
    }

    let db = tmp.path().join("batch.sqlite");
    let queue = BatchQueue::open(&db, &input, &output, Mode::Archival).unwrap();
    for name in &files {
        let path = input.join(name);
        queue
            .enqueue(
                name,
                &path,
                fs::metadata(&path).unwrap().len(),
                modified_ns(&path).unwrap(),
            )
            .unwrap();
    }

    let control = Arc::new(Control::default());
    let control_clone = Arc::clone(&control);
    let queue_clone = queue.clone();
    let output_clone = output.clone();

    let worker_thread = thread::spawn(move || {
        run_batch(
            &queue_clone,
            &binary("ffmpeg"),
            &binary("ffprobe"),
            &output_clone,
            Mode::Archival,
            ResourceProfile::Maximum,
            &control_clone,
        )
    });

    // Wait until at least one worker is active
    let mut any_processing = false;
    for _ in 0..100 {
        thread::sleep(Duration::from_millis(50));
        let jobs = queue.list().unwrap();
        if jobs.iter().any(|j| j.status == "processing") {
            any_processing = true;
            break;
        }
    }
    assert!(any_processing, "at least one worker should be processing");

    control.cancel();

    worker_thread
        .join()
        .expect("worker thread joined")
        .expect("run_batch returned ok");

    // All original files must remain untouched
    for (name, hash) in files.iter().zip(original_hashes.iter()) {
        let path = input.join(name);
        assert_eq!(&file_sha256(&path), hash, "source modified: {name}");
    }

    // No leftover temporary partial files
    for entry in fs::read_dir(&output).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_string();
        assert!(
            !name.starts_with(".mar2i-partial-"),
            "orphan temp file left behind: {name}"
        );
    }

    // In SQLite, every job must either be interrupted, queued, or completed
    let jobs = queue.list().unwrap();
    for job in &jobs {
        assert!(
            matches!(job.status.as_str(), "interrupted" | "queued" | "completed"),
            "unexpected job status: {}",
            job.status
        );
    }
    assert!(
        jobs.iter().any(|j| j.status == "interrupted"),
        "at least one job should be marked interrupted"
    );
}

#[test]
fn restart_and_recovery_resumes_interrupted_work_to_completion() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("source");
    let output = tmp.path().join("dest");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();

    let raw = input.join("resumed_test.avi");
    create_synthetic_video(&raw, 2.0);
    let original_hash = file_sha256(&raw);

    let db = tmp.path().join("batch.sqlite");
    let queue = BatchQueue::open(&db, &input, &output, Mode::Archival).unwrap();
    queue
        .enqueue(
            "resumed_test.avi",
            &raw,
            fs::metadata(&raw).unwrap().len(),
            modified_ns(&raw).unwrap(),
        )
        .unwrap();

    let control = Arc::new(Control::default());
    let control_clone = Arc::clone(&control);
    let queue_clone = queue.clone();
    let output_clone = output.clone();

    let worker_thread = thread::spawn(move || {
        run_batch(
            &queue_clone,
            &binary("ffmpeg"),
            &binary("ffprobe"),
            &output_clone,
            Mode::Archival,
            ResourceProfile::Low,
            &control_clone,
        )
    });

    for _ in 0..100 {
        thread::sleep(Duration::from_millis(50));
        let jobs = queue.list().unwrap();
        if !jobs.is_empty() && jobs[0].status == "processing" {
            break;
        }
    }
    control.cancel();
    worker_thread.join().unwrap().unwrap();

    let jobs_after_cancel = queue.list().unwrap();
    assert_eq!(jobs_after_cancel[0].status, "interrupted");

    // Now simulate application restart / recovery
    let fresh_control = Control::default();
    queue.recover().unwrap();

    // Run batch to completion with fresh control
    run_batch(
        &queue,
        &binary("ffmpeg"),
        &binary("ffprobe"),
        &output,
        Mode::Archival,
        ResourceProfile::Low,
        &fresh_control,
    )
    .unwrap();

    let final_jobs = queue.list().unwrap();
    assert_eq!(final_jobs.len(), 1);
    assert_eq!(final_jobs[0].status, "completed");
    assert!(output.join("resumed_test.avi.mar2i.mkv").is_file());
    assert_eq!(file_sha256(&raw), original_hash);
}

#[cfg(unix)]
#[test]
fn child_process_termination_timeout_forces_sigkill() {
    use mar2i_core::runner::terminate_child_bounded;
    // Spawn a child process that explicitly ignores SIGTERM and signals readiness on stdout
    let mut child = Command::new("python3")
        .args([
            "-c",
            "import signal, time; signal.signal(signal.SIGTERM, signal.SIG_IGN); print('ready', flush=True); time.sleep(30)",
        ])
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn python3 ignoring SIGTERM");

    let stdout = child.stdout.take().expect("take child stdout");
    let mut reader = BufReader::new(stdout);
    let mut ready_line = String::new();
    reader
        .read_line(&mut ready_line)
        .expect("read readiness line from python3");
    assert_eq!(ready_line.trim(), "ready");

    let start = std::time::Instant::now();
    terminate_child_bounded(&mut child).expect("bounded termination succeeds");
    let elapsed = start.elapsed();

    // Must have waited for the grace period (1000ms) then sent SIGKILL, taking ~1.0-1.5s
    assert!(
        elapsed >= Duration::from_millis(950),
        "should have respected grace period: {elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_millis(2500),
        "should not exceed bounded timeout: {elapsed:?}"
    );

    // Process must be dead
    let status = child.try_wait().expect("try_wait");
    assert!(status.is_some(), "child process should be terminated");
}
