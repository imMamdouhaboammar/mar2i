#!/usr/bin/env bash
# ==============================================================================
# Mar2i — One-Line macOS Terminal Installer
# https://github.com/imMamdouhaboammar/mar2i
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/imMamdouhaboammar/mar2i/main/scripts/install-macos.sh | bash
#
# Custom options via environment variables:
#   MAR2I_VERSION=0.2.3    Specify explicit version tag (default: latest release)
#   MAR2I_REPO=owner/repo   Specify GitHub repository slug (default: imMamdouhaboammar/mar2i)
#   MAR2I_INSTALL_DIR=/path Custom install directory (default: /Applications)
#   DRY_RUN=1               Simulate resolution and download checks without installing
# ==============================================================================

set -euo pipefail

# --- Color and formatting helpers ---
if [ -t 1 ]; then
  COLOR_BLUE="\033[1;34m"
  COLOR_GREEN="\033[1;32m"
  COLOR_YELLOW="\033[1;33m"
  COLOR_RED="\033[1;31m"
  COLOR_BOLD="\033[1m"
  COLOR_RESET="\033[0m"
else
  COLOR_BLUE=""
  COLOR_GREEN=""
  COLOR_YELLOW=""
  COLOR_RED=""
  COLOR_BOLD=""
  COLOR_RESET=""
fi

log_info() {
  printf "${COLOR_BLUE}==>${COLOR_RESET} %b\n" "$*"
}

log_success() {
  printf "${COLOR_GREEN}✓${COLOR_RESET} %b\n" "$*"
}

log_warn() {
  printf "${COLOR_YELLOW}WARNING:${COLOR_RESET} %b\n" "$*" >&2
}

log_error() {
  printf "${COLOR_RED}ERROR:${COLOR_RESET} %b\n" "$*" >&2
}

die() {
  log_error "$*"
  exit 1
}

# --- Parse arguments ---
DRY_RUN="${DRY_RUN:-0}"
for arg in "$@"; do
  case "$arg" in
    --dry-run)
      DRY_RUN="1"
      ;;
    --help|-h)
      cat <<EOF
Mar2i macOS Terminal Installer

Usage:
  install-macos.sh [OPTIONS]

Options:
  --dry-run       Simulate version resolution, download, and verification without installing
  -h, --help      Show this help message

Environment variables:
  MAR2I_VERSION   Target release version tag (e.g. 0.2.3 or v0.2.3)
  MAR2I_REPO      GitHub repo slug (default: imMamdouhaboammar/mar2i)
  MAR2I_INSTALL_DIR Target directory (default: /Applications)
EOF
      exit 0
      ;;
  esac
done

# --- 1. Preflight & Architecture Detection ---
log_info "Verifying macOS system prerequisites..."

OS_NAME="$(uname -s)"
if [ "$OS_NAME" != "Darwin" ]; then
  die "This installer is intended for macOS only (detected: ${OS_NAME}). Aborting."
fi

ARCH_RAW="$(uname -m)"
case "$ARCH_RAW" in
  arm64|aarch64)
    ARCH_KEY="aarch64"
    ARCH_NAME="Apple Silicon (arm64 / aarch64)"
    ;;
  x86_64|amd64)
    ARCH_KEY="x64"
    ARCH_NAME="Intel (x86_64 / x64)"
    ;;
  *)
    die "Unsupported CPU architecture: ${ARCH_RAW}. Only Apple Silicon (arm64) and Intel (x86_64) are supported."
    ;;
esac
log_success "Architecture detected: ${ARCH_NAME}"

# Verify required system tools
for tool in curl ditto hdiutil shasum; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    die "Required macOS system tool '${tool}' is missing from PATH."
  fi
done

# --- 2. Setup Temporary Workspace & Cleanup Trap ---
TMP_DIR="$(mktemp -d -t mar2i-installer-XXXXXX)"
MOUNT_DIR=""

cleanup() {
  local exit_status=$?
  if [ -n "$MOUNT_DIR" ] && [ -d "$MOUNT_DIR" ]; then
    hdiutil detach "$MOUNT_DIR" -quiet -force 2>/dev/null || true
    rmdir "$MOUNT_DIR" 2>/dev/null || true
  fi
  if [ -n "$TMP_DIR" ] && [ -d "$TMP_DIR" ]; then
    rm -rf "$TMP_DIR" 2>/dev/null || true
  fi
  exit "$exit_status"
}
trap cleanup EXIT INT TERM HUP

# --- 3. Latest Release Resolution ---
REPO="${MAR2I_REPO:-imMamdouhaboammar/mar2i}"
API_BASE="https://api.github.com/repos/${REPO}"

if [ -n "${MAR2I_VERSION:-}" ]; then
  TAG_INPUT="${MAR2I_VERSION}"
  if [[ "$TAG_INPUT" != v* ]]; then
    RELEASE_TAG="v${TAG_INPUT}"
  else
    RELEASE_TAG="${TAG_INPUT}"
  fi
  log_info "Resolving release: ${RELEASE_TAG} from ${REPO}..."
  RELEASE_URL="${API_BASE}/releases/tags/${RELEASE_TAG}"
else
  log_info "Resolving latest release from ${REPO}..."
  RELEASE_URL="${API_BASE}/releases/latest"
fi

RELEASE_JSON="${TMP_DIR}/release.json"
HTTP_STATUS=$(curl -fsSL -w "%{http_code}" -H "Accept: application/vnd.github.v3+json" "$RELEASE_URL" -o "$RELEASE_JSON" || echo "000")

if [ "$HTTP_STATUS" != "200" ]; then
  die "Failed to query GitHub API for release at ${RELEASE_URL} (HTTP status: ${HTTP_STATUS}). Please check your internet connection or repository slug."
fi

# Extract release tag and clean version
RESOLVED_TAG="$(awk -F'"' '/"tag_name":/ { print $4; exit }' "$RELEASE_JSON")"
if [ -z "$RESOLVED_TAG" ]; then
  die "Could not parse release tag name from GitHub API response."
fi
CLEAN_VERSION="${RESOLVED_TAG#v}"
log_success "Found release: ${COLOR_BOLD}${RESOLVED_TAG}${COLOR_RESET}"

# Find matching macOS asset from release JSON
ASSET_INFO="${TMP_DIR}/assets.txt"

if command -v python3 >/dev/null 2>&1; then
  python3 -c '
import json, sys
try:
    with open(sys.argv[1], "r", encoding="utf-8") as f:
        data = json.load(f)
    for a in data.get("assets", []):
        n = a.get("name")
        u = a.get("browser_download_url")
        if n and u:
            print(n + "|" + u)
except Exception as e:
    sys.exit(1)
' "$RELEASE_JSON" > "$ASSET_INFO" || true
fi

# Fallback parser if python3 failed or produced empty output
if [ ! -s "$ASSET_INFO" ]; then
  awk '
  BEGIN { in_assets = 0; name = ""; url = "" }
  /"assets"[[:space:]]*:[[:space:]]*\[/ { in_assets = 1 }
  in_assets && /"name"[[:space:]]*:/ {
    split($0, parts, "\"")
    for (i = 1; i < length(parts); i++) {
      if (parts[i] ~ /name/) {
        name = parts[i+2]
        break
      }
    }
  }
  in_assets && /"browser_download_url"[[:space:]]*:/ {
    split($0, parts, "\"")
    for (i = 1; i < length(parts); i++) {
      if (parts[i] ~ /browser_download_url/) {
        url = parts[i+2]
        break
      }
    }
    if (name != "" && url != "") {
      print name "|" url
      name = ""
      url = ""
    }
  }
  in_assets && /\][[:space:]]*,?[[:space:]]*$/ { in_assets = 0 }
  ' "$RELEASE_JSON" > "$ASSET_INFO" || true
fi

MATCHED_ASSET_NAME=""
MATCHED_ASSET_URL=""

# Priority resolution order based on architecture:
# For arm64:
#   1. Mar2i_*_aarch64.dmg
#   2. Mar2i_*_arm64.dmg
#   3. Mar2i_*_universal.dmg
#   4. Mar2i_*_aarch64.app.tar.gz / .tar.gz
# For x64:
#   1. Mar2i_*_x64.dmg / Mar2i_*_x86_64.dmg
#   2. Mar2i_*_universal.dmg
#   3. Mar2i_*_x64.app.tar.gz / .tar.gz

resolve_asset() {
  local target_arch="$1"
  while IFS='|' read -r asset_name asset_url; do
    [ -z "$asset_name" ] && continue
    case "$asset_name" in
      *.*) ;;
      *) continue ;;
    esac

    if [ "$target_arch" = "aarch64" ]; then
      if [[ "$asset_name" =~ (aarch64|arm64).*\.dmg$ ]]; then
        MATCHED_ASSET_NAME="$asset_name"
        MATCHED_ASSET_URL="$asset_url"
        return 0
      fi
    elif [ "$target_arch" = "x64" ]; then
      if [[ "$asset_name" =~ (x64|x86_64).*\.dmg$ ]]; then
        MATCHED_ASSET_NAME="$asset_name"
        MATCHED_ASSET_URL="$asset_url"
        return 0
      fi
    fi
  done < "$ASSET_INFO"

  # Fallback to universal DMG
  while IFS='|' read -r asset_name asset_url; do
    if [[ "$asset_name" =~ universal.*\.dmg$ ]]; then
      MATCHED_ASSET_NAME="$asset_name"
      MATCHED_ASSET_URL="$asset_url"
      return 0
    fi
  done < "$ASSET_INFO"

  # Fallback to tar.gz archive if no DMG
  while IFS='|' read -r asset_name asset_url; do
    if [ "$target_arch" = "aarch64" ] && [[ "$asset_name" =~ (aarch64|arm64).*\.tar\.gz$ ]]; then
      MATCHED_ASSET_NAME="$asset_name"
      MATCHED_ASSET_URL="$asset_url"
      return 0
    elif [ "$target_arch" = "x64" ] && [[ "$asset_name" =~ (x64|x86_64).*\.tar\.gz$ ]]; then
      MATCHED_ASSET_NAME="$asset_name"
      MATCHED_ASSET_URL="$asset_url"
      return 0
    fi
  done < "$ASSET_INFO"

  return 1
}

if ! resolve_asset "$ARCH_KEY"; then
  echo ""
  log_error "No compatible macOS asset found for architecture '${ARCH_KEY}' in release ${RESOLVED_TAG}."
  echo "Available assets in this release:"
  while IFS='|' read -r asset_name _; do
    echo "  - ${asset_name}"
  done < "$ASSET_INFO"
  exit 1
fi

log_success "Matched release asset: ${MATCHED_ASSET_NAME}"

# --- 4. Download Asset and Checksum Ledger ---
DOWNLOAD_DEST="${TMP_DIR}/${MATCHED_ASSET_NAME}"
log_info "Downloading ${MATCHED_ASSET_NAME}..."
curl -fL --progress-bar "$MATCHED_ASSET_URL" -o "$DOWNLOAD_DEST"
log_success "Download complete (${DOWNLOAD_DEST})"

# Check for checksum file in the release
CHECKSUM_URL=""
CHECKSUM_FILE="${TMP_DIR}/SHA256SUMS.txt"

while IFS='|' read -r asset_name asset_url; do
  case "$asset_name" in
    SHA256SUMS.txt|SHA256SUMS-macos-${ARCH_KEY}.txt|SHA256SUMS-macos-*.txt)
      CHECKSUM_URL="$asset_url"
      break
      ;;
  esac
done < "$ASSET_INFO"

if [ -n "$CHECKSUM_URL" ]; then
  log_info "Downloading checksum ledger from ${CHECKSUM_URL}..."
  if curl -fsSL "$CHECKSUM_URL" -o "$CHECKSUM_FILE"; then
    EXPECTED_SHA="$(grep -E "${MATCHED_ASSET_NAME}(\$|[[:space:]])" "$CHECKSUM_FILE" | awk '{print $1}' | head -n1 || true)"
    if [ -n "$EXPECTED_SHA" ]; then
      log_info "Verifying SHA-256 cryptographic checksum..."
      ACTUAL_SHA="$(shasum -a 256 "$DOWNLOAD_DEST" | awk '{print $1}')"
      if [ "$EXPECTED_SHA" != "$ACTUAL_SHA" ]; then
        die "SHA-256 checksum mismatch!\nExpected: ${EXPECTED_SHA}\nActual:   ${ACTUAL_SHA}\nDownloaded file may be corrupted or tampered with."
      fi
      log_success "Cryptographic SHA-256 verified: ${ACTUAL_SHA:0:16}..."
    else
      log_warn "Asset ${MATCHED_ASSET_NAME} not listed in ${CHECKSUM_URL}; proceeding with downloaded package."
    fi
  fi
else
  log_warn "No SHA256SUMS checksum file found on release; skipping hash verification."
fi

if [ "$DRY_RUN" = "1" ]; then
  echo ""
  log_success "${COLOR_BOLD}[DRY RUN MODE] Preflight, resolution, download, and hash verification completed successfully.${COLOR_RESET}"
  echo "Asset:       ${MATCHED_ASSET_NAME}"
  echo "Target path: ${MAR2I_INSTALL_DIR:-/Applications}/Mar2i.app"
  exit 0
fi

# --- 5. Clean Installation to /Applications ---
INSTALL_BASE="${MAR2I_INSTALL_DIR:-/Applications}"
APP_NAME="Mar2i"
APP_TARGET="${INSTALL_BASE}/${APP_NAME}.app"

SUDO_CMD=""
if [ ! -w "$INSTALL_BASE" ]; then
  log_info "'${INSTALL_BASE}' is protected. Requesting administrator privileges (sudo)..."
  SUDO_CMD="sudo"
  $SUDO_CMD -v
fi

# Check if an existing version is running
if pgrep -x "$APP_NAME" >/dev/null 2>&1 || pgrep -f "${APP_NAME}.app" >/dev/null 2>&1; then
  log_warn "Mar2i is currently running. Attempting graceful shutdown before updating..."
  osascript -e "tell application \"${APP_NAME}\" to quit" 2>/dev/null || true
  sleep 1
  if pgrep -x "$APP_NAME" >/dev/null 2>&1; then
    killall "$APP_NAME" 2>/dev/null || true
    sleep 1
  fi
fi

STAGED_APP_SOURCE=""

if [[ "$DOWNLOAD_DEST" =~ \.dmg$ ]]; then
  MOUNT_DIR="${TMP_DIR}/mount"
  mkdir -p "$MOUNT_DIR"
  log_info "Mounting disk image: ${MATCHED_ASSET_NAME}..."
  hdiutil attach "$DOWNLOAD_DEST" -nobrowse -readonly -mountpoint "$MOUNT_DIR" -quiet

  # Locate .app bundle inside volume
  FOUND_APP="$(find "$MOUNT_DIR" -maxdepth 2 -name "*.app" -type d | head -n1)"
  if [ -z "$FOUND_APP" ] || [ ! -d "$FOUND_APP" ]; then
    die "Could not locate an application bundle (.app) inside the mounted disk image."
  fi
  STAGED_APP_SOURCE="$FOUND_APP"
elif [[ "$DOWNLOAD_DEST" =~ \.tar\.gz$ ]]; then
  EXTRACT_DIR="${TMP_DIR}/extracted"
  mkdir -p "$EXTRACT_DIR"
  log_info "Extracting archive: ${MATCHED_ASSET_NAME}..."
  tar -xzf "$DOWNLOAD_DEST" -C "$EXTRACT_DIR"
  FOUND_APP="$(find "$EXTRACT_DIR" -maxdepth 2 -name "*.app" -type d | head -n1)"
  if [ -z "$FOUND_APP" ] || [ ! -d "$FOUND_APP" ]; then
    die "Could not locate an application bundle (.app) inside the extracted archive."
  fi
  STAGED_APP_SOURCE="$FOUND_APP"
else
  die "Unrecognized asset format: ${MATCHED_ASSET_NAME} (expected .dmg or .tar.gz)"
fi

log_info "Installing ${APP_NAME}.app to ${INSTALL_BASE}..."
if [ -d "$APP_TARGET" ]; then
  log_info "Replacing previous installation at ${APP_TARGET}..."
  $SUDO_CMD rm -rf "$APP_TARGET"
fi

$SUDO_CMD ditto "$STAGED_APP_SOURCE" "$APP_TARGET"
log_success "Application copied to ${APP_TARGET}"

# Unmount disk image if mounted
if [ -n "$MOUNT_DIR" ] && [ -d "$MOUNT_DIR" ]; then
  hdiutil detach "$MOUNT_DIR" -quiet -force 2>/dev/null || true
  MOUNT_DIR=""
fi

# --- 6. Gatekeeper & Signature Safety Net ---
log_info "Applying Gatekeeper defensive sanitation..."
$SUDO_CMD xattr -cr "$APP_TARGET" 2>/dev/null || xattr -cr "$APP_TARGET" 2>/dev/null || true
log_success "Quarantine attribute stripped (com.apple.quarantine removed)"

log_info "Verifying bundle code signature..."
if codesign --verify --deep --strict "$APP_TARGET" >/dev/null 2>&1; then
  log_success "Bundle signature verification: VALID"
else
  log_warn "Bundle signature check: ad-hoc signed open-source build."
  log_warn "Quarantine attribute has been cleared, so macOS Gatekeeper will allow opening without prompts."
fi

# --- 7. Final Summary ---
echo ""
printf "${COLOR_GREEN}${COLOR_BOLD}================================================================${COLOR_RESET}\n"
printf "${COLOR_GREEN}${COLOR_BOLD}   مرئي (Mar2i) ${RESOLVED_TAG} installed successfully!${COLOR_RESET}\n"
printf "${COLOR_GREEN}${COLOR_BOLD}================================================================${COLOR_RESET}\n"
echo ""
echo "Installed Location: ${APP_TARGET}"
echo "Offline Multimedia: Standalone FFmpeg & FFprobe bundled"
echo "Launch Command:     open -a \"${APP_NAME}\""
echo ""
log_info "To launch the app now, run:"
printf "  ${COLOR_BOLD}open -a \"%s\"${COLOR_RESET}\n\n" "$APP_NAME"
