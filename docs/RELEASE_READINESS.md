# Mar2i v1.0 release readiness evidence

Last reviewed: 2026-11-11. Scope: private repository `imMamdouhaboammar/mar2i`; macOS Apple Silicon local verification and GitHub CI. This is a living verification record, not a release approval.

**Classification: NOT READY for v1.0 Release Candidate.**

The documented release contract includes safe original-file handling, dependable recovery, real native desktop behavior, offline installation and independent runtime evidence on macOS Apple Silicon, macOS Intel and Windows x86-64. Passing core tests is necessary but does not meet those requirements.

## Remediation completed

| Priority | Verified defect | Correction | Evidence |
| --- | --- | --- | --- |
| P0 | Recovery removed unrelated user files with a matching temporary filename prefix | Track only application-owned temporary files in SQLite; bound recovery cleanup to registered, validated paths | [Issue #1](https://github.com/imMamdouhaboammar/mar2i/issues/1), [PR #5](https://github.com/imMamdouhaboammar/mar2i/pull/5) merged |
| P1 | On no-hardlink volumes, disk-full or interrupted publication could leave a truncated file visible at the final filename | Publish a fully verified temporary output with `NamedTempFile::persist_noclobber`; never stream a fallback copy directly to the final path | [Issue #3](https://github.com/imMamdouhaboammar/mar2i/issues/3), [PR #6](https://github.com/imMamdouhaboammar/mar2i/pull/6) merged |
| P1 | Source bytes could change while file size and modification timestamp remained unchanged | Hash complete source content with SHA256 before work and before publication; check cancellation during streaming hash | [Issue #4](https://github.com/imMamdouhaboammar/mar2i/issues/4), [PR #7](https://github.com/imMamdouhaboammar/mar2i/pull/7) merged |
| P1 | Another process could recover and reassign live work from an active instance | Non-blocking OS file lease across recovery or processing; permit independent SQLite status readers | [Issue #2](https://github.com/imMamdouhaboammar/mar2i/issues/2), [PR #8](https://github.com/imMamdouhaboammar/mar2i/pull/8) merged |

### Reproduction and regression evidence

- A real, isolated **80 MiB ExFAT disk image** mounted on macOS rejected hard links. An explicit opt-in Rust integration test reproduced the previous incomplete-final-file failure under constrained capacity (**RED**) and passed after the publication fix (**GREEN**). The source SHA256 remained unchanged. This test is intentionally ignored in ordinary CI because it requires a separately mounted, bounded filesystem fixture.
- A deterministic test reproduced a same-size in-place byte rewrite with the original modification time restored (**RED**), then passed after SHA256 source verification (**GREEN**).
- A deterministic separate-OS-handle lock test demonstrated that recovery previously interrupted a live job (**RED**). Following the fix, concurrent recovery and a second batch start are refused, while status reads continue and recovery works after the lease is released (**GREEN**).
- The cancellation integration suite exercises real or controlled FFmpeg child-process termination, repeated cancellation and restart recovery. It passed on the local Apple Silicon machine.
- Existing codec integration tests exercise HEVC, AV1 and VP9 with synthetic media and available local encoders. Their passing results do not independently establish a complete stream-preservation matrix.

## Quality gate observations

| Gate | Observed result | Coverage boundary |
| --- | --- | --- |
| Bun frozen install | Passed | Local arm64 |
| TypeScript unit tests | **22 passed, 0 failed** | Tests include pure logic, widgets and UI state; not native E2E |
| Oxlint | **0 warnings, 0 errors** | Frontend sources |
| TypeScript / Vite build | Passed | Web assets only |
| Rust Core rustfmt + Clippy `-D warnings` | Passed | Local arm64 |
| Rust Core full tests with real FFmpeg / FFprobe | Passed | Local arm64, FFmpeg 8.1.2 |
| Tauri rustfmt, Clippy `-D warnings`, `cargo check` | Passed | Compile/check only, arm64 |
| Tauri release `.app` bundle | Built an **unsigned** ARM64 `Mar2i.app` (12.74 MiB) with `bunx tauri build --bundles app --no-sign --ci`; launch process and actual Arabic app window observed | Local build smoke only, not a signed distributable or native E2E |
| macOS code-signing verification | Did not pass for the intentionally unsigned test bundle | Signing, hardened runtime and notarization are still release blockers |
| Anti-Slop AST, Rust triage and architectural drift | Passed, no findings | Changed code only |
| Gitleaks staged changes | No leaks found | Does not replace complete supply-chain or history review |
| GitHub Actions | Ubuntu TypeScript and Rust Core jobs passed for PRs #5 to #8 | No Tauri packaging or macOS/Windows runtime jobs |
| CodeRabbit | Check completed with neutral verdict / no inline actionable comments observed | Not a manual security review |

A Linux CI pass or `cargo check` is not proof that an installed desktop app launches or performs correct work.

## Release acceptance gates

| Gate | Status | Evidence or outstanding requirement |
| --- | --- | --- |
| A. Data safety | **Partially verified** | Confirmed P0/P1 fixes and source SHA tests; more filesystem races, removable-drive faults and disk-full stages remain untested |
| B. Processing reliability | **Partially verified** | Real cancellation and restart tests, OS lease regression; full crash-injection lifecycle matrix remains outstanding |
| C. Media correctness | **Partially verified** | Synthetic FFmpeg codecs tested; full HDR, VFR, rotation, language/disposition, subtitles, chapters and sync matrix not executed |
| D. Desktop experience | **Partially verified** | The native ARM64 app launched and its Arabic window appeared. Folder selection through report and relaunch, keyboard/a11y review and RTL screen audit remain unverified |
| E. Quality | **Locally verified within stated scope** | Rust / Bun / lint / build pass; security and platform-specific tests still incomplete |
| F. Cross-platform | **Not verified** | No actual macOS Intel or Windows x86-64 installed-app test results |
| G. Distribution | **Blocked** | FFmpeg/FFprobe redistribution and fresh offline install are unresolved; signing/notarization not verified |

### Supported-platform evidence

| Target | Core and native compile | Installed app and media workflow | Installer/signing |
| --- | --- | --- | --- |
| macOS Apple Silicon | Core tests, Tauri cargo check and unsigned release `.app` build passed | App process and Arabic main window observed; full native E2E not completed | Unsigned test app only; production signing and notarization pending |
| macOS Intel | Not tested | Not tested | Pending |
| Windows x86-64 | Not tested | Not tested | Pending |

## Confirmed release blockers

1. [Issue #9](https://github.com/imMamdouhaboammar/mar2i/issues/9): Fresh offline installs have no bundled, legally reviewed FFmpeg and FFprobe. `src-tauri/tauri.conf.json` has no external binary or resource declarations; the native resolver falls back to local installations and PATH. CI's version-pinned test tools must not be assumed legally redistributable.
2. [Issue #10](https://github.com/imMamdouhaboammar/mar2i/issues/10): Native desktop E2E, platform-specific packaging, Intel and Windows runtime verification, signing and notarization evidence are missing.

### Further verification still needed

- Inject termination across each durable lifecycle transition, especially after publication but before SQLite completion, with exact output and state assertions.
- Perform real disk-full, permission-loss, source-replacement and removable-storage fault testing on each supported filesystem. File path checks cannot eliminate all external races.
- Benchmark one, two and higher supported worker counts on representative videos; measure CPU, peak memory, disk pressure, throughput and quality. **No performance results are asserted in this record.**
- Run discovery stress tests for 100, 1,000 and 10,000 lightweight files; run real encoding on a smaller representative corpus.
- Define per-mode codec and stream-preservation contracts. Distinguish byte-identical copying, decoded-frame identity, structural preservation and SSIM-based lossy quality gates. SSIM is not proof of visual equivalence.
- Execute complete native screens and interaction tests (New Batch, Options, Review, Progress, Results, Settings), including accurate Arabic RTL, keyboard and screen-reader behavior and exported report fidelity. The unsigned ARM64 app launched, but the UI automation attempt could not be completed reliably during concurrent desktop activity; no end-to-end result is claimed.
- Review dependency vulnerabilities, FFmpeg license/build provenance, third-party notices, media subprocess bounds and signing identities. HTML/CSV escaping tests pass; optional XLSX/PDF are not release requirements until scoped.

## Decision

**NOT READY.** The confirmed data-safety and concurrent recovery defects are repaired and merged, but release acceptance cannot be claimed until native E2E, offline distribution, cross-platform execution and licensing/signing gates have concrete passing evidence.
