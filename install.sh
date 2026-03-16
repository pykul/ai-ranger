#!/usr/bin/env bash
set -euo pipefail

# AI Ranger installer for Linux and macOS.
# Downloads the correct binary from GitHub Releases, verifies checksum,
# optionally enrolls with a backend, and installs as a system service.
#
# Usage:
#   curl -sSL https://your-instance.com/install.sh | sh -s -- --token=tok_abc123 --backend=https://your-instance.com
#   ./install.sh --token=tok_abc123 --backend=https://your-instance.com
#   ./install.sh  # standalone mode, no backend

REPO="pykul/ai-ranger"
INSTALL_DIR="/usr/local/bin"
BINARY="ai-ranger"

# Parse arguments
TOKEN=""
BACKEND=""
for arg in "$@"; do
  case "$arg" in
    --token=*) TOKEN="${arg#--token=}" ;;
    --backend=*) BACKEND="${arg#--backend=}" ;;
    --help|-h)
      echo "Usage: install.sh [--token=TOKEN --backend=URL]"
      echo ""
      echo "  --token    Enrollment token (required for backend mode)"
      echo "  --backend  Backend URL (required for backend mode)"
      echo ""
      echo "Without --token/--backend, installs in standalone mode (stdout only)."
      exit 0
      ;;
    *) echo "Unknown argument: $arg"; exit 1 ;;
  esac
done

# Detect OS and arch
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)
    case "$ARCH" in
      x86_64)  TARGET="x86_64-unknown-linux-gnu" ;;
      aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
      *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
    esac
    ;;
  Darwin)
    case "$ARCH" in
      x86_64)  TARGET="x86_64-apple-darwin" ;;
      arm64)   TARGET="aarch64-apple-darwin" ;;
      *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
    esac
    ;;
  *)
    echo "Unsupported OS: $OS (use the PowerShell installer for Windows)"
    exit 1
    ;;
esac

echo "[ai-ranger] Detected: $OS $ARCH → $TARGET"

# Get latest release tag
LATEST=$(curl -sSL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)
if [ -z "$LATEST" ]; then
  echo "[ai-ranger] Error: could not determine latest release"
  exit 1
fi
echo "[ai-ranger] Latest release: $LATEST"

# Download binary and checksum
DOWNLOAD_URL="https://github.com/$REPO/releases/download/$LATEST/ai-ranger-$TARGET"
CHECKSUM_URL="$DOWNLOAD_URL.sha256"

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

echo "[ai-ranger] Downloading $DOWNLOAD_URL"
curl -sSL -o "$TMPDIR/$BINARY" "$DOWNLOAD_URL"
curl -sSL -o "$TMPDIR/$BINARY.sha256" "$CHECKSUM_URL"

# Verify checksum
echo "[ai-ranger] Verifying checksum..."
cd "$TMPDIR"
if command -v sha256sum &>/dev/null; then
  sha256sum -c "$BINARY.sha256"
elif command -v shasum &>/dev/null; then
  shasum -a 256 -c "$BINARY.sha256"
else
  echo "[ai-ranger] Warning: no sha256 tool found, skipping checksum verification"
fi
cd - >/dev/null

# Install
echo "[ai-ranger] Installing to $INSTALL_DIR/$BINARY"
chmod +x "$TMPDIR/$BINARY"
sudo mv "$TMPDIR/$BINARY" "$INSTALL_DIR/$BINARY"

# Enroll if token provided
if [ -n "$TOKEN" ] && [ -n "$BACKEND" ]; then
  echo "[ai-ranger] Enrolling with backend..."
  "$INSTALL_DIR/$BINARY" --enroll --token="$TOKEN" --backend="$BACKEND"
fi

# Install as system service
if [ "$OS" = "Linux" ] && command -v systemctl &>/dev/null; then
  echo "[ai-ranger] Installing systemd service..."
  sudo tee /etc/systemd/system/ai-ranger.service >/dev/null <<UNIT
[Unit]
Description=AI Ranger Agent
After=network.target

[Service]
Type=simple
ExecStart=$INSTALL_DIR/$BINARY
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
UNIT
  sudo systemctl daemon-reload
  sudo systemctl enable ai-ranger
  sudo systemctl start ai-ranger
  echo "[ai-ranger] Service started. Check status: sudo systemctl status ai-ranger"

elif [ "$OS" = "Darwin" ]; then
  PLIST="/Library/LaunchDaemons/com.ai-ranger.agent.plist"
  echo "[ai-ranger] Installing launchd service..."
  sudo tee "$PLIST" >/dev/null <<PLIST_CONTENT
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.ai-ranger.agent</string>
    <key>ProgramArguments</key>
    <array>
        <string>$INSTALL_DIR/$BINARY</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/var/log/ai-ranger.log</string>
    <key>StandardErrorPath</key>
    <string>/var/log/ai-ranger.err</string>
</dict>
</plist>
PLIST_CONTENT
  sudo launchctl load "$PLIST"
  echo "[ai-ranger] Service started. Check logs: /var/log/ai-ranger.log"
fi

echo "[ai-ranger] Installation complete!"
