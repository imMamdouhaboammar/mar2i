use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn cli_can_compress_real_directory_and_export_reports_without_touching_source() {
    let tmp = tempdir().unwrap();
    let source = tmp.path().join("مقاطع");
    let output = tmp.path().join("نتائج");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(&output).unwrap();
    let video = source.join("خام.avi");
    let generated = Command::new(std::env::var("MAR2I_TEST_FFMPEG").unwrap_or("ffmpeg".into()))
        .args(["-hide_banner", "-loglevel", "error", "-f", "lavfi",
            "-i", "testsrc2=size=160x96:rate=24", "-t", "0.5",
            "-c:v", "rawvideo", "-pix_fmt", "yuv420p", "-y"])
        .arg(&video).status().unwrap();
    assert!(generated.success());
    let before = fs::read(&video).unwrap();
    let run = Command::new(env!("CARGO_BIN_EXE_mar2i"))
        .arg("run").arg(&source).arg(&output).arg("archive").arg("low")
        .arg(std::env::var("MAR2I_TEST_FFMPEG").unwrap_or("ffmpeg".into()))
        .arg(std::env::var("MAR2I_TEST_FFPROBE").unwrap_or("ffprobe".into()))
        .output().unwrap();
    assert!(run.status.success(), "{:?}", String::from_utf8_lossy(&run.stderr));
    assert!(output.join("خام.avi.mar2i.mkv").exists());
    assert!(output.join(".mar2i/report.csv").exists());
    assert!(output.join(".mar2i/report.html").exists());
    assert_eq!(before, fs::read(&video).unwrap());
    let rerun = Command::new(env!("CARGO_BIN_EXE_mar2i"))
        .arg("run").arg(&source).arg(&output).arg("archive").arg("low")
        .arg(std::env::var("MAR2I_TEST_FFMPEG").unwrap_or("ffmpeg".into()))
        .arg(std::env::var("MAR2I_TEST_FFPROBE").unwrap_or("ffprobe".into()))
        .output().unwrap();
    assert!(rerun.status.success());
    assert_eq!(fs::read_dir(&output).unwrap().count(), 2);
}
