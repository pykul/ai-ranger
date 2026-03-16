#!/usr/bin/env bash
# AI Ranger — install development dependencies on Linux.
# Supports Debian/Ubuntu (apt) and Fedora/RHEL (dnf).
set -euo pipefail

echo "=== AI Ranger: Linux dependency installer ==="
echo ""

# Detect package manager
if command -v apt-get &>/dev/null; then
    PKG="apt"
elif command -v dnf &>/dev/null; then
    PKG="dnf"
else
    echo "Error: neither apt nor dnf found. Install dependencies manually."
    exit 1
fi
echo "Detected package manager: $PKG"
echo ""

# -- Docker --------------------------------------------------------------------
if ! command -v docker &>/dev/null; then
    echo "[1/5] Installing Docker..."
    if [ "$PKG" = "apt" ]; then
        sudo apt-get update -qq
        sudo apt-get install -y -qq ca-certificates curl gnupg
        sudo install -m 0755 -d /etc/apt/keyrings
        curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg 2>/dev/null || true
        sudo chmod a+r /etc/apt/keyrings/docker.gpg
        echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu $(. /etc/os-release && echo "$VERSION_CODENAME") stable" | sudo tee /etc/apt/sources.list.d/docker.list >/dev/null
        sudo apt-get update -qq
        sudo apt-get install -y -qq docker-ce docker-ce-cli containerd.io docker-compose-plugin
    else
        sudo dnf install -y dnf-plugins-core
        sudo dnf config-manager --add-repo https://download.docker.com/linux/fedora/docker-ce.repo
        sudo dnf install -y docker-ce docker-ce-cli containerd.io docker-compose-plugin
        sudo systemctl enable --now docker
    fi
else
    echo "[1/5] Docker already installed — skipping"
fi

# -- Rust (rustup) -------------------------------------------------------------
if ! command -v rustup &>/dev/null; then
    echo "[2/5] Installing Rust via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    # shellcheck source=/dev/null
    source "$HOME/.cargo/env"
else
    echo "[2/5] Rust already installed — skipping"
fi

# -- Go ------------------------------------------------------------------------
if ! command -v go &>/dev/null; then
    echo "[3/5] Installing Go..."
    if [ "$PKG" = "apt" ]; then
        sudo apt-get install -y -qq golang
    else
        sudo dnf install -y golang
    fi
else
    echo "[3/5] Go already installed — skipping"
fi

# -- Python 3.12+ --------------------------------------------------------------
if ! command -v python3 &>/dev/null; then
    echo "[4/5] Installing Python 3..."
    if [ "$PKG" = "apt" ]; then
        sudo apt-get install -y -qq python3 python3-pip python3-venv
    else
        sudo dnf install -y python3 python3-pip
    fi
else
    echo "[4/5] Python 3 already installed — skipping"
fi

# -- Protobuf compiler ---------------------------------------------------------
if ! command -v protoc &>/dev/null; then
    echo "[5/5] Installing protobuf compiler..."
    if [ "$PKG" = "apt" ]; then
        sudo apt-get install -y -qq protobuf-compiler
    else
        sudo dnf install -y protobuf-compiler
    fi
else
    echo "[5/5] Protoc already installed — skipping"
fi

# -- Verification --------------------------------------------------------------
echo ""
echo "=== Verification ==="
echo "Docker:   $(docker --version 2>/dev/null || echo 'not found')"
echo "Compose:  $(docker compose version 2>/dev/null || echo 'not found')"
echo "Rust:     $(rustc --version 2>/dev/null || echo 'not found')"
echo "Cargo:    $(cargo --version 2>/dev/null || echo 'not found')"
echo "Go:       $(go version 2>/dev/null || echo 'not found')"
echo "Python:   $(python3 --version 2>/dev/null || echo 'not found')"
echo "Protoc:   $(protoc --version 2>/dev/null || echo 'not found')"
echo ""
echo "=== All dependencies installed successfully ==="
