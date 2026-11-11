use mar2i_core::policy::Mode;
use mar2i_core::runner::{RunOutcome, run_one};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

fn binary(name: &str) -> PathBuf {
    std::env::var(format!("MAR2I_TEST_{}", name.to_uppercase()))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(name))
}

fn sha256(path: &Path) -> Vec<u8> {
    let mut file = File::open(path).unwrap();
    let mut hasher = Sha256::new();
    let mut chunk = [0u8; 64 * 1024];
    loop {
        let len = file.read(&mut chunk).unwrap();
        if len == 0 {
            break;
        }
        hasher.update(&chunk[..len]);
    }
    hasher.finalize().to_vec()
}

#[test]
#[ignore = "requires an isolated, mounted 80 MiB no-hardlink filesystem"]
fn no_hardlink_filesystem_never_exposes_incomplete_final_output() {
    let volume = PathBuf::from(
        std::env::var_os("MAR2I_TEST_NO_HARDLINK_DIR")
            .expect("set MAR2I_TEST_NO_HARDLINK_DIR to the isolated test volume"),
    );
    assert!(volume.is_dir());
    let output = tempfile::tempdir_in(volume).unwrap();
    let probe = output.path().join("probe");
    fs::write(&probe, b"probe").unwrap();
    assert!(
        fs::hard_link(&probe, output.path().join("probe-link")).is_err(),
        "test requires a filesystem that does not support hard links"
    );

    let input_dir = tempdir().unwrap();
    let source = input_dir.path().join("original.mp4");
    let status = Command::new(binary("ffmpeg"))
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=128x128:rate=24",
            "-t",
            "1",
            "-c:v",
            "libx264",
            "-movflags",
            "+faststart",
            "-y",
        ])
        .arg(&source)
        .status()
        .unwrap();
    assert!(status.success());
    OpenOptions::new()
        .write(true)
        .open(&source)
        .unwrap()
        .set_len(48 * 1024 * 1024)
        .unwrap();
    let before = sha256(&source);
    let destination = output.path().join("original.mp4");

    let result = run_one(
        &binary("ffmpeg"),
        &binary("ffprobe"),
        &source,
        Path::new("original.mp4"),
        output.path(),
        Mode::Archival,
    );

    match result {
        Ok(RunOutcome::Copied {
            output: published, ..
        }) => {
            assert_eq!(published, destination);
            assert_eq!(sha256(&published), before);
        }
        Err(_) => assert!(
            !destination.exists(),
            "failed publication exposed an incomplete final output"
        ),
        Ok(other) => panic!("unexpected archival outcome: {other:?}"),
    }
    assert_eq!(sha256(&source), before);
}
