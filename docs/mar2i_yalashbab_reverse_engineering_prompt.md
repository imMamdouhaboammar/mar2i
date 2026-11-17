# Master Agent Prompt: Reverse-Engineer `yalashbab.com/video/` & Replicate its Exact 2-Pass ABR Compression Pipeline Inside `mar2i` (Local Tauri v2 + Rust Desktop App)

## 0. Role, Mission & Ground Truth Context

You are a Principal Media Systems Architect and Senior Rust/Tauri Engineer operating inside the **`mar2i` (`مرئي`)** repository (Tauri v2 + `mar2i-core` Rust crate + Vanilla TypeScript/Vite RTL UI).

Your mission is to:
1. Perform a forensic reverse-engineering audit of **`https://yalashbab.com/video/`** (and cross-verify it against the binary bitstream telemetry extracted from its live output) to document its exact UI flow, target bitrate formula, 2-pass encoding lifecycle, audio handling, and container muxing behavior.
2. Replicate (`mimic`) that exact compression engine, formula, and streamlined workflow **100% locally and offline** inside the `mar2i` desktop application (`core/`, `src-tauri/`, and `src/`), while preserving `mar2i`'s strict institutional safety guarantees (read-only source immutability, atomic temp-file publication, SQLite WAL crash recovery, and zero fake UI metrics).

---

## 1. Forensic Reverse-Engineering Baseline of `https://yalashbab.com/video/`

Before writing or modifying code, anchor your implementation to the verified forensic telemetry already extracted from `https://yalashbab.com/video/` on the 30-second 1080p benchmark (`sample-30s.mp4` -> `compressed.mp4`):

### 1.1 Input vs. Output Bitstream Telemetry (Verified Ground Truth)
| Parameter | Original Input (`sample-30s.mp4`) | `yalashbab.com/video/` Output (`compressed.mp4`) | Exact Engineering Behavior |
| :--- | :--- | :--- | :--- |
| **File Size** | `21,657,943` bytes (`20.65 MB`) | `10,784,658` bytes (`10.29 MB`) | **-50.20% exact size reduction** |
| **Container & Atoms** | `MP4` (`isom/avc1/mp41`) | `MP4` (`isom/iso2/avc1/mp41`), `moov` atom at byte offset `32` | **`-movflags +faststart`** (relocates `moov` before `mdat`) |
| **Resolution & Aspect** | `1920x1080` (`SAR 1:1`, `DAR 16:9`) | `1920x1080` (`SAR 1:1`, `DAR 16:9`) | **Zero downscaling** (100% spatial preservation) |
| **Frame Rate & Count** | `30.00 fps` (`911` frames, `30.366s`) | `30.00 fps` (`911` frames, `30.366s`) | **Zero frame drops** (`-fps_mode passthrough`) |
| **Video Codec & Profile** | `H.264 / AVC` (`High@L4.0`, `yuv420p`, 8-bit) | `H.264 / AVC` (`High@L4.0`, `yuv420p`, 8-bit) | Preserves `High@L4.0` and `yuv420p` |
| **Video Bitrate** | `5,569,685 bps` (`~5,570 kbps`) | `2,705,169 bps` (`~2,705 kbps`) | **-51.43% video bitrate reduction** |
| **Total Mux Bitrate** | `5,687,298 bps` (`~5,687 kbps`) | `2,834,245 bps` (`~2,834 kbps`) | **-50.17% total bitrate reduction** |
| **Audio Stream** | `AAC LC`, `44,100 Hz`, Stereo, `126,541 bps` (`1,312` frames) | `AAC LC`, `44,100 Hz`, Stereo, `126,541 bps` (`1,312` frames) | **`-c:a copy`** when AAC (identical Audio MD5: `eb4430571a8c11b9a86ede01126a44ea`) |
| **Visual Fidelity** | Reference (`1.0000`) | **`SSIM = 0.958814`** (`All: 13.85 dB`), **`PSNR = 33.41 dB`** | High visual retention on complex moving foliage at 50% size |

### 1.2 Extracted `x264` SEI NAL Unit Signature from `compressed.mp4`
The output binary contains the following verbatim `libx264` SEI configuration string:
```text
Lavc62.28.103 libx264
x264 - core 165 - H.264/MPEG-4 AVC codec - Copyleft 2003-2025 - options:
cabac=1 ref=2 deblock=1:0:0 analyse=0x3:0x113 me=hex subme=6 psy=1 psy_rd=1.00:0.00
mixed_ref=1 me_range=16 chroma_me=1 trellis=1 8x8dct=1 cqm=0 deadzone=21,11
fast_pskip=1 chroma_qp_offset=-2 threads=1 lookahead_threads=1 sliced_threads=0 nr=0
decimate=1 interlaced=0 bluray_compat=0 constrained_intra=0 bframes=3 b_pyramid=2
b_adapt=1 b_bias=0 direct=1 weightb=1 open_gop=0 weightp=1 keyint=250 keyint_min=25
scenecut=40 intra_refresh=0 rc_lookahead=30 rc=2pass mbtree=1 bitrate=2700
ratetol=1.0 qcomp=0.60 qpmin=0 qpmax=69 qpstep=4 cplxblur=20.0 qblur=0.5
ip_ratio=1.40 aq=1:1.00
```

### 1.3 What Every Parameter in that SEI Header Proves
1. **Preset Identity (`-preset fast`)**: In `libx264`, the tuple `(ref=2, subme=6, rc_lookahead=30, weightp=1, me=hex, trellis=1)` is the exact canonical fingerprint of **`-preset fast`** (whereas `-preset medium` sets `ref=3, subme=7, rc_lookahead=40, weightp=2`).
2. **Rate Control (`rc=2pass`, `mbtree=1`, `bitrate=2700`)**: The engine does **not** use 1-pass constant CRF (`rc=crf`); it executes a deterministic **2-Pass Average Bitrate (`-b:v 2700k -pass 1` then `-pass 2`)** with Macroblock-Tree (`mbtree=1`) lookahead across 30 frames to allocate bits dynamically to high-motion frames while strictly hitting the target file size.
3. **Target Bitrate Calculation Formula**:
   Given `input_bytes = 21,657,943`, `duration = 30.441s`, `audio_bitrate = 126,541 bps`, and `target_ratio = 0.50` (50% size reduction):
   $$\text{Target Total Bitrate (kbps)} = \frac{\text{input\_bytes} \times 8 \times \text{target\_ratio}}{\text{duration} \times 1000} = \frac{21,657,943 \times 8 \times 0.50}{30.441 \times 1000} \approx 2,845.9 \text{ kbps}$$
   Subtracting the preserved audio stream (`126.5 kbps`) and MP4 container muxing overhead (`~18.5 kbps`) and rounding to the nearest `50 kbps` step yields:
   $$\text{Target Video Bitrate} = \text{round\_to\_50}(2845.9 - 126.5 - 18.5) = \mathbf{2,700 \text{ kbps}}$$

---

## 2. Critical Architectural Diagnosis of `mar2i` (Why Current `mar2i` Behaves Differently)

When you inspect the current `mar2i` codebase (`core/src/policy.rs` and `core/src/runner.rs`), you will find **4 critical root causes** where `mar2i` currently diverges from `yalashbab.com/video/` and **fails to compress** the exact same `sample-30s.mp4` file:

1. **Root Cause #1 — The `BPP <= 0.14` Guard in `core/src/policy.rs` (Lines 232–255) Rejects `sample-30s.mp4`!**
   * In `sample-30s.mp4` (`1920x1080 @ 30fps`, `video_bitrate = 5,569,685 bps`), the bits-per-pixel-per-frame is:
     $$\text{BPP} = \frac{5,569,685}{1920 \times 1080 \times 30} = \mathbf{0.08953}$$
   * Because `0.08953 <= 0.14`, `choose_plan_with_preference()` in `core/src/policy.rs` immediately returns `Decision::Copy { reason: "already-efficient H.264 source" }`!
   * In other words: if a user drops `sample-30s.mp4` (`20.65 MB`) into `mar2i` today, **`mar2i` refuses to compress it and copies it unchanged at `20.65 MB` (0% savings)**, whereas `yalashbab.com/video/` compresses it to `10.29 MB` (-50.20%)!
2. **Root Cause #2 — 1-Pass Fixed CRF (`-crf 21 -preset medium`) vs. 2-Pass Target-Bitrate ABR (`-preset fast -b:v <target>k -pass 1/2`)**:
   * `core/src/runner.rs` (Lines 457–483) only supports 1-pass `-crf` encoding with `-preset medium`, which has no awareness of target file size reduction (`-50%`) and cannot guarantee a predictable output size.
3. **Root Cause #3 — The Hard `SSIM < 0.965` Rejection Gate in `core/src/runner.rs` (Line 490)**:
   * On `sample-30s.mp4` (dense moving park foliage), 50% size reduction via `libx264` 2-pass `2700k` achieves `SSIM = 0.958814` (`~0.959`).
   * Current `runner.rs` hard-rejects and deletes any output with `score < 0.965`! For target-size reduction mode (`YalaShbab 2-Pass ABR`), the SSIM gate must be configurable (e.g. `0.945` floor for 50% size reduction mode, while keeping `0.965` for standard CRF Broadcast/Publish presets).
4. **Root Cause #4 — Audio Codec Rejection (`core/src/policy.rs` Line 219) & Folder-Only Input**:
   * Current `policy.rs` skips any video whose audio is not already `aac` instead of transcoding non-AAC audio to `-c:a aac -b:a 128k` while stream-copying existing `aac` (`-c:a copy`).
   * Furthermore, `yalashbab.com/video/` offers a frictionless **Single-File / Drag-and-Drop Smart Compress** flow (`MP4 / MOV`), whereas `mar2i` currently forces the user to pick two separate directories before doing anything.

---

## 3. Execution Phases & Engineering Requirements

Follow a disciplined **Loop & Context Engineering** workflow. Track your work in `docs/gtd/YALASHBAB_PARITY_SPEC.md` and execute the following 5 phases sequentially, running `cargo test --manifest-path core/Cargo.toml`, `bun test`, and `bunx tsc --noEmit` after every phase.

### Phase 1: Live Web Reconnaissance & Spec Ledger (`docs/gtd/YALASHBAB_PARITY_SPEC.md`)
1. Fetch and inspect `https://yalashbab.com/video/` (HTML DOM, inline `<script>` blocks, linked `.js`/`.css` assets, and form/API endpoints if accessible from your local environment).
2. Document in `docs/gtd/YALASHBAB_PARITY_SPEC.md`:
   * The exact UI flow: Dropzone (`MP4 / MOV`, up to `500 MB` in web, unlimited/terabyte-scale locally in `mar2i`), instant file metadata preview, one-click compression trigger, real-time progress bar, and before/after size & percentage savings summary card.
   * The exact bitstream parameters proven by the forensic capture (`Lavf62` / `libx264`, `-preset fast`, `rc=2pass`, `mbtree=1`, `rc_lookahead=30`, `-pix_fmt yuv420p`, `-c:a copy` for AAC / `-c:a aac -b:a 128k` fallback, `-movflags +faststart`).

### Phase 2: Upgrade `mar2i-core` Policy & Rate-Control Engine (`core/src/policy.rs` & `core/src/profiles.rs`)
Extend `mar2i-core` so it supports **both** existing institutional CRF/Archival modes **and** the new **`SmartSizeReduction` (YalaShbab 2-Pass ABR)** strategy without breaking existing unit tests:

1. **Add `RateControl` & `AudioPlan` to `core/src/policy.rs`**:
   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
   pub enum RateControl {
       /// Traditional constant quality factor (used by existing H.264/HEVC/AV1/VP9 presets)
       Crf(u8),
       /// YalaShbab 2-Pass Target Bitrate (in kbps) with a minimum SSIM floor (in basis points, e.g. 9450 = 0.945)
       TwoPassAbr {
           target_kbps: u32,
           preset: &'static str, // "fast"
           min_ssim_bps: u16,    // 9450 (0.9450)
       },
   }

   #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
   pub enum AudioPlan {
       Copy,
       TranscodeAac { bitrate_kbps: u32 },
   }
   ```
2. **Implement the Smart Target-Bitrate Calculator (`compute_yalashbab_target_kbps`) in `core/src/policy.rs`**:
   * Given `input_bytes: u64`, `duration_sec: f64`, `audio_bitrate_bps: u64`, `width: u32`, `height: u32`, `fps: f64`, and `target_ratio: f64` (default `0.50` for 50% size reduction):
     1. Compute `total_target_kbps = ((input_bytes as f64 * 8.0 * target_ratio) / (duration_sec * 1000.0))`.
     2. Compute `audio_kbps = (audio_bitrate_bps as f64 / 1000.0).clamp(64.0, 320.0)` (if `audio_count == 0`, `0.0`).
     3. Subtract `audio_kbps` and container overhead (`18.5 kbps`), then round to the nearest `50 kbps` step:
        `raw_video_kbps = ((total_target_kbps - audio_kbps - 18.5) / 50.0).round() * 50.0`.
     4. **Anti-Starvation BPP Guard (Solving the Low-Bitrate Edge Case)**:
        Calculate the minimum safe bitrate floor so already-compressed WhatsApp/social videos never turn into macroblock soup:
        $$\text{min\_safe\_kbps} = \frac{\text{width} \times \text{height} \times \text{fps} \times 0.032}{1000.0}$$
        (For `1920x1080 @ 30fps`, `min_safe_kbps` is `~1,990 kbps`; `sample-30s.mp4` targets `2,700 kbps`, which is safely above the floor and produces the exact `2700k` result from `yalashbab.com/video/`!).
     5. If the source video bitrate is already at or below `min_safe_kbps`, return `Decision::Copy { reason: "already at minimum safe perceptual bitrate floor".into() }`.
3. **Add `CodecPreference::SmartAbr` (or `MinistryProfile` ID `"smart_half_size"` / `"الضغط الذكي لتقليص الحجم 50% (مطابق يلا شباب)"`)**:
   * In `core/src/profiles.rs`, register the new profile:
     * `id`: `"smart_half_size"`
     * `name_ar`: `"الضغط الذكي لتقليص الحجم 50% (2-Pass ABR)"`
     * `name_en`: `"Smart 50% Size Reduction (2-Pass ABR)"`
     * `codec`: `"h264_2pass"`
     * `container`: `"mp4"`
   * Ensure all existing tests in `core/tests/policy.rs` continue to pass 100% by keeping `CodecPreference::Auto` and `CodecPreference::H264` behavior backward-compatible while routing `CodecPreference::SmartAbr` (or the new mode/preset) through the 2-Pass ABR planner.

### Phase 3: Implement Atomic 2-Pass `libx264` Execution in `core/src/runner.rs`
In `core/src/runner.rs`, add execution support for the 2-Pass ABR `libx264` plan:
1. **Isolated Passlogfile Directory**:
   * Create a temporary directory inside `parent` (or system temp) using `tempfile::Builder::new().prefix(".mar2i-passlog-").tempdir_in(parent)?` so concurrent workers in `worker.rs` never collide on `ffmpeg2pass-0.log` or `ffmpeg2pass-0.log.mbtree`!
   * Pass `-passlogfile <temp_pass_dir>/ffmpeg2pass` to both Pass 1 and Pass 2 commands.
2. **Pass 1 Command (Analysis & MB-Tree Construction)**:
   ```rust
   // Null-mux output on Pass 1; explicit argv, zero shell invocation
   command_pass1
       .args(["-hide_banner", "-nostdin", "-v", "error", "-y", "-i"])
       .arg(source)
       .args([
           "-map", "0:v:0",
           "-c:v", "libx264",
           "-preset", "fast",
           "-b:v", &format!("{target_kbps}k"),
           "-pix_fmt", "yuv420p",
           "-fps_mode", "passthrough",
           "-threads", &threads.to_string(),
           "-pass", "1",
           "-passlogfile",
       ])
       .arg(&passlog_prefix)
       .args(["-an", "-f", "mp4"])
       .arg(if cfg!(windows) { "NUL" } else { "/dev/null" });
   ```
3. **Pass 2 Command (Final Bitstream Encoding + Audio Copy + Faststart)**:
   ```rust
   command_pass2
       .args(["-hide_banner", "-nostdin", "-v", "error", "-y", "-i"])
       .arg(source)
       .args([
           "-map", "0:v:0",
           "-map", "0:a?",
           "-map_metadata", "0",
           "-map_chapters", "0",
           "-c:v", "libx264",
           "-preset", "fast",
           "-b:v", &format!("{target_kbps}k"),
           "-pix_fmt", "yuv420p",
           "-fps_mode", "passthrough",
           "-threads", &threads.to_string(),
           "-pass", "2",
           "-passlogfile",
       ])
       .arg(&passlog_prefix)
       .args(match audio_plan {
           AudioPlan::Copy => vec!["-c:a", "copy"],
           AudioPlan::TranscodeAac { .. } => vec!["-c:a", "aac", "-b:a", "128k"],
       })
       .args(["-movflags", "+faststart"])
       .arg(temporary);
   ```
4. **Verification Pipeline**:
   * Run `verify_decode(ffmpeg, temporary, control)?`
   * Run `verify_stream_contract(&input, &accepted, options.mode)?` (ensuring `1920x1080` resolution, `30.00 fps` frame rate, duration, and audio stream count match 100%).
   * Measure `SSIM` and `PSNR` (extend `ssim_score` or add `measure_quality_metrics` to parse both `SSIM All:` and `PSNR average:` in a single FFmpeg filtergraph pass: `[0:v:0]setpts=PTS-STARTPTS,format=yuv420p[ref];[1:v:0]setpts=PTS-STARTPTS,format=yuv420p[test];[ref][test]ssim;[0:v:0][1:v:0]psnr`).
   * Enforce `score >= min_ssim` (`0.945` for Smart 50% ABR, `0.965` for standard CRF publishing) and `after < input.input_bytes`.
   * Publish atomically via `tempfile.publish(&destination)?` and clean up the temporary passlog directory automatically on drop.

### Phase 4: Expose Single-File & Before/After Technical Snapshot in Tauri IPC (`src-tauri/src/lib.rs`)
To mimic the effortless UX of `yalashbab.com/video/` alongside `mar2i`'s bulk folder mode:
1. Add a new Tauri command `compress_single_video` (and `inspect_media_file`) in `src-tauri/src/lib.rs`:
   * Accepts `source_path: String`, `output_dir: Option<String>`, `target_ratio: Option<f64>` (default `0.50`, or slider options `30% / 50% / 70%`).
   * If `output_dir` is `None`, safely writes the output next to the original file as `<stem>.compressed.mp4` (never overwriting an existing file, and never modifying the source file).
   * Returns a structured `CompressionSnapshotReport` containing **both** `before:InspectedMedia` and `after:InspectedMedia`, exact byte sizes, percentage saved, `target_kbps`, `ssim`, `psnr`, `audio_md5_preserved: bool`, and `elapsed_ms`.
2. Update `parse_codec` in `src-tauri/src/lib.rs` to accept `"smart_abr"` / `"yalashbab"` so batch runs also support the 2-Pass ABR engine across entire folders of raw/camera footage.

### Phase 5: Frontend UI & Technical Specs Snapshot Card (`src/main.ts`, `src/native.ts`, `src/data.ts`)
1. **Add the "الضغط الذكي (تقليص الحجم 50% - 2-Pass ABR)" Option**:
   * Add `'smart_abr'` to `CodecId` in `src/data.ts` and make it selectable in the main screen (`renderNew`) and options screen (`renderOptions`).
2. **Add Quick Single-File Dropzone & Before/After Specs Snapshot View**:
   * On the "دفعة جديدة" (`renderNew`) screen, allow the operator to either:
     * **Pick a single video file (`MP4 / MOV / MKV`)** (mimicking `yalashbab.com/video/`'s one-click flow) OR **Pick an entire folder** for bulk processing.
   * When a compression job completes, allow clicking **"بطاقة المقارنة الفنية (Before / After Snapshot)"** (or opening `ForensicProbeDrawer`) to display the real side-by-side technical telemetry pulled from `ffprobe`:
     * File Size (Before vs. After + `% Reduced`)
     * Resolution (`width x height`) & FPS (`frame_rate`)
     * Video Bitrate (`kbps` Before vs. After) & Rate Control (`libx264 2-Pass ABR`)
     * Audio Stream (`AAC LC` Stream Copy verification)
     * Measured `SSIM` & `PSNR` badges
3. **Preserve `native-honesty.test.ts` Compliance**:
   * Never inject fake metrics when `isDesktop` is true and no batch/file has been processed. Only render measured native `ffprobe` / `SSIM` numbers after real execution.

---

## 4. Mandatory Verification & Acceptance Gates (Definition of Done)

You are **NOT** done until all of the following commands execute with zero errors and zero warnings:

1. **New Integration Test (`core/tests/yalashbab_abr.rs`)**:
   * Write a deterministic integration test in `core/tests/yalashbab_abr.rs` that generates a 5-second `1920x1080 @ 30fps` H.264 + AAC MP4 file with a video bitrate around `5,500 kbps` (`BPP ~ 0.089`, which previously got skipped by `BPP <= 0.14`).
   * Run `run_one_with_options` using `CodecPreference::SmartAbr`.
   * Assert that:
     * The outcome is `RunOutcome::Accepted` (NOT `Copied` or `Skipped`).
     * Output size is approximately `50%` of input size (`after <= before * 58 / 100` and `after >= before * 40 / 100`).
     * Output resolution is `1920x1080` and frame rate is `30.0 fps`.
     * Audio stream is copied bit-for-bit without re-encoding.
     * The `moov` atom precedes `mdat` (`+faststart`).
     * No `.mar2i-passlog-*` or `.mar2i-partial-*` files remain in the output directory after completion.
2. **Full Test Suite & Linter Verification**:
   ```bash
   cargo test --manifest-path core/Cargo.toml
   cargo clippy --manifest-path core/Cargo.toml -- -D warnings
   bun test
   bunx tsc --noEmit
   ```
3. **Benchmark Reproduction on `sample-30s.mp4` (if present)**:
   * Run the CLI `cargo run --manifest-path core/Cargo.toml --bin mar2i -- ...` on `sample-30s.mp4` and verify the output matches `yalashbab.com/video/` (`~10.29 MB`, `1920x1080`, `30 fps`, `~2700 kbps`, `SSIM >= 0.958`).
