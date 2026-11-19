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

## Automated Packaging Tool (`tools/package-offline.ts`)

An automated packaging script fetches, cryptographically verifies, and stages the target platform's media binaries into `src-tauri/bin/`:

```bash
# Stage Apple Silicon macOS binaries with SHA-256 validation
bun run tools/package-offline.ts --target=macos-arm64

# Stage Windows x86-64 binaries with SHA-256 validation
bun run tools/package-offline.ts --target=windows-x64

# Verify current staging and manifest
bun run tools/package-offline.ts --verify
```

---

## Bundling Steps for Production Releases

### 1. Place or Stage Binaries into `src-tauri/bin/`

Use the automated tool or place platform-specific binaries manually:

- **For macOS Apple Silicon (`aarch64-apple-darwin`):**
  ```bash
  bun run tools/package-offline.ts --target=macos-arm64
  ```

- **For Windows (`x86_64-pc-windows-msvc`):**
  ```bash
  bun run tools/package-offline.ts --target=windows-x64
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

---

## Automated CI/CD Release Pipeline

The GitHub Actions workflow at `.github/workflows/release.yml` automates cross-platform release builds:
- Triggers on version tag push (`v*`) or manual `workflow_dispatch`.
- Builds standalone `.dmg` for macOS Apple Silicon and `.exe` (NSIS) / `.msi` for Windows x86-64.
- Generates `SHA256SUMS.txt` cryptographic ledger and publishes artifacts.
- Automatically executes clean ad-hoc signing (`codesign --force --deep --sign -`) on nested multimedia sidecars and the main bundle before packaging the DMG.
- Automatically triggers `scripts/update-homebrew-cask.sh` to update the Cask formula and push to the Homebrew tap repository.

---

## macOS Zero-Cost Distribution Channels

Because Apple Developer ID certificates ($99/yr) and notarization are not yet configured for open-source distribution, standard browser downloads of `.dmg` files get flagged with `com.apple.quarantine`, causing Gatekeeper to block opening.

Mar2i provides two zero-cost, automated distribution channels that eliminate Gatekeeper quarantine blocks for end users:

### 1. One-Line Terminal Installer (`scripts/install-macos.sh`)

Users can install or update Mar2i via a single terminal command:

```bash
curl -fsSL https://raw.githubusercontent.com/imMamdouhaboammar/mar2i/main/scripts/install-macos.sh | bash
```

**Installer Execution Flow:**
1. **Preflight**: Verifies macOS operating system and detects CPU architecture (`arm64` -> Apple Silicon, `x86_64` -> Intel).
2. **Resolution**: Queries GitHub Releases API for the latest release (or `MAR2I_VERSION` if specified).
3. **Integrity Verification**: Downloads `SHA256SUMS.txt` from the release and verifies the downloaded DMG's SHA-256 hash.
4. **Clean Installation**: Mounts the DMG quietly via `hdiutil`, uses `ditto` to copy `Mar2i.app` to `/Applications` (preserving permissions and bundle structure), and unmounts.
5. **Gatekeeper Sanitation**: Runs `xattr -cr /Applications/Mar2i.app` defensively to guarantee zero quarantine flags exist.
6. **Code Signature Check**: Runs `codesign --verify --deep --strict /Applications/Mar2i.app` and outputs launch instructions (`open -a "Mar2i"`).

### 2. Homebrew Cask Distribution (`packaging/homebrew/Casks/mar2i.rb`)

Users can tap and install via Homebrew with `--no-quarantine`:

```bash
brew tap imMamdouhaboammar/mar2i
brew install --cask --no-quarantine mar2i
```

#### Verification Command
To verify that zero quarantine attributes are attached to the installed application:

```bash
xattr -l /Applications/Mar2i.app
```
If this command produces no output, no quarantine attributes exist and the app will open immediately.

---

## Homebrew Tap Setup Guide for Repository Owner

To enable `brew tap imMamdouhaboammar/mar2i`, complete the following one-time manual setup steps on GitHub:

1. **Create the Tap Repository on GitHub:**
   - Go to GitHub -> **New Repository**.
   - Repository name: **`homebrew-mar2i`** (Homebrew requires the `homebrew-` prefix for custom taps, which maps to `brew tap imMamdouhaboammar/mar2i`).
   - Visibility: **Public**.
   - Initialize with a `README.md`.

2. **Generate a GitHub Personal Access Token (PAT):**
   - Go to **GitHub Settings -> Developer Settings -> Personal access tokens (fine-grained or classic)**.
   - For Fine-Grained: Grant **Read & Write** access to `imMamdouhaboammar/homebrew-mar2i` contents.
   - For Classic: Select the `repo` scope.

3. **Add Secret to `mar2i` Repository:**
   - In `imMamdouhaboammar/mar2i` repository, navigate to **Settings -> Secrets and variables -> Actions**.
   - Click **New repository secret**.
   - Name: **`HOMEBREW_TAP_TOKEN`**
   - Value: `<your-generated-github-token>`

4. **Automated Synchronization:**
   - Whenever a new release is published via `.github/workflows/release.yml`, the `update-homebrew-cask` job will automatically invoke `./scripts/update-homebrew-cask.sh`, calculate new SHA-256 hashes, update `Casks/mar2i.rb`, and push directly to `imMamdouhaboammar/homebrew-mar2i`.
   - If the secret is not configured, the release workflow continues without failing.
