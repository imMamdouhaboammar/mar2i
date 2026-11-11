---
name: mar2i-ops
description: >
  Operational guide, CLI handbook, and Model Context Protocol (MCP) specification for
  mar2i (مرئي) — the bulk video compression and archival platform for the Saudi Ministry
  of Hajj and Umrah. Use when automating video intake, executing offline batch transcoding,
  probing hardware acceleration, predicting storage savings via stratified sampling,
  running the MCP server, or auditing archival integrity with SSIM and framemd5.
version: "1.0.0"
author: "Mamdouh Aboammar"
license: "MIT"
metadata:
  target_system: "mar2i (Tauri 2.0 / Rust / Bun / TypeScript)"
  audience: "Hajj & Umrah media engineers and autonomous AI agents"
---

# mar2i-ops: Local Video Compression & Archival Operations

> Development guide only. The app is not an officially approved Ministry product. GPU encoding, AV1/HEVC processing, trimming and visual audit widgets remain disconnected from native batch execution. Sparse fingerprints are similarity hints, not full-file integrity proofs.

> Autonomous operations and programmatic integration manual for `mar2i` (مرئي), the high-assurance bulk video compression and archival workstation engine for the Saudi Ministry of Hajj and Umrah.

---

## 1. Non-Negotiable System Invariants

1. **Original File Immutability**: Source footage files are strictly read-only. `mar2i` never writes into, renames, modifies, or deletes an input file.
2. **Archival Lossless Isolation**: In `Mode::Archival`, media is encoded exclusively via CPU FFV1 in Matroska (`.mkv`) with frame-by-frame `framemd5` verification. Hardware GPU encoding is strictly prohibited in Archival mode.
3. **Hardware Acceleration Boundary**: Hardware probing exists, but the production runner currently uses CPU H.264 publishing only. GPU encoding must not be reported as active.
4. **Sleep Prevention**: A best-effort RAII assertion is acquired during a batch; pause currently holds it until the worker exits. The UI has no live assertion confirmation.
5. **Sparse Fingerprint Registry**: The optional, unconnected registry can offer a quick similarity hint, never proof of full-file identity, via a bounded-read SHA-256 fingerprint: $\text{SHA-256}(\text{size}_{\text{LE}} \parallel \text{head}_{1\text{MB}} \parallel \text{tail}_{1\text{MB}})$.

---

## 2. Command-Line Interface (CLI) Handbook

The `mar2i` CLI is a self-contained, standalone Rust binary built from `core/src/bin/mar2i.rs`.

### 2.1 Batch Scanning & Ingestion
```bash
# Scan and evaluate candidate footage without modifying disk
mar2i scan /Volumes/HajjFootage/Mina_Day1 /Volumes/ArchiveOutput/Mina_Day1 publish balanced
```

### 2.2 Unattended Batch Processing
```bash
# Run batch transcode with SQLite state machine recovery and KeepAwake
mar2i run /Volumes/HajjFootage/Arafat /Volumes/ArchiveOutput/Arafat archive maximum /opt/homebrew/bin/ffmpeg /opt/homebrew/bin/ffprobe
```

### 2.3 Hardware Encoder Probe
```bash
# Verify GPU acceleration support via 1-frame probe test
mar2i probe-hw
```

### 2.4 Empirical Stratified Size Estimation
```bash
# Probe 3 evenly distributed slices to predict compressed size and savings
mar2i estimate /Volumes/HajjFootage/Tawaf_Raw.mov ffmpeg ffprobe hevc 22
```

### 2.5 Sparse SHA-256 Media Fingerprinting
```bash
# Generate deterministic O(1) fingerprint in <1 ms
mar2i fingerprint /Volumes/HajjFootage/4K_Master.mov
```

### 2.6 Ministry Media Profiles
```bash
# List example Hajj & Umrah profiles (not officially approved)
mar2i profiles
```

---

## 3. Model Context Protocol (MCP) Server

AI agents (Antigravity, Claude, custom orchestrators) interface with `mar2i` via its stdio JSON-RPC 2.0 MCP server:

```bash
mar2i mcp
```

### Client Configuration (`mcpServers`)
```json
{
  "mcpServers": {
    "mar2i": {
      "command": "/path/to/mar2i",
      "args": ["mcp"]
    }
  }
}
```

### Available Tools

| Tool Name | Parameters | Purpose |
| :--- | :--- | :--- |
| `mar2i_analyze_directory` | `input_dir`, `output_dir` | Discovers media, checks stream validity, returns volume |
| `mar2i_detect_hardware` | *(none)* | Probes and returns host GPU hardware encoders |
| `mar2i_list_profiles` | *(none)* | Returns proposed profiles, not approved presets |
| `mar2i_fingerprint_file` | `file_path` | Generates instantaneous sparse SHA-256 hash |

---

## 4. Visual Quality Verification (A/B & SSIM)

During the Review stage (`Screen::Review`), `mar2i` mounts an accessible, RTL-native `CompareSlider`:
- The current widget compares sample images in the browser demo only, not actual processed media.
- Displays a sample SSIM number in the demo; production SSIM is computed separately inside the Rust runner.
- The processing core currently uses a provisional publishing acceptance gate of SSIM >= 0.965; this score is not proof of human-perceived equivalence.
