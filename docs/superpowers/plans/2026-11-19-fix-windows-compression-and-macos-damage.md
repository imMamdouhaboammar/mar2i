# Windows Zero-Compression & macOS Damage Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate silent video copy fallback on Windows/publishing mode, ensure real cross-platform 2-Pass ABR compression with honest error reporting, and guarantee macOS bundle seal integrity with recursive deep codesigning and Gatekeeper sanitation.

**Architecture:** Refactor `core/src/policy.rs` and `core/src/runner.rs` to treat VFR as compressible via `-fps_mode passthrough`, replace non-portable null sinks with `-f null -`, normalize passlog paths, and enforce fail-closed error contracts across `src-tauri` and the UI. In CI and packaging scripts, recursively codesign all nested Mach-O sidecars inside `Mar2i.app` before packaging the DMG.

**Tech Stack:** Rust (Tauri v2, mar2i-core), FFmpeg/FFprobe, TypeScript (Bun, Vite), GitHub Actions (macOS-15, Windows-2022).

---

## Global Constraints

1. **No Silent Fallbacks:** Never copy the original input video to output and claim successful compression when compression was requested.
2. **Honest Error Contracts:** Surface the exact exit code and the last 15 lines of FFmpeg stderr on failure.
3. **Cross-Platform Null Sink:** Always use `-f null -` for first-pass ABR analysis across all platforms.
4. **macOS Bundle Seal:** All Mach-O binaries in `Contents/Resources/bin` must be signed with `--options runtime` and verified with `codesign --verify --deep --strict` before DMG generation.

---

## Tasks

- [ ] **Task 1: Core Policy & 2-Pass ABR Execution Refactoring**
  - [x] Edit `core/src/policy.rs`: Remove VFR copy triggers in `Mode::Publishing`.
  - [x] Edit `core/src/policy.rs`: Update `smart_abr_plan` to target 50% bitrate with anti-starvation floor and never copy original.
  - [x] Edit `core/src/runner.rs`: Surface exit code and last 15 lines of stderr in `checked_command`.
  - [x] Edit `core/src/runner.rs`: Set `command.stdin(Stdio::null())` in `run_command_cancellable`.
  - [ ] Edit `core/src/runner.rs`: Update `smart_abr` pass 1 to use universal `-f null -` and normalize passlog path with forward slashes.
  - [ ] Edit `core/src/runner.rs`: Expand hardware encoder selection to support `h264_qsv`, `h264_amf`, and `h264_vaapi` alongside `videotoolbox` and `nvenc`, with `libx264` fallback.

- [ ] **Task 2: Tauri IPC & Pre-flight Diagnostics**
  - [ ] Edit `src-tauri/src/lib.rs`: Add `verify_media_tools` startup check for `ffmpeg -version` and `ffprobe -version`.
  - [ ] Edit `src-tauri/src/lib.rs`: Strip `\\?\` UNC prefixes on Windows in `resolve_media_binary`.
  - [ ] Edit `src-tauri/src/lib.rs`: Reject `RunOutcome::Copied` in `compress_single_video` with an honest descriptive error.
  - [ ] Edit `core/src/report.rs`: Map `copied` jobs to `skipped` instead of silently ignoring or conflating with completed savings.

- [ ] **Task 3: Frontend Error Transparency & Status Clarity**
  - [ ] Edit `src/main.ts`: Distinguish between successful compression (>0% reduction) and optimal/uncompressed outcomes.
  - [ ] Edit `src/main.ts`: Add `copied` to recognized done statuses and label it "محفوظ دون تغيير".

- [ ] **Task 4: macOS Packaging & Deep Codesigning Pipeline**
  - [ ] Edit `.github/workflows/release.yml`: Deep-sign all nested binaries in `Contents/Resources/bin` and the `.app` bundle before DMG packaging.
  - [ ] Edit `.github/workflows/release.yml`: Add `codesign --verify --deep --strict --verbose=2` verification gate.
  - [ ] Verify `scripts/install-macos.sh` and update documentation for Gatekeeper quarantine clearance.

- [ ] **Task 5: Verification Gate & Physical Proof**
  - [ ] Update `core/tests/policy.rs` to reflect that VFR videos are compressed in publishing mode.
  - [ ] Run `cargo test` across all core and tauri test suites.
  - [ ] Run `bun test` and frontend lint.
  - [ ] Generate synthetic compressible video via FFmpeg `lavfi` and execute actual compression, verifying smaller byte size and differing SHA-256 hash.
