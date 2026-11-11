use mar2i_core::policy::Mode;
use mar2i_core::queue::BatchQueue;
use mar2i_core::report::{report_csv, report_html, summarize};
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
fn arabic_and_unicode_paths_processed_and_preserved() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("source");
    let output = tmp.path().join("dest");

    let nested_arabic = input.join("وزارة_الحج/موسم_١٤٤٥/توثيق_المشاعر");
    fs::create_dir_all(&nested_arabic).unwrap();
    fs::create_dir_all(&output).unwrap();

    let clip1 = nested_arabic.join("تسجيل_عرفات_رقم_١.avi");
    let clip2 = nested_arabic.join("Raw Video Clip (Hajj & Umrah).avi");

    create_synthetic_video(&clip1, 0.5);
    create_synthetic_video(&clip2, 0.5);

    let hash1_before = file_sha256(&clip1);
    let hash2_before = file_sha256(&clip2);

    let db = tmp.path().join("batch.sqlite");
    let queue = BatchQueue::open(&db, &input, &output, Mode::Archival).unwrap();

    let rel1 = "وزارة_الحج/موسم_١٤٤٥/توثيق_المشاعر/تسجيل_عرفات_رقم_١.avi";
    let rel2 = "وزارة_الحج/موسم_١٤٤٥/توثيق_المشاعر/Raw Video Clip (Hajj & Umrah).avi";

    for (rel, path) in [(rel1, &clip1), (rel2, &clip2)] {
        queue
            .enqueue(
                rel,
                path,
                fs::metadata(path).unwrap().len(),
                modified_ns(path).unwrap(),
            )
            .unwrap();
    }

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
    assert_eq!(jobs.len(), 2);
    let summary = summarize(&jobs);
    assert_eq!(summary.completed, 2, "all arabic jobs should complete");

    // Output files exist at matched nested paths
    let expected_out1 =
        output.join("وزارة_الحج/موسم_١٤٤٥/توثيق_المشاعر/تسجيل_عرفات_رقم_١.avi.mar2i.mkv");
    let expected_out2 = output
        .join("وزارة_الحج/موسم_١٤٤٥/توثيق_المشاعر/Raw Video Clip (Hajj & Umrah).avi.mar2i.mkv");
    assert!(expected_out1.is_file(), "arabic output 1 should exist");
    assert!(expected_out2.is_file(), "unicode output 2 should exist");

    // Source files completely untouched
    assert_eq!(file_sha256(&clip1), hash1_before);
    assert_eq!(file_sha256(&clip2), hash2_before);

    // Reports accurately reflect arabic names
    let csv = report_csv(&jobs);
    assert!(csv.contains("تسجيل_عرفات_رقم_١.avi"));
    let html = report_html(&jobs);
    assert!(html.contains("تسجيل_عرفات_رقم_١.avi"));
}

#[test]
fn duplicate_filenames_in_different_subdirectories_do_not_collide() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("source");
    let output = tmp.path().join("dest");

    let sub1 = input.join("cam_a");
    let sub2 = input.join("cam_b");
    fs::create_dir_all(&sub1).unwrap();
    fs::create_dir_all(&sub2).unwrap();
    fs::create_dir_all(&output).unwrap();

    let file1 = sub1.join("record.avi");
    let file2 = sub2.join("record.avi");
    create_synthetic_video(&file1, 0.5);
    create_synthetic_video(&file2, 0.5);

    let db = tmp.path().join("batch.sqlite");
    let queue = BatchQueue::open(&db, &input, &output, Mode::Archival).unwrap();
    queue
        .enqueue(
            "cam_a/record.avi",
            &file1,
            fs::metadata(&file1).unwrap().len(),
            modified_ns(&file1).unwrap(),
        )
        .unwrap();
    queue
        .enqueue(
            "cam_b/record.avi",
            &file2,
            fs::metadata(&file2).unwrap().len(),
            modified_ns(&file2).unwrap(),
        )
        .unwrap();

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

    let out1 = output.join("cam_a/record.avi.mar2i.mkv");
    let out2 = output.join("cam_b/record.avi.mar2i.mkv");
    assert!(out1.is_file());
    assert!(out2.is_file());

    let jobs = queue.list().unwrap();
    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].status, "completed");
    assert_eq!(jobs[1].status, "completed");
}

#[test]
fn existing_output_collision_never_overwrites_existing_file() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("source");
    let output = tmp.path().join("dest");
    fs::create_dir_all(&input).unwrap();
    fs::create_dir_all(&output).unwrap();

    let raw = input.join("clip.avi");
    create_synthetic_video(&raw, 0.5);

    let destination = output.join("clip.avi.mar2i.mkv");
    let sentinel = b"PRESERVED_USER_OUTPUT_DO_NOT_TOUCH";
    fs::write(&destination, sentinel).unwrap();

    let db = tmp.path().join("batch.sqlite");
    let queue = BatchQueue::open(&db, &input, &output, Mode::Archival).unwrap();
    queue
        .enqueue(
            "clip.avi",
            &raw,
            fs::metadata(&raw).unwrap().len(),
            modified_ns(&raw).unwrap(),
        )
        .unwrap();

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

    // The existing file must remain untouched
    assert_eq!(fs::read(&destination).unwrap(), sentinel);

    let jobs = queue.list().unwrap();
    assert_eq!(jobs[0].status, "failed");
    assert!(
        jobs[0]
            .reason
            .as_deref()
            .unwrap_or("")
            .contains("already exists")
    );
}
