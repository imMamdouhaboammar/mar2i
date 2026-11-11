//! Model Context Protocol (MCP) Server for mar2i.
//!
//! Exposes mar2i core batch processing, discovery, estimation, and hardware detection
//! tools to autonomous AI agents (Claude, Antigravity, custom harnesses) via standard
//! JSON-RPC 2.0 over stdin/stdout.

use crate::discovery::{discover, validate_directories};
use crate::fingerprint::compute_fingerprint;
use crate::hardware::detect_hardware;
use crate::profiles::standard_profiles;
use serde_json::{Value, json};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

const PROTOCOL_VERSION: &str = "2024-11-05";

pub fn run_mcp_server<R: BufRead, W: Write>(mut reader: R, mut writer: W, ffmpeg: &Path) {
    let mut line = String::new();

    while let Ok(bytes_read) = reader.read_line(&mut line) {
        if bytes_read == 0 {
            break;
        }

        let trimmed = line.trim();
        if !trimmed.is_empty()
            && let Ok(request) = serde_json::from_str::<Value>(trimmed)
            && let Some(response) = handle_jsonrpc_message(&request, ffmpeg)
        {
            let _ = writeln!(writer, "{}", response);
            let _ = writer.flush();
        }

        line.clear();
    }
}

fn handle_jsonrpc_message(msg: &Value, ffmpeg: &Path) -> Option<Value> {
    let id = msg.get("id").cloned();
    let method = msg.get("method").and_then(Value::as_str)?;

    match method {
        "initialize" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": PROTOCOL_VERSION,
                "serverInfo": {
                    "name": "mar2i-mcp",
                    "version": "0.1.0"
                },
                "capabilities": {
                    "tools": {}
                }
            }
        })),
        "notifications/initialized" => None,
        "tools/list" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": get_tool_definitions()
            }
        })),
        "tools/call" => {
            let params = msg.get("params")?;
            let tool_name = params.get("name").and_then(Value::as_str)?;
            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

            let tool_result = dispatch_tool(tool_name, &arguments, ffmpeg);
            Some(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": tool_result
            }))
        }
        "ping" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {}
        })),
        _ => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32601,
                "message": format!("Method not found: {}", method)
            }
        })),
    }
}

pub fn get_tool_definitions() -> Value {
    json!([
        {
            "name": "mar2i_analyze_directory",
            "description": "Scans an input directory of video footage for the Saudi Ministry of Hajj & Umrah, reporting total file volume, format distribution, and processing admissibility.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "input_dir": {
                        "type": "string",
                        "description": "Absolute path to source video directory."
                    },
                    "output_dir": {
                        "type": "string",
                        "description": "Absolute path to destination output directory."
                    }
                },
                "required": ["input_dir", "output_dir"]
            }
        },
        {
            "name": "mar2i_detect_hardware",
            "description": "Detects host graphics card hardware acceleration capabilities (VideoToolbox, NVENC, QSV, AMF) via 1-frame probe test.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        },
        {
            "name": "mar2i_list_profiles",
            "description": "Lists example media processing profile proposals, not government-approved presets. Only automatic H.264 and limited archival FFV1 processing are currently connected.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        },
        {
            "name": "mar2i_fingerprint_file",
            "description": "Computes a sparse SHA-256 similarity hint (size + 1 MiB head + 1 MiB tail). Not a full content-integrity hash or a deduplication guarantee.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Absolute path to media file."
                    }
                },
                "required": ["file_path"]
            }
        }
    ])
}

fn dispatch_tool(name: &str, args: &Value, ffmpeg: &Path) -> Value {
    match name {
        "mar2i_detect_hardware" => {
            let report = detect_hardware(ffmpeg);
            json!({
                "content": [
                    {
                        "type": "text",
                        "text": serde_json::to_string_pretty(&report).unwrap_or_default()
                    }
                ]
            })
        }
        "mar2i_list_profiles" => {
            let profiles = standard_profiles();
            json!({
                "content": [
                    {
                        "type": "text",
                        "text": serde_json::to_string_pretty(&profiles).unwrap_or_default()
                    }
                ]
            })
        }
        "mar2i_analyze_directory" => {
            let input_str = match args.get("input_dir").and_then(Value::as_str) {
                Some(s) => s,
                None => return tool_error("input_dir string parameter is required"),
            };
            let output_str = match args.get("output_dir").and_then(Value::as_str) {
                Some(s) => s,
                None => return tool_error("output_dir string parameter is required"),
            };

            let in_path = PathBuf::from(input_str);
            let out_path = PathBuf::from(output_str);

            match validate_directories(&in_path, &out_path) {
                Ok(_) => match discover(&in_path) {
                    Ok(scanned) => {
                        let total_bytes: u64 = scanned.iter().map(|f| f.size).sum();
                        json!({
                            "content": [
                                {
                                    "type": "text",
                                    "text": serde_json::to_string_pretty(&json!({
                                        "status": "valid",
                                        "total_files": scanned.len(),
                                        "total_bytes": total_bytes,
                                        "total_gb": format!("{:.2} GB", total_bytes as f64 / (1024.0 * 1024.0 * 1024.0)),
                                        "files": scanned.iter().take(20).map(|f| &f.relative).collect::<Vec<_>>()
                                    })).unwrap_or_default()
                                }
                            ]
                        })
                    }
                    Err(e) => tool_error(&format!("Discovery error: {e}")),
                },
                Err(e) => tool_error(&format!("Validation error: {e}")),
            }
        }
        "mar2i_fingerprint_file" => {
            let file_str = match args.get("file_path").and_then(Value::as_str) {
                Some(s) => s,
                None => return tool_error("file_path string parameter is required"),
            };
            let p = PathBuf::from(file_str);
            match compute_fingerprint(&p) {
                Ok(fp) => json!({
                    "content": [
                        {
                            "type": "text",
                            "text": serde_json::to_string_pretty(&fp).unwrap_or_default()
                        }
                    ]
                }),
                Err(e) => tool_error(&format!("Fingerprinting failed: {e}")),
            }
        }
        _ => tool_error(&format!("Unknown tool: {}", name)),
    }
}

fn tool_error(message: &str) -> Value {
    json!({
        "isError": true,
        "content": [
            {
                "type": "text",
                "text": message
            }
        ]
    })
}
