#!/usr/bin/env bash
# AI Ranger - Linux update script
#
# Stops the running service, downloads the latest binary from GitHub Releases,
# verifies the checksum, replaces the existing binary, and restarts the service.
# Enrollment config is not touched -- the agent remembers its enrollment.
#
# Usage:
#   sudo bash scripts/update/linux.sh
#
# Requirements:
#   - Root privileges
#   - curl
#   - systemd
#   - AI Ranger already installed via scripts/install/linux.sh

set -euo pipefail

# -- Constants ----------------------------------------------------------------

# Where the agent binary is installed (must match install script).
readonly BINARY_PATH="/usr/local/bin/ai-ranger"

# systemd service name (must match install script).
readonly SERVICE_NAME="ai-ranger"

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

if ! systemctl is-active --quiet "${SERVICE_NAME}" 2>/dev/null; then
    echo "Warning: ${SERVICE_NAME} service is not currently running" >&2
fi

# -- Stop service -------------------------------------------------------------

echo "Stopping ${SERVICE_NAME} service..."
systemctl stop "${SERVICE_NAME}" 2>/dev/null || true

# -- Download and replace binary ----------------------------------------------

# shellcheck source=../lib/download.sh
source "${LIB_DIR}/download.sh"

TARGET="$(detect_target)"
download_binary "${TARGET}" "${BINARY_PATH}"

# -- Restart service ----------------------------------------------------------

echo "Starting ${SERVICE_NAME} service..."
systemctl start "${SERVICE_NAME}"

# -- Done ---------------------------------------------------------------------

echo ""
echo "AI Ranger updated and running."
echo ""
systemctl status "${SERVICE_NAME}" --no-pager
