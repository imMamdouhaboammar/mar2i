# Mar2i Smart 2-Pass ABR (50% Target Size Reduction) Specification

## 1. Executive Summary & Purpose
This document specifies the technical design, rate-control parameters, mathematical bitrate allocation formula, and integration architecture for the **Smart 2-Pass ABR (`smart_abr`)** compression engine in `mar2i` (`مرئي`). It documents the two-pass encoding lifecycle, audio preservation contract, streaming atom placement (`-movflags +faststart`), and integration points across `mar2i-core`, `src-tauri`, and the UI.

## 2. Technical Bitstream Specification

| Parameter | Baseline Input | Smart ABR Output (`smart_abr`) | Design Invariant |
| :--- | :--- | :--- | :--- |
| **File Size Reduction** | Source baseline (100%) | ~50% target reduction | **Predictable offline archival & distribution** |
| **Container & Atoms** | MP4 / MOV / MKV | MP4 (`isom/iso2/avc1/mp41`), `moov` at file start | **`-movflags +faststart`** (instant streaming playback) |
| **Resolution & SAR** | Source resolution (e.g. 1920x1080) | 1:1 Preserved (1920x1080) | **100% spatial preservation, zero downscaling** |
| **Frame Rate** | Source frame rate (e.g. 30.00 fps) | 1:1 Preserved (30.00 fps) | **Zero frame drops (`-fps_mode passthrough`)** |
| **Video Codec** | High bitrate H.264 / ProRes / Mezzanine | H.264 / AVC (`High@L4.0`, `yuv420p`, 8-bit) | Universal hardware & browser playback |
| **Rate Control** | Variable / Constant Quality | Deterministic 2-Pass ABR (`libx264 -preset fast`) | Macroblock-tree lookahead (`mbtree=1`, `rc_lookahead=30`) |
| **Audio Stream** | AAC LC / Linear PCM / AC3 | AAC LC stream-copied when compatible | **`-c:a copy`** for AAC; transcode fallback for non-AAC |
| **Quality Floor** | Reference (1.0000) | **SSIM $\ge 0.945$** | Verified perceptual fidelity with anti-starvation BPP floor |

## 3. Mathematical Formula for 50% Target Bitrate
Given:
- $S_{in}$: Input size in bytes
- $D$: Duration in seconds
- $B_a$: Audio bitrate in bps
- $R_{target}$: Target reduction ratio ($0.50$)

$$B_{total\_target} = \frac{S_{in} \times 8 \times R_{target}}{D \times 1000} \text{ (in kbps)}$$

$$B_{video\_target} = \text{round\_to\_50}(B_{total\_target} - B_a - 18.5)$$

### Anti-Starvation Guard:
$$\text{min\_safe\_kbps} = \frac{\text{width} \times \text{height} \times \text{fps} \times 0.032}{1000.0}$$
If the calculated target or the source bitrate is already below $\text{min\_safe\_kbps}$, the engine avoids destructive macroblocking by copying the stream or bounding the rate.

## 4. Execution Lifecycle
1. **Pass 1 (Analysis & Motion Modeling)**:
   - Output redirected to null sink (`/dev/null` on Unix, `NUL` on Windows).
   - Writes passlog to an isolated temporary directory to prevent concurrent collision.
2. **Pass 2 (Final Bitstream Assembly)**:
   - Reads passlog to allocate bits dynamically.
   - Copies audio bitstream directly when format is AAC.
   - Applies `-movflags +faststart` to place index atoms (`moov`) before payload (`mdat`).
3. **Verification**:
   - Decodability check via FFmpeg.
   - SSIM metric calculated against original source.
   - Atomic publication to destination directory.
