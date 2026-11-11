use mar2i_core::discovery::{discover, validate_directories};
use std::fs;
use tempfile::tempdir;

#[test]
fn recursively_discovers_only_supported_video_files_and_keeps_relative_paths() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join("موسم/حج")).unwrap();
    fs::write(dir.path().join("موسم/حج/فيديو 01.MOV"), b"a").unwrap();
    fs::write(dir.path().join("موسم/حج/notes.txt"), b"ignore").unwrap();
    fs::write(dir.path().join("موسم/another.mp4"), b"b").unwrap();
    let files = discover(dir.path()).unwrap();
    let relative: Vec<_> = files.iter().map(|f| f.relative.to_string_lossy().into_owned()).collect();
    assert_eq!(relative, vec!["موسم/another.mp4", "موسم/حج/فيديو 01.MOV"]);
    assert_eq!(files.iter().map(|f| f.size).sum::<u64>(), 2);
}

#[test]
fn rejects_overlapping_output_directories() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("source");
    let nested = input.join("compressed");
    fs::create_dir_all(&nested).unwrap();
    assert!(validate_directories(&input, &nested).is_err());
    assert!(validate_directories(&nested, &input).is_err());
    assert!(validate_directories(&input, &input).is_err());
    let output = dir.path().join("other");
    assert!(validate_directories(&input, &output).is_ok());
}

#[cfg(unix)]
#[test]
fn never_traverses_symlinked_directories() {
    use std::os::unix::fs::symlink;
    let dir = tempdir().unwrap();
    let outside = tempdir().unwrap();
    fs::write(outside.path().join("private.mp4"), b"do not read").unwrap();
    symlink(outside.path(), dir.path().join("linked")).unwrap();
    assert!(discover(dir.path()).unwrap().is_empty());
}
