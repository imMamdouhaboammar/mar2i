use mar2i_core::mcp::{run_mcp_server, run_mcp_server_scoped};
use serde_json::{Value, json};
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn mcp_server_handles_initialize_and_tools_list() {
    let input = format!(
        "{}\n{}\n",
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {}
            }
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        })
    );

    let reader = Cursor::new(input.into_bytes());
    let mut writer = Vec::new();
    let ffmpeg = PathBuf::from("ffmpeg");

    run_mcp_server(reader, &mut writer, &ffmpeg);

    let output_str = String::from_utf8(writer).expect("valid utf8 output");
    let lines: Vec<&str> = output_str.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2, "expected 2 responses for 2 requests");

    let init_res: Value = serde_json::from_str(lines[0]).expect("json parse init");
    assert_eq!(init_res["id"], 1);
    assert_eq!(init_res["result"]["serverInfo"]["name"], "mar2i-mcp");

    let tools_res: Value = serde_json::from_str(lines[1]).expect("json parse tools");
    assert_eq!(tools_res["id"], 2);
    let tools = tools_res["result"]["tools"]
        .as_array()
        .expect("tools array");
    assert!(tools.iter().any(|t| t["name"] == "mar2i_analyze_directory"));
    assert!(tools.iter().any(|t| t["name"] == "mar2i_detect_hardware"));
    assert!(tools.iter().any(|t| t["name"] == "mar2i_list_profiles"));
    assert!(tools.iter().any(|t| t["name"] == "mar2i_fingerprint_file"));
}

#[test]
fn mcp_server_executes_tool_call_list_profiles() {
    let input = format!(
        "{}\n",
        json!({
            "jsonrpc": "2.0",
            "id": 42,
            "method": "tools/call",
            "params": {
                "name": "mar2i_list_profiles",
                "arguments": {}
            }
        })
    );

    let reader = Cursor::new(input.into_bytes());
    let mut writer = Vec::new();
    let ffmpeg = PathBuf::from("ffmpeg");

    run_mcp_server(reader, &mut writer, &ffmpeg);

    let output_str = String::from_utf8(writer).expect("valid utf8 output");
    let res: Value = serde_json::from_str(output_str.trim()).expect("json parse");
    assert_eq!(res["id"], 42);
    assert!(
        res["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("master_archive")
    );
}

#[test]
fn mcp_denies_file_access_without_an_explicit_root_grant() {
    let tmp = tempdir().unwrap();
    let secret = tmp.path().join("private.mp4");
    fs::write(&secret, b"fixture-not-private").unwrap();
    let request = format!(
        "{}\n",
        json!({
            "jsonrpc": "2.0", "id": 7, "method": "tools/call",
            "params": {
                "name": "mar2i_fingerprint_file",
                "arguments": { "file_path": secret.to_string_lossy() }
            }
        })
    );
    let mut writer = Vec::new();
    run_mcp_server(
        Cursor::new(request.into_bytes()),
        &mut writer,
        &PathBuf::from("ffmpeg"),
    );
    let response: Value = serde_json::from_slice(&writer).unwrap();
    assert_eq!(response["result"]["isError"], true);
    assert!(
        response["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("authorized")
    );
}

#[test]
fn mcp_scoped_access_rejects_outside_and_symlink_escape_paths() {
    let tmp = tempdir().unwrap();
    let allowed = tmp.path().join("allowed");
    let outside = tmp.path().join("outside");
    fs::create_dir_all(&allowed).unwrap();
    fs::create_dir_all(&outside).unwrap();
    let inside_file = allowed.join("inside.mp4");
    let outside_file = outside.join("private.mp4");
    fs::write(&inside_file, b"ok").unwrap();
    fs::write(&outside_file, b"out-of-scope").unwrap();

    let call = |path: &std::path::Path| {
        let request = format!(
            "{}\n",
            json!({
                "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                "params": {
                    "name": "mar2i_fingerprint_file",
                    "arguments": { "file_path": path.to_string_lossy() }
                }
            })
        );
        let mut writer = Vec::new();
        run_mcp_server_scoped(
            Cursor::new(request.into_bytes()),
            &mut writer,
            &PathBuf::from("ffmpeg"),
            std::slice::from_ref(&allowed),
        );
        serde_json::from_slice::<Value>(&writer).unwrap()
    };

    assert!(call(&inside_file)["result"].get("isError").is_none());
    assert_eq!(call(&outside_file)["result"]["isError"], true);
    assert_eq!(
        call(&allowed.join("../outside/private.mp4"))["result"]["isError"],
        true
    );

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&outside_file, allowed.join("link.mp4")).unwrap();
        assert_eq!(call(&allowed.join("link.mp4"))["result"]["isError"], true);
    }
}
