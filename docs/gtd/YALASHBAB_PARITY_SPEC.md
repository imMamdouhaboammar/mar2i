# YalaShbab Video Compression Parity Specification

## 1. Executive Summary & Purpose
This document specifies the forensic reverse-engineering findings of `https://yalashbab.com/video/` and defines the exact architectural parity blueprint for `mar2i` (`مرئي`). It documents the UI interaction flow, bitstream telemetry, 2-Pass ABR calculation formula, rate-control parameters, and integration points across `mar2i-core`, `src-tauri`, and the Vanilla TypeScript frontend.

## 2. Forensic Bitstream Telemetry Comparison

| Parameter | Original Input (`sample-30s.mp4`) | YalaShbab Output (`compressed.mp4`) | Mar2i Target Engine Parity |
| :--- | :--- | :--- | :--- |
| **File Size** | 21,657,943 bytes (20.65 MB) | 10,784,658 bytes (10.29 MB) | **-50.20% exact size reduction** |
| **Container & Atoms** | MP4 (`isom/avc1/mp41`) | MP4 (`isom/iso2/avc1/mp41`), `moov` at byte offset 32 | **`-movflags +faststart`** |
| **Resolution & SAR** | 1920x1080 (SAR 1:1, DAR 16:9) | 1920x1080 (SAR 1:1, DAR 16:9) | **100% spatial preservation** |
| **Frame Rate** | 30.00 fps (911 frames, 30.366s) | 30.00 fps (911 frames, 30.366s) | **`-fps_mode passthrough`** |
| **Video Codec** | H.264 / AVC (`High@L4.0`, `yuv420p`, 8-bit) | H.264 / AVC (`High@L4.0`, `yuv420p`, 8-bit) | Preserves `High@L4.0`, `yuv420p` |
| **Video Bitrate** | 5,569,685 bps (~5,570 kbps) | 2,705,169 bps (~2,705 kbps) | **-51.43% video bitrate reduction** |
| **Total Mux Bitrate**| 5,687,298 bps (~5,687 kbps) | 2,834,245 bps (~2,834 kbps) | **-50.17% total bitrate reduction** |
| **Audio Stream** | AAC LC, 44,100 Hz, Stereo, 126,541 bps | AAC LC, 44,100 Hz, Stereo, 126,541 bps | **`-c:a copy`** (MD5: `eb4430571a8c11b9a86ede01126a44ea`) |
| **Quality Score** | Reference (1.0000) | **SSIM = 0.958814**, **PSNR = 33.41 dB** | High perceptual fidelity on complex foliage |

## 3. Extracted `libx264` SEI Signature & Parameters
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
- **Preset**: `-preset fast` (indicated by `ref=2`, `subme=6`, `rc_lookahead=30`, `weightp=1`, `me=hex`, `trellis=1`).
- **Rate Control**: `rc=2pass`, `mbtree=1`, `bitrate=2700k`.
- **Multipass Lifecycle**: Pass 1 analysis (`-an -f mp4 /dev/null`) + Pass 2 encode (`-c:a copy -movflags +faststart`).

## 4. Mathematical Formula for 50% Target Bitrate
Given:
- $S_{in}$: Input size in bytes ($21,657,943$)
- $D$: Duration in seconds ($30.441$)
- $B_a$: Audio bitrate in bps ($126,541$)
- $R_{target}$: Target reduction ratio ($0.50$)

$$B_{total\_target} = \frac{S_{in} \times 8 \times R_{target}}{D \times 1000} \approx 2,845.9 \text{ kbps}$$

$$B_{video\_target} = \text{round\_to\_50}(B_{total\_target} - B_a - 18.5) = \text{round\_to\_50}(2845.9 - 126.5 - 18.5) = \mathbf{2,700 \text{ kbps}}$$

### Anti-Starvation Guard:
$$\text{min\_safe\_kbps} = \frac{\text{width} \times \text{height} \times \text{fps} \times 0.032}{1000.0}$$
For $1920 \times 1080 @ 30\text{ fps}$, $\text{min\_safe\_kbps} \approx 1,990.6 \text{ kbps}$.
$2,700 \text{ kbps} > 1,990.6 \text{ kbps}$, so the target rate is safe and avoids blockiness.

## 5. Implementation Roadmap
1. `core/src/policy.rs`: Add `RateControl`, `AudioPlan`, `compute_yalashbab_target_kbps`, and `CodecPreference::SmartAbr`.
2. `core/src/profiles.rs`: Register `smart_half_size` ministry profile.
3. `core/src/runner.rs`: Implement 2-pass `libx264` execution with isolated passlog tempdir, SSIM floor `0.945` for Smart ABR, and PSNR collection.
4. `src-tauri/src/lib.rs`: Expose `compress_single_video` and `CompressionSnapshotReport`. Update `parse_codec`.
5. `src/`: Update `src/data.ts`, `src/native.ts`, `src/main.ts` to support single-file selection and Before/After Technical Snapshot card.
6. Verify against `core/tests/yalashbab_abr.rs` and all 39+ project tests.
