use mar2i_core::discovery::discover;
use std::fs;
use std::path::Path;
use std::time::Instant;
use tempfile::tempdir;

fn populate_dummy_files(dir: &Path, count: usize) {
    for i in 0..count {
        let sub = dir.join(format!("sub_{:02}", i / 100));
        let _ = fs::create_dir_all(&sub);
        let file = sub.join(format!("video_{:05}.mp4", i));
        // Lightweight simulated video payload (128 bytes with mp4 header-like content)
        fs::write(
            &file,
            b"ftypisom\0\0\0\x20isomiso2avc1mp41\0\0\0\0moov\0\0\0lmvhd",
        )
        .unwrap();
    }
}

#[test]
fn bench_discovery_scale() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().join("corpus");
    fs::create_dir_all(&root).unwrap();

    // 100 files
    populate_dummy_files(&root, 100);
    let start_100 = Instant::now();
    let files_100 = discover(&root).unwrap();
    let dur_100 = start_100.elapsed();
    assert_eq!(files_100.len(), 100);

    // 1,000 files
    let tmp_1000 = tempdir().unwrap();
    let root_1000 = tmp_1000.path().join("corpus_1000");
    fs::create_dir_all(&root_1000).unwrap();
    populate_dummy_files(&root_1000, 1000);
    let start_1000 = Instant::now();
    let files_1000 = discover(&root_1000).unwrap();
    let dur_1000 = start_1000.elapsed();
    assert_eq!(files_1000.len(), 1000);

    // 10,000 files
    let tmp_10000 = tempdir().unwrap();
    let root_10000 = tmp_10000.path().join("corpus_10000");
    fs::create_dir_all(&root_10000).unwrap();
    populate_dummy_files(&root_10000, 10000);
    let start_10000 = Instant::now();
    let files_10000 = discover(&root_10000).unwrap();
    let dur_10000 = start_10000.elapsed();
    assert_eq!(files_10000.len(), 10000);

    eprintln!("\n=== Discovery Benchmarks ===");
    eprintln!("100 files discovery:     {:?}", dur_100);
    eprintln!("1,000 files discovery:   {:?}", dur_1000);
    eprintln!("10,000 files discovery:  {:?}", dur_10000);

    // Upper bound sanity checks
    assert!(
        dur_100.as_millis() < 500,
        "100 files should discover in <500ms"
    );
    assert!(
        dur_1000.as_millis() < 2000,
        "1,000 files should discover in <2000ms"
    );
    assert!(
        dur_10000.as_secs() < 10,
        "10,000 files should discover in <10s"
    );
}
