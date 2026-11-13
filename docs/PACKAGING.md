# Mar2i Offline Multimedia Packaging Guide

This guide details how to bundle verified, audited FFmpeg and FFprobe binaries for fully offline Mar2i installations.

---

## Architecture & Binary Discovery

`Mar2i` discovers the required `ffmpeg` and `ffprobe` binaries in the following deterministic order of precedence (implemented in `src-tauri/src/lib.rs::resolve_media_binary`):

1. **Environment Variables (Runtime Override):**
   - `MAR2I_FFMPEG_PATH`: Explicit path to `ffmpeg` executable.
   - `MAR2I_FFPROBE_PATH`: Explicit path to `ffprobe` executable.
   - `MAR2I_MEDIA_DIR`: Directory containing `ffmpeg` and `ffprobe`.

2. **Application Bundle Resources (Primary Offline Distribution):**
   - **macOS (`.app` bundle):**
     - `Mar2i.app/Contents/Resources/bin/ffmpeg`
     - `Mar2i.app/Contents/Resources/bin/ffprobe`
     - `Mar2i.app/Contents/Resources/ffmpeg`
     - `Mar2i.app/Contents/Resources/ffprobe`
   - **Windows / Linux:**
     - `<exe_dir>/bin/ffmpeg.exe`
     - `<exe_dir>/bin/ffprobe.exe`
     - `<exe_dir>/ffmpeg.exe`
     - `<exe_dir>/ffprobe.exe`

3. **Local Developer System Paths (Development / Fallback):**
   - `/opt/homebrew/bin/ffmpeg`
   - `/usr/local/bin/ffmpeg`
   - `/usr/bin/ffmpeg`

4. **System PATH (Global Fallback).**

---

## Bundling Steps for Production Releases

### 1. Place Binaries into `src-tauri/bin/`

Before running `tauri build`, place the platform-specific binaries into `src-tauri/bin/`:

- **For macOS Apple Silicon (`aarch64-apple-darwin`):**
  ```bash
  cp /path/to/audited/ffmpeg src-tauri/bin/ffmpeg
  cp /path/to/audited/ffprobe src-tauri/bin/ffprobe
  chmod +x src-tauri/bin/ffmpeg src-tauri/bin/ffprobe
  ```

- **For Windows (`x86_64-pc-windows-msvc`):**
  ```powershell
  copy C:\path\to\ffmpeg.exe src-tauri\bin\ffmpeg.exe
  copy C:\path\to\ffprobe.exe src-tauri\bin\ffprobe.exe
  ```

### 2. Verify Inclusion in `tauri.conf.json`

Ensure `src-tauri/tauri.conf.json` includes:
```json
"bundle": {
  "active": true,
  "targets": "all",
  "resources": [
    "bin/*"
  ]
}
```

### 3. Build Desktop Application

```bash
bun run build
bunx tauri build
```

The resulting package will bundle the multimedia binaries into the application's resources directory, enabling instant, fully offline operation without requiring Homebrew or system PATH tools.
