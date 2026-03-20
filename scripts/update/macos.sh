#!/usr/bin/env bash
# AI Ranger - macOS update script
#
# Stops the running daemon, downloads the latest binary from GitHub Releases,
# verifies the checksum, replaces the existing binary, and restarts the daemon.
# Enrollment config is not touched -- the agent remembers its enrollment.
#
# Usage:
#   sudo bash scripts/update/macos.sh
#
# Requirements:
#   - Root privileges
#   - curl
#   - AI Ranger already installed via scripts/install/macos.sh

set -euo pipefail

# -- Constants ----------------------------------------------------------------

# Where the agent binary is installed (must match install script).
readonly BINARY_PATH="/usr/local/bin/ai-ranger"

# launchd plist path (must match install script).
readonly PLIST_PATH="/Library/LaunchDaemons/io.ai-ranger.agent.plist"

# launchd service label (must match install script).
readonly SERVICE_LABEL="io.ai-ranger.agent"

# Path to the shared download library relative to this script.
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly LIB_DIR="${SCRIPT_DIR}/../lib"

# -- Preflight checks --------------------------------------------------------

if [ "$(id -u)" -ne 0 ]; then
    echo "Error: this script must be run as root (sudo)" >&2
    exit 1
fi

if ! command -v curl >/dev/null 2>&1; then
    echo "Error: curl is required but not found" >&2
    exit 1
fi

if [ ! -f "${PLIST_PATH}" ]; then
    echo "Error: plist not found at ${PLIST_PATH}" >&2
    echo "Is AI Ranger installed? Run scripts/install/macos.sh first." >&2
    exit 1
fi

# -- Stop daemon --------------------------------------------------------------

echo "Stopping ${SERVICE_LABEL} daemon..."
launchctl unload "${PLIST_PATH}" 2>/dev/null || true

# -- Download and replace binary ----------------------------------------------

# shellcheck source=../lib/download.sh
source "${LIB_DIR}/download.sh"

TARGET="$(detect_target)"
download_binary "${TARGET}" "${BINARY_PATH}"

# -- Restart daemon -----------------------------------------------------------

echo "Starting ${SERVICE_LABEL} daemon..."
launchctl load "${PLIST_PATH}"

# -- Done ---------------------------------------------------------------------

echo ""
echo "AI Ranger updated and running."
echo ""
echo "Check status: launchctl list ${SERVICE_LABEL}"
launchctl list "${SERVICE_LABEL}" 2>/dev/null || true
