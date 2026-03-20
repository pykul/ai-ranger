#!/usr/bin/env bash
# AI Ranger - Linux installer
#
# Downloads the latest agent binary, enrolls with a backend, installs a systemd
# service, and starts it. After running this script the agent runs in the
# background, starts on boot, and reports to the configured backend.
#
# Usage:
#   sudo bash scripts/install/linux.sh --token=TOK --backend=https://ranger.example.com/ingest
#
# Requirements:
#   - Root privileges (raw socket capture requires root)
#   - curl
#   - systemd
#   - x86_64 or aarch64 architecture

set -euo pipefail

# -- Constants ----------------------------------------------------------------

# Where the agent binary is installed.
readonly INSTALL_DIR="/usr/local/bin"
readonly BINARY_NAME="ai-ranger"
readonly BINARY_PATH="${INSTALL_DIR}/${BINARY_NAME}"

# systemd unit file path.
readonly UNIT_FILE="/etc/systemd/system/ai-ranger.service"

# Restart delay after the agent exits unexpectedly.
# 5 seconds avoids tight restart loops while recovering quickly from transient failures.
readonly RESTART_DELAY_SECS=5

# Path to the shared download library relative to this script.
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly LIB_DIR="${SCRIPT_DIR}/../lib"

# -- Argument parsing ---------------------------------------------------------

TOKEN=""
BACKEND=""

for arg in "$@"; do
    case "${arg}" in
        --token=*)  TOKEN="${arg#--token=}" ;;
        --backend=*) BACKEND="${arg#--backend=}" ;;
        *)
            echo "Unknown argument: ${arg}" >&2
            echo "Usage: sudo bash $0 --token=TOKEN --backend=URL" >&2
            exit 1
            ;;
    esac
done

if [ -z "${TOKEN}" ] || [ -z "${BACKEND}" ]; then
    echo "Error: --token and --backend are required" >&2
    echo "Usage: sudo bash $0 --token=TOKEN --backend=URL" >&2
    exit 1
fi

# -- Preflight checks --------------------------------------------------------

if [ "$(id -u)" -ne 0 ]; then
    echo "Error: this script must be run as root (sudo)" >&2
    exit 1
fi

if ! command -v curl >/dev/null 2>&1; then
    echo "Error: curl is required but not found" >&2
    exit 1
fi

if ! command -v systemctl >/dev/null 2>&1; then
    echo "Error: systemd is required but systemctl was not found" >&2
    exit 1
fi

# -- Download binary ----------------------------------------------------------

# shellcheck source=../lib/download.sh
source "${LIB_DIR}/download.sh"

TARGET="$(detect_target)"
download_binary "${TARGET}" "${BINARY_PATH}"

# -- Enroll -------------------------------------------------------------------

echo "Enrolling with backend at ${BACKEND}..."
"${BINARY_PATH}" --enroll --token="${TOKEN}" --backend="${BACKEND}"

# Print the enrollment config path so the admin knows where it landed.
# Running as root, so config is under /root/.config/ai-ranger/config.json
readonly CONFIG_PATH="/root/.config/ai-ranger/config.json"
echo "Enrollment config saved to: ${CONFIG_PATH}"

# -- Install systemd service --------------------------------------------------

echo "Installing systemd service..."
cat > "${UNIT_FILE}" <<UNIT
[Unit]
Description=AI Ranger - passive AI provider detection agent
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=${BINARY_PATH}
Restart=on-failure
RestartSec=${RESTART_DELAY_SECS}

[Install]
WantedBy=multi-user.target
UNIT

systemctl daemon-reload
systemctl enable ai-ranger
systemctl start ai-ranger

# -- Done ---------------------------------------------------------------------

echo ""
echo "AI Ranger installed and running."
echo ""
systemctl status ai-ranger --no-pager
