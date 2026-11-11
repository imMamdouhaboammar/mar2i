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
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=160x96:rate=24",
            "-t",
            "0.5",
            "-c:v",
            "rawvideo",
            "-pix_fmt",
            "yuv420p",
            "-y",
        ])
        .arg(&video)
        .status()
        .unwrap();
    assert!(generated.success());
    let before = fs::read(&video).unwrap();
    let run = Command::new(env!("CARGO_BIN_EXE_mar2i"))
        .arg("run")
        .arg(&source)
        .arg(&output)
        .arg("archive")
        .arg("low")
        .arg(std::env::var("MAR2I_TEST_FFMPEG").unwrap_or("ffmpeg".into()))
        .arg(std::env::var("MAR2I_TEST_FFPROBE").unwrap_or("ffprobe".into()))
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{:?}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(output.join("خام.avi.mar2i.mkv").exists());
    assert!(output.join(".mar2i/report.csv").exists());
    assert!(output.join(".mar2i/report.html").exists());
    assert_eq!(before, fs::read(&video).unwrap());
    let rerun = Command::new(env!("CARGO_BIN_EXE_mar2i"))
        .arg("run")
        .arg(&source)
        .arg(&output)
        .arg("archive")
        .arg("low")
        .arg(std::env::var("MAR2I_TEST_FFMPEG").unwrap_or("ffmpeg".into()))
        .arg(std::env::var("MAR2I_TEST_FFPROBE").unwrap_or("ffprobe".into()))
        .output()
        .unwrap();
    assert!(rerun.status.success());
    assert_eq!(fs::read_dir(&output).unwrap().count(), 2);
}

#[test]
fn mcp_cli_grants_only_explicitly_configured_media_roots() {
    use std::io::Write;
    use std::process::Stdio;

    let tmp = tempdir().unwrap();
    let allowed = tmp.path().join("allowed");
    fs::create_dir_all(&allowed).unwrap();
    let video = allowed.join("video.mp4");
    fs::write(&video, b"synthetic").unwrap();
    let request = format!(
        "{}\n",
        serde_json::json!({
            "jsonrpc": "2.0", "id": 9, "method": "tools/call",
            "params": { "name": "mar2i_fingerprint_file", "arguments": { "file_path": video.to_string_lossy() } }
        })
    );
    let mut child = Command::new(env!("CARGO_BIN_EXE_mar2i"))
        .arg("mcp")
        .env(
            "MAR2I_MCP_ALLOWED_ROOTS",
            std::env::join_paths([&allowed]).unwrap(),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(request.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let response: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(response["result"].get("isError").is_none(), "{response}");
    assert!(
        response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("size_bytes")
    );
}
