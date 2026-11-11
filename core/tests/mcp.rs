use mar2i_core::mcp::run_mcp_server;
use serde_json::{Value, json};
use std::io::Cursor;
use std::path::PathBuf;

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
    let tools = tools_res["result"]["tools"].as_array().expect("tools array");
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
    assert!(res["result"]["content"][0]["text"].as_str().unwrap().contains("master_archive"));
}
