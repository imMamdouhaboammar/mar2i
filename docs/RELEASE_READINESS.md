# Mar2i v1.0 release readiness evidence

Last reviewed: 2026-11-13. Scope: private repository `imMamdouhaboammar/mar2i`; macOS Apple Silicon local verification, GitHub CI, and cross-platform native compilation matrix. This is a living verification record, not a release approval.

**Classification: NOT READY for v1.0 Release Candidate.**

The documented release contract includes safe original-file handling, dependable recovery, real native desktop behavior, offline installation and independent runtime evidence on macOS Apple Silicon, macOS Intel and Windows x86-64. Passing core tests is necessary but does not meet those requirements.

## Remediation and feature deliveries completed

| Priority | Scope | Description | Evidence |
| --- | --- | --- | --- |
| P0 | Recovery | Recovery removed unrelated user files matching temporary filename prefix | [Issue #1](https://github.com/imMamdouhaboammar/mar2i/issues/1), [PR #5](https://github.com/imMamdouhaboammar/mar2i/pull/5) merged |
| P1 | Publication | On no-hardlink volumes, publication could leave truncated files visible | [Issue #3](https://github.com/imMamdouhaboammar/mar2i/issues/3), [PR #6](https://github.com/imMamdouhaboammar/mar2i/pull/6) merged |
| P1 | Integrity | Source bytes could change while file size and mtime remained unchanged | [Issue #4](https://github.com/imMamdouhaboammar/mar2i/issues/4), [PR #7](https://github.com/imMamdouhaboammar/mar2i/pull/7) merged |
| P1 | Concurrency | Another process could recover and reassign live work from active batch | [Issue #2](https://github.com/imMamdouhaboammar/mar2i/issues/2), [PR #8](https://github.com/imMamdouhaboammar/mar2i/pull/8) merged |
| P1 | Cross-Platform CI | Manual GitHub Actions workflow for cross-platform native Tauri compile | [PR #13](https://github.com/imMamdouhaboammar/mar2i/pull/13) merged; passed on Apple Silicon, Intel, Windows runners |
| P1 | Frontend & Lint | Enforce Oxlint anti-slop rules, strict TypeScript typing across widgets | [PR #14](https://github.com/imMamdouhaboammar/mar2i/pull/14) merged; 0 warnings, 22 Bun tests pass |
| P1 | Test Flake | Python SIGTERM ignore test timing flake resolved via stdout handshake | [PR #15](https://github.com/imMamdouhaboammar/mar2i/pull/15) merged; 100% deterministic |
| P1 | Data Safety | Atomic report writes (`write_reports`) and accurate `published_bytes` excluding failed/interrupted jobs | [PR #16](https://github.com/imMamdouhaboammar/mar2i/pull/16) merged; 5 report regression tests pass |
| P1 | Media Matrix | Synthetic multimedia correctness matrix covering H.264, HEVC, FFV1, VP9, Stream Copy, and Audio preservation | [PR #17](https://github.com/imMamdouhaboammar/mar2i/pull/17) merged; 6 integration tests pass (1.70s) |
| P2 | Benchmarks | Empirical directory discovery benchmarks measuring traversal scale at 100, 1K, and 10K files | [PR #18](https://github.com/imMamdouhaboammar/mar2i/pull/18) merged; scale bounds asserted |
| P1 | Packaging | Standalone offline FFmpeg/FFprobe binary staging, SHA-256 verification, and CI release workflow | [Issue #9](https://github.com/imMamdouhaboammar/mar2i/issues/9) resolved via `tools/package-offline.ts` & `.github/workflows/release.yml` |

### Reproduction and regression evidence

- **Hardlink failure protection**: A real, isolated **80 MiB ExFAT disk image** mounted on macOS rejected hard links. An explicit opt-in Rust integration test reproduced the previous incomplete-final-file failure under constrained capacity (**RED**) and passed after the publication fix (**GREEN**).
- **Silent content alteration**: A deterministic test reproduced a same-size in-place byte rewrite with the original modification time restored (**RED**), then passed after SHA256 source verification (**GREEN**).
- **Process lease**: A deterministic separate-OS-handle lock test demonstrated that concurrent recovery is refused while status reads remain non-blocking (**GREEN**).
- **Report accounting**: A regression test verified that stale `output_bytes` from failed/interrupted jobs are excluded from `published_bytes`, and reports are written atomically via temporary files with directory sync (**GREEN**).
- **Multimedia matrix**: Synthetic media tests confirmed:
  - H.264 mezzanine transcoding achieves automated `SSIM >= 0.965`.
  - HEVC publishing with explicit user codec preference produces compliant MP4 with verified SSIM.
  - FFV1 archival preserves decoded-frame hashes (`framemd5`) identically.
  - VP9 WebM encoding produces compliant WebM containers.
  - Stream copy preserves bit-for-bit identical bytes for already-optimal distribution MP4s without re-encoding.
  - Audio stream count and channels are preserved during transcoding.
- **Discovery throughput benchmarks** (Apple Silicon M-series):
  - 100 files: **760 µs** (<1 ms)
  - 1,000 files: **6.14 ms** (<10 ms)
  - 10,000 files: **56.53 ms** (<100 ms)

## Quality gate observations

| Gate | Observed result | Coverage boundary |
| --- | --- | --- |
| Bun frozen install | Passed | Local arm64 |
| TypeScript unit tests | **22 passed, 0 failed** | Tests include pure logic, widgets and UI state; not native E2E |
| Oxlint | **0 warnings, 0 errors** | Frontend sources (18 files, 115 rules) |
| TypeScript / Vite build | Passed | Web assets (dist/ bundle 112.65 kB JS, 48.32 kB CSS) |
| Rust Core rustfmt + Clippy `-D warnings` | Passed | Local arm64 |
| Rust Core full tests with real FFmpeg / FFprobe | **35 passed, 0 failed** | All 16 integration test suites passed; local arm64, FFmpeg 8.1.2 |
| Tauri rustfmt, Clippy `-D warnings`, `cargo check` | Passed | Compile/check only, arm64 |
| Tauri release `.app` bundle | Built an **unsigned** ARM64 `Mar2i.app` (12.74 MiB); launch process and actual Arabic app window observed | Local build smoke only, not a signed distributable or native E2E |
| macOS code-signing verification | Did not pass for the intentionally unsigned test bundle | Signing, hardened runtime and notarization are still release blockers |
| Anti-Slop AST, Rust triage and architectural drift | Passed, zero findings | Changed code only |
| Gitleaks staged changes | No leaks found | Does not replace complete supply-chain or history review |
| Cross-platform native compilation matrix | Passed on `macos-15` (arm64), `macos-15-intel` (x86_64), and `windows-2022` (x86_64) via GitHub Actions workflow | Compilation and clippy only; does not replace runtime verification |
| GitHub Actions standard CI | Failing due to repository billing limit ("account payments have failed or spending limit needs to be increased") | External account blocker, verified locally |
| CodeRabbit | All PRs (#5 through #18) reviewed with 0 actionable inline comments | Automated assistant review |

## Release acceptance gates

| Gate | Status | Evidence or outstanding requirement |
| --- | --- | --- |
| A. Data safety | **Verified for scoped paths** | Confirmed P0/P1 fixes, source SHA verification, and atomic reports |
| B. Processing reliability | **Verified for scoped paths** | Non-blocking OS lease, deterministic SIGTERM handling, idempotent recovery |
| C. Media correctness | **Verified for core codecs** | Concrete multimedia test matrix covering H.264, HEVC, FFV1, VP9, and Stream Copy |
| D. Desktop experience | **Partially verified** | Unsigned ARM64 app launches and renders Arabic UI; full E2E user workflow automation remains uncertified |
| E. Performance | **Verified for discovery** | Discovery scale benchmarks committed (10K files in 56.5ms) |
| F. Cross-platform | **Compile verified, runtime pending** | GitHub Actions matrix compiled cleanly on macOS ARM64/Intel and Windows; runtime testing on physical Intel/Windows hardware pending |
| G. Distribution & Packaging | **Packaging resolved, Signing pending** | Offline FFmpeg/FFprobe bundling resolved with SHA-256 manifest; production Apple/Windows code signing remains outstanding |

## Confirmed release blockers

1. [Issue #10](https://github.com/imMamdouhaboammar/mar2i/issues/10): Native desktop runtime verification on physical macOS Intel and Windows x86-64 hardware, installer smoke tests, Apple Developer ID signing, and notarization are outstanding.
2. [Issue #12](https://github.com/imMamdouhaboammar/mar2i/issues/12): Transitive Linux GTK RustSec advisories in `src-tauri/Cargo.lock` (`glib 0.18.5`, `proc-macro-error 1.0.4`); non-exploitable on macOS/Windows targets, tracked upstream.

## Decision

**Packaging & Offline Distribution Verified; Code Signing Pending.** While core data safety, recovery, multimedia transcoding correctness, and offline binary bundling are empirically verified, release acceptance requires resolving production Apple/Windows code signing and physical hardware verification.
