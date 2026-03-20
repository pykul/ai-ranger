#!/usr/bin/env bash
# AI Ranger - macOS installer
#
# Downloads the latest agent binary, enrolls with a backend, installs a launchd
# daemon, and starts it. After running this script the agent runs in the
# background, starts on boot, and reports to the configured backend.
#
# Usage:
#   sudo bash scripts/install/macos.sh --token=TOK --backend=https://ranger.example.com/ingest
#
# Requirements:
#   - Root privileges (raw socket capture requires root)
#   - curl
#   - Intel (x86_64) or Apple Silicon (arm64)

set -euo pipefail

# -- Constants ----------------------------------------------------------------

# Where the agent binary is installed.
readonly INSTALL_DIR="/usr/local/bin"
readonly BINARY_NAME="ai-ranger"
readonly BINARY_PATH="${INSTALL_DIR}/${BINARY_NAME}"

# launchd plist path. LaunchDaemons run as root and start on boot.
readonly PLIST_PATH="/Library/LaunchDaemons/io.ai-ranger.agent.plist"

# launchd service label matching the plist filename convention.
readonly SERVICE_LABEL="io.ai-ranger.agent"

# Directory for agent log files (stdout and stderr from launchd).
readonly LOG_DIR="/var/log/ai-ranger"

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

# -- Download binary ----------------------------------------------------------

# shellcheck source=../lib/download.sh
source "${LIB_DIR}/download.sh"

TARGET="$(detect_target)"
download_binary "${TARGET}" "${BINARY_PATH}"

# -- Enroll -------------------------------------------------------------------

echo "Enrolling with backend at ${BACKEND}..."
"${BINARY_PATH}" --enroll --token="${TOKEN}" --backend="${BACKEND}"

# Print the enrollment config path so the admin knows where it landed.
# Running as root, so config is under /var/root/Library/Application Support/ai-ranger/config.json
readonly CONFIG_PATH="/var/root/Library/Application Support/ai-ranger/config.json"
echo "Enrollment config saved to: ${CONFIG_PATH}"

# -- Install launchd daemon ---------------------------------------------------

echo "Installing launchd daemon..."
mkdir -p "${LOG_DIR}"

cat > "${PLIST_PATH}" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>${SERVICE_LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>${BINARY_PATH}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>${LOG_DIR}/stdout.log</string>
    <key>StandardErrorPath</key>
    <string>${LOG_DIR}/stderr.log</string>
</dict>
</plist>
PLIST

# Load the daemon. If already loaded, unload first to pick up plist changes.
if launchctl list "${SERVICE_LABEL}" >/dev/null 2>&1; then
    launchctl unload "${PLIST_PATH}" 2>/dev/null || true
fi
launchctl load "${PLIST_PATH}"

# -- Done ---------------------------------------------------------------------

echo ""
echo "AI Ranger installed and running."
echo "Logs: ${LOG_DIR}/stdout.log and ${LOG_DIR}/stderr.log"
echo ""
echo "Check status: launchctl list ${SERVICE_LABEL}"
launchctl list "${SERVICE_LABEL}" 2>/dev/null || true
