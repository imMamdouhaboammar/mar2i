# Third-Party Software Notices and Licenses

Mar2i (مرئي) utilizes and interacts with open-source software libraries and multimedia utilities. This document outlines the third-party components, their respective licenses, and redistribution obligations.

---

## 1. FFmpeg and FFprobe

- **Project:** FFmpeg (https://ffmpeg.org)
- **Copyright:** (c) 2000-2026 the FFmpeg developers
- **License Options:**
  - **LGPL v2.1 or later**: When compiled without `--enable-gpl` and without GPL-only external libraries.
  - **GPL v2 or later / GPL v3**: When compiled with `--enable-gpl` and linked with GPL libraries (such as `libx264` or `libx265`).

### Licensing & Redistribution Guidelines for Mar2i Packages:

1. **Process Boundary Separation:**
   - In Mar2i, FFmpeg and FFprobe run as distinct standalone subprocesses (invoked via standard OS process execution APIs with pipes, not dynamically linked into the Tauri process memory space).
   - The desktop application communicates exclusively via standard CLI arguments and standard streams (stdout/stderr).

2. **GPL v3 Compliance Checklist (when bundling GPL-licensed builds):**
   - Provide complete, corresponding machine-readable source code for the exact FFmpeg build used, including all patches, configure scripts, and build instructions.
   - Accompany binary distributions with a prominent notice stating that the package includes software licensed under the GNU General Public License.
   - Include the full text of `COPYING.GPLv3`.

3. **LGPL Alternative (Permissive Codec Path):**
   - If an LGPL-only distribution is required, FFmpeg must be built with `--enable-version3` (without `--enable-gpl`).
   - Software encoding using `libx264` and `libx265` must be omitted in LGPL builds; instead, use hardware-accelerated platform encoders:
     - **macOS:** VideoToolbox (`h264_videotoolbox`, `hevc_videotoolbox`)
     - **Windows:** MediaFoundation (`h264_mf`, `hevc_mf`)
     - **AV1:** `libsvtav1` (BSD-3-Clause)
     - **VP9:** `libvpx` (BSD-3-Clause)

---

## 2. Rust Crates & Frontend Dependencies

All direct Rust dependencies used in `mar2i-core` and `src-tauri` are distributed under standard permissive open-source licenses:
- **MIT License** / **Apache 2.0**: Rusqlite, Serde, Tokio, Tempfile, Sha2, Anyhow, Sysinfo, Tauri v2.
- **Frontend Assets**: Lucide React (ISC), Tailwind CSS (MIT), Vite (MIT).
