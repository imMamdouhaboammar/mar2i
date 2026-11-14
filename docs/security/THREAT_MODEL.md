# Threat Model: مرئي (Mar2i) — Bulk Video Compression & Archival

**Date**: 2026-11-11 | **Version**: 1.0.0 | **Author**: Security Architect Division

## 1. System Overview
- **Architecture**: Hybrid Desktop Application — Client WebView (TypeScript/Vite) + Native Rust Desktop Shell (Tauri v2) + Native Rust Processing Engine (`mar2i-core`).
- **Tech Stack**:
  - Frontend: TypeScript, Vite, Vanilla Web Components, OKLCH/CSS.
  - Native Shell: Tauri v2, `@tauri-apps/api`, `@tauri-apps/plugin-dialog`.
  - Core Processing: Rust (`mar2i-core`), `rusqlite` (WAL mode queue), FFmpeg / FFprobe native discovery.
- **Data Classification**: Sensitive Media & Archival Assets (Saudi Ministry of Hajj & Umrah media assets, high-resolution ceremony footage, press releases).
- **Deployment**: Standalone Air-Gapped / Offline Desktop Executable (macOS / Linux / Windows).
- **External Integrations**: Zero external cloud or network integrations. Strictly air-gapped / offline by design.

---

## 2. Trust Boundaries

| Boundary | From | To | Controls & Enforcements |
|---|---|---|---|
| **TB-1: UI ↔ Tauri IPC** | Untrusted WebView DOM | Tauri IPC Handlers | Typed IPC commands (`analyze_directory`, `start_batch`, `control_batch`, `batch_status`); no arbitrary shell execution; input path canonicalization. |
| **TB-2: IPC ↔ File System** | Native Rust Core | Local File System | Explicit source and output directory boundaries; canonical path resolution; symlink traversal rejection; no in-place overwrite. |
| **TB-3: Core ↔ Subprocess** | Native Runner | `ffmpeg` / `ffprobe` | Fixed parameter vectors; strict separation of options and filenames; shell invocation bypassed (direct `std::process::Command` exec); timeout and process group containment. |
| **TB-4: Core ↔ Database** | Native Worker | SQLite WAL Queue | Parameterized SQL statements; local-only database file; file-level permission isolation; crash-consistent transactions. |
| **TB-5: Export ↔ External App** | CSV / HTML Export | Spreadsheet Software / Browser | Formula injection neutralization (prefixing `[=+@-\t\r]` with `'`); HTML character escaping (`escapeHtml`). |

---

## 3. STRIDE Analysis

| Threat Category | Target Component | Risk | Concrete Attack Scenario | Mitigation Architecture |
|---|---|---|---|---|
| **Spoofing** | Media Metadata & Identity | High | Malicious video file fakes codec parameters or header values to evade policy checks. | Hard ffprobe stream validation (`media.rs`); decoded frame-level hash checks in archive mode; SSIM validation ($\ge 0.965$) in publish mode. |
| **Tampering** | Directory Traversal / Symlinks | High | Crafting symlink pointing to `/etc` or sensitive directories inside source tree to read/overwrite unauthorized files. | `discovery.rs` explicitly rejects symlinks, validates canonical paths, and checks that every scanned file resides within the selected source root. |
| **Repudiation** | Audit Reports & Logs | Med | Operator denies batch execution or attempts to forge compression metrics. | Deterministic job status tracking with SQLite WAL logging; immutable batch report generation with SHA-256 sparse hash verification and Merkle tree audit roots. |
| **Information Disclosure** | Subprocess / UI Errors | Med | Subprocess failure outputs stack traces, environment variables, or private directory structures to UI. | Structured error handling; domain-mapped error messages; sensitive environment details scrubbed before delivery to WebView. |
| **Denial of Service** | Resource Exhaustion (Zip Bombs / Decompression Bombs) | High | Submitting recursive directories or multi-gigabyte corrupted streams that crash ffmpeg or exhaust host memory. | Bounded concurrency workers based on Resource profile (`low`, `balanced`, `maximum`); explicit memory limits; process timeouts; early zero-byte and oversized output abort gates. |
| **Elevation of Privilege** | Command Injection via Filenames | Crit | File containing shell metacharacters (`$(rm -rf /)`, `;`, `|`, `&`) in filename attempting arbitrary command execution during encoding. | Direct vector-based subprocess execution via `std::process::Command` with argument array passing — no intermediate shell invocation (`sh -c` or `cmd.exe`) is ever permitted. |

---

## 4. Attack Surface Inventory

- **External Attack Surface**: None. Application binds zero external network listeners, makes no telemetry or update HTTP calls, and operates 100% offline.
- **Local Desktop Attack Surface**:
  - Directory selection picker (`pickDirectory`)
  - Batch job queue database (`mar2i.db` in app data)
  - Generated reports (`.csv`, `.html`)
  - Subprocess pipes (`stdin`, `stdout`, `stderr` of `ffmpeg`/`ffprobe`)
- **Supply Chain**:
  - Rust crates audited via `cargo audit`
  - NPM dependencies managed via Bun and locked in `bun.lock`
  - Oxlint anti-slop rules guarding against ungrounded type assertions and module mocking leaks

---

## 5. Security Test Coverage Checklist

- [x] **Symlink Rejection**: Enforced in `core/src/discovery.rs` and verified in test suite.
- [x] **Path Traversal Gate**: Enforced in path normalization and canonicalization.
- [x] **CSV Formula Injection**: Escaped via `safeCsvCell` (tested in `src/data.test.ts`).
- [x] **HTML XSS Prevention**: Neutralized via `escapeHtml` (tested in `src/data.test.ts`).
- [x] **Truthful State Guard**: Disallows synthetic certificates or fake savings before real batch exists (tested in `src/native-honesty.test.ts`).
- [x] **Fail-Closed Processing**: Rejects zero-byte outputs or larger-than-source outputs without silent fallback.
