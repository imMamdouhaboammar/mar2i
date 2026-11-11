use mar2i_core::fingerprint::{
    FingerprintRecord, FingerprintRegistry, compute_fingerprint,
};
use std::fs::File;
use std::io::Write;
use tempfile::Builder;

#[test]
fn sparse_fingerprint_deterministic_on_synthetic_media() -> anyhow::Result<()> {
    let dir = Builder::new().prefix("mar2i-test-fp-").tempdir()?;
    let file1 = dir.path().join("clip1.mp4");
    let file2 = dir.path().join("clip2.mp4");

    // Write 2.5 MiB of data
    let mut f1 = File::create(&file1)?;
    let mut f2 = File::create(&file2)?;

    let chunk = vec![0x42u8; 1024 * 1024]; // 1 MiB
    f1.write_all(&chunk)?;
    f1.write_all(&[0x00; 512 * 1024])?; // 512 KiB middle
    f1.write_all(&chunk)?; // 1 MiB tail

    f2.write_all(&chunk)?;
    f2.write_all(&[0x00; 512 * 1024])?;
    f2.write_all(&chunk)?;

    drop(f1);
    drop(f2);

    let fp1 = compute_fingerprint(&file1)?;
    let fp2 = compute_fingerprint(&file2)?;

    assert_eq!(fp1.hash, fp2.hash, "identical media must produce identical sparse fingerprint");
    assert_eq!(fp1.size_bytes, 2560 * 1024);

    // Modify a tail byte to verify sensitivity
    let mut f2_mut = std::fs::OpenOptions::new().write(true).open(&file2)?;
    f2_mut.write_all(&[0xFF])?;
    drop(f2_mut);

    let fp3 = compute_fingerprint(&file2)?;
    assert_ne!(fp1.hash, fp3.hash, "fingerprint must change when content changes");

    Ok(())
}

#[test]
fn fingerprint_registry_sqlite_persistence_and_lookup() -> anyhow::Result<()> {
    let registry = FingerprintRegistry::open_in_memory()?;

    let record = FingerprintRecord {
        fingerprint: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into(),
        source_path: "/data/hajj/mina_raw_001.mov".into(),
        output_path: "/archive/2026/mina_001.ffv1.mkv".into(),
        mode: "archive".into(),
        codec: "ffv1".into(),
        input_bytes: 4_294_967_296,
        output_bytes: 2_147_483_648,
        created_at: "2026-11-11 12:00:00".into(),
    };

    assert!(registry.lookup(&record.fingerprint)?.is_none());

    registry.register(&record)?;

    let found = registry.lookup(&record.fingerprint)?.expect("should find registered fingerprint");
    assert_eq!(found.source_path, record.source_path);
    assert_eq!(found.input_bytes, record.input_bytes);
    assert_eq!(found.output_bytes, record.output_bytes);
    assert_eq!(found.codec, "ffv1");

    Ok(())
}
