#!/usr/bin/env bash
# ==============================================================================
# Mar2i — Homebrew Cask Updater & Tap Synchronizer
# https://github.com/imMamdouhaboammar/mar2i
#
# Usage:
#   ./scripts/update-homebrew-cask.sh [--version=0.2.3]
#
# Environment variables:
#   HOMEBREW_TAP_TOKEN  GitHub Personal Access Token (PAT) with repo write permissions to tap repo
#   HOMEBREW_TAP_REPO   Target tap repository slug (default: imMamdouhaboammar/homebrew-mar2i)
#   MAR2I_REPO          Source release repository slug (default: imMamdouhaboammar/mar2i)
#   MAR2I_VERSION       Target release version (default: latest GitHub release)
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
TARGET_VERSION="${MAR2I_VERSION:-}"
for arg in "$@"; do
  case "$arg" in
    --version=*)
      TARGET_VERSION="${arg#*=}"
      ;;
    --help|-h)
      cat <<EOF
Usage: update-homebrew-cask.sh [OPTIONS]

Options:
  --version=X.Y.Z   Release version to update Cask to (e.g. 0.2.3)
  -h, --help        Show this help message

Environment variables:
  HOMEBREW_TAP_TOKEN  GitHub token to push updates to homebrew-mar2i tap
  HOMEBREW_TAP_REPO   Target Homebrew tap repository (default: imMamdouhaboammar/homebrew-mar2i)
  MAR2I_REPO          Source repository (default: imMamdouhaboammar/mar2i)
EOF
      exit 0
      ;;
  esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
CASK_FILE="${REPO_ROOT}/packaging/homebrew/Casks/mar2i.rb"

if [ ! -f "$CASK_FILE" ]; then
  die "Cask formula file not found at ${CASK_FILE}."
fi

SOURCE_REPO="${MAR2I_REPO:-imMamdouhaboammar/mar2i}"
TAP_REPO="${HOMEBREW_TAP_REPO:-imMamdouhaboammar/homebrew-mar2i}"

TMP_DIR="$(mktemp -d -t mar2i-cask-update-XXXXXX)"
cleanup() {
  local exit_status=$?
  rm -rf "$TMP_DIR" 2>/dev/null || true
  exit "$exit_status"
}
trap cleanup EXIT INT TERM HUP

# --- Resolve Release Metadata ---
if [ -n "$TARGET_VERSION" ]; then
  TAG_INPUT="$TARGET_VERSION"
  [[ "$TAG_INPUT" != v* ]] && RELEASE_TAG="v${TAG_INPUT}" || RELEASE_TAG="${TAG_INPUT}"
  log_info "Resolving release: ${RELEASE_TAG} from ${SOURCE_REPO}..."
  RELEASE_URL="https://api.github.com/repos/${SOURCE_REPO}/releases/tags/${RELEASE_TAG}"
else
  log_info "Resolving latest release from ${SOURCE_REPO}..."
  RELEASE_URL="https://api.github.com/repos/${SOURCE_REPO}/releases/latest"
fi

RELEASE_JSON="${TMP_DIR}/release.json"
HTTP_STATUS=$(curl -fsSL -w "%{http_code}" -H "Accept: application/vnd.github.v3+json" "$RELEASE_URL" -o "$RELEASE_JSON" || echo "000")

if [ "$HTTP_STATUS" != "200" ]; then
  die "Failed to query GitHub API for release at ${RELEASE_URL} (HTTP status: ${HTTP_STATUS})."
fi

RESOLVED_TAG="$(awk -F'"' '/"tag_name":/ { print $4; exit }' "$RELEASE_JSON")"
CLEAN_VERSION="${RESOLVED_TAG#v}"
log_success "Found release tag: ${COLOR_BOLD}${RESOLVED_TAG}${COLOR_RESET} (version: ${CLEAN_VERSION})"

# --- Parse Release Assets & Checksums ---
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
except Exception:
    sys.exit(1)
' "$RELEASE_JSON" > "$ASSET_INFO" || true
fi

ARM_DMG_URL=""
ARM_DMG_NAME=""
INTEL_DMG_URL=""
INTEL_DMG_NAME=""
CHECKSUM_URL=""

while IFS='|' read -r asset_name asset_url; do
  [ -z "$asset_name" ] && continue
  if [[ "$asset_name" =~ (aarch64|arm64).*\.dmg$ ]]; then
    ARM_DMG_NAME="$asset_name"
    ARM_DMG_URL="$asset_url"
  elif [[ "$asset_name" =~ (x64|x86_64).*\.dmg$ ]]; then
    INTEL_DMG_NAME="$asset_name"
    INTEL_DMG_URL="$asset_url"
  elif [[ "$asset_name" =~ universal.*\.dmg$ ]]; then
    [ -z "$ARM_DMG_NAME" ] && { ARM_DMG_NAME="$asset_name"; ARM_DMG_URL="$asset_url"; }
    [ -z "$INTEL_DMG_NAME" ] && { INTEL_DMG_NAME="$asset_name"; INTEL_DMG_URL="$asset_url"; }
  fi

  case "$asset_name" in
    SHA256SUMS.txt|SHA256SUMS-macos-*.txt)
      CHECKSUM_URL="$asset_url"
      ;;
  esac
done < "$ASSET_INFO"

# Compute or fetch SHA256 for Apple Silicon asset
ARM_SHA=""
if [ -n "$ARM_DMG_URL" ]; then
  log_info "Calculating SHA-256 for ARM asset: ${ARM_DMG_NAME}..."
  ARM_DEST="${TMP_DIR}/${ARM_DMG_NAME}"
  curl -fsSL "$ARM_DMG_URL" -o "$ARM_DEST"
  ARM_SHA="$(shasum -a 256 "$ARM_DEST" | awk '{print $1}')"
  log_success "Apple Silicon SHA-256: ${ARM_SHA}"
fi

# Compute or fetch SHA256 for Intel asset
INTEL_SHA=""
if [ -n "$INTEL_DMG_URL" ]; then
  log_info "Calculating SHA-256 for Intel asset: ${INTEL_DMG_NAME}..."
  INTEL_DEST="${TMP_DIR}/${INTEL_DMG_NAME}"
  curl -fsSL "$INTEL_DMG_URL" -o "$INTEL_DEST"
  INTEL_SHA="$(shasum -a 256 "$INTEL_DEST" | awk '{print $1}')"
  log_success "Intel SHA-256: ${INTEL_SHA}"
fi

if [ -z "$ARM_SHA" ]; then
  die "Could not resolve or download Apple Silicon DMG asset for release ${RESOLVED_TAG}."
fi

# If Intel DMG is not present in release, mirror arm64 or maintain placeholder
if [ -z "$INTEL_SHA" ]; then
  log_warn "Intel x64 DMG not found in release ${RESOLVED_TAG}. Setting intel sha256 to mirror arm64 until multi-arch release is built."
  INTEL_SHA="$ARM_SHA"
fi

# --- Update Cask Formula File ---
log_info "Updating ${CASK_FILE}..."

if command -v python3 >/dev/null 2>&1; then
  python3 -c '
import sys, re

cask_path = sys.argv[1]
version = sys.argv[2]
arm_sha = sys.argv[3]
intel_sha = sys.argv[4]

with open(cask_path, "r", encoding="utf-8") as f:
    content = f.read()

# Update version
content = re.sub(r"version \"[^\"]+\"", f"version \"{version}\"", content)

# Update sha256 block
sha_replacement = f"""sha256 arm:   \"{arm_sha}\",\n         intel: \"{intel_sha}\""""
content = re.sub(r"sha256 arm:\s*\"[^\"]+\",\s*intel:\s*\"[^\"]+\"", sha_replacement, content)

with open(cask_path, "w", encoding="utf-8") as f:
    f.write(content)
' "$CASK_FILE" "$CLEAN_VERSION" "$ARM_SHA" "$INTEL_SHA"
else
  # Sed fallback
  sed -i '' -e "s/version \".*\"/version \"${CLEAN_VERSION}\"/" "$CASK_FILE"
  sed -i '' -e "s/arm:   \".*\"/arm:   \"${ARM_SHA}\"/" "$CASK_FILE"
  sed -i '' -e "s/intel: \".*\"/intel: \"${INTEL_SHA}\"/" "$CASK_FILE"
fi

# Verify Ruby syntax of the modified Cask
if command -v ruby >/dev/null 2>&1; then
  ruby -c "$CASK_FILE" >/dev/null
  log_success "Updated ${CASK_FILE} with valid Ruby syntax (version: ${CLEAN_VERSION})"
else
  log_success "Updated ${CASK_FILE} (version: ${CLEAN_VERSION})"
fi

# --- Push to Homebrew Tap Repo (Optional & Guarded) ---
TOKEN="${HOMEBREW_TAP_TOKEN:-${GH_TOKEN:-}}"

if [ -z "$TOKEN" ]; then
  log_warn "Neither HOMEBREW_TAP_TOKEN nor GH_TOKEN is set."
  log_warn "Local Cask updated successfully. Remote tap push to '${TAP_REPO}' skipped."
  echo ""
  echo "To sync manually or in CI, configure repository secret HOMEBREW_TAP_TOKEN."
  exit 0
fi

log_info "Attempting synchronization with Homebrew tap repository '${TAP_REPO}'..."
TAP_CLONE_DIR="${TMP_DIR}/tap_repo"
TAP_URL="https://x-access-token:${TOKEN}@github.com/${TAP_REPO}.git"

if ! git clone --depth 1 "$TAP_URL" "$TAP_CLONE_DIR" 2>/dev/null; then
  log_warn "Could not clone tap repository '${TAP_REPO}'. Ensure the repository exists and the token has repo permissions."
  log_warn "Skipping remote push (release will not fail)."
  exit 0
fi

mkdir -p "${TAP_CLONE_DIR}/Casks"
cp "$CASK_FILE" "${TAP_CLONE_DIR}/Casks/mar2i.rb"

cd "$TAP_CLONE_DIR"
git config user.name "github-actions[bot]"
git config user.email "github-actions[bot]@users.noreply.github.com"

if git diff --quiet Casks/mar2i.rb; then
  log_success "Tap repository '${TAP_REPO}' is already up to date with version ${CLEAN_VERSION}."
else
  git add Casks/mar2i.rb
  git commit -m "Update mar2i cask to v${CLEAN_VERSION}"
  if git push origin HEAD 2>/dev/null; then
    log_success "Successfully pushed updated Cask to ${TAP_REPO}!"
  else
    log_warn "Failed to push to ${TAP_REPO}. Check token permissions."
  fi
fi
