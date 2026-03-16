#!/usr/bin/env bash
# AI Ranger — install development dependencies on macOS via Homebrew.
set -euo pipefail

echo "=== AI Ranger: macOS dependency installer ==="
echo ""

# -- Homebrew ------------------------------------------------------------------
if ! command -v brew &>/dev/null; then
    echo "[1/5] Installing Homebrew..."
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
else
    echo "[1/5] Homebrew already installed — skipping"
fi

# -- Docker Desktop ------------------------------------------------------------
if ! command -v docker &>/dev/null; then
    echo "[2/5] Installing Docker Desktop..."
    brew install --cask docker
    echo "       Please open Docker Desktop to finish setup."
else
    echo "[2/5] Docker already installed — skipping"
fi

# -- Rust (rustup) -------------------------------------------------------------
if ! command -v rustup &>/dev/null; then
    echo "[3/5] Installing Rust via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    # shellcheck source=/dev/null
    source "$HOME/.cargo/env"
else
    echo "[3/5] Rust already installed — skipping"
fi

# -- Go ------------------------------------------------------------------------
if ! command -v go &>/dev/null; then
    echo "[4/5] Installing Go..."
    brew install go
else
    echo "[4/5] Go already installed — skipping"
fi

# -- Python 3.12+ & protobuf --------------------------------------------------
if ! command -v python3 &>/dev/null; then
    echo "[5/5] Installing Python 3 and protobuf..."
    brew install python@3.12 protobuf
else
    echo "[5/5] Python 3 already installed — skipping"
    if ! command -v protoc &>/dev/null; then
        echo "       Installing protobuf compiler..."
        brew install protobuf
    fi
fi

# -- Verification --------------------------------------------------------------
echo ""
echo "=== Verification ==="
echo "Docker:   $(docker --version 2>/dev/null || echo 'not found — open Docker Desktop')"
echo "Rust:     $(rustc --version 2>/dev/null || echo 'not found')"
echo "Cargo:    $(cargo --version 2>/dev/null || echo 'not found')"
echo "Go:       $(go version 2>/dev/null || echo 'not found')"
echo "Python:   $(python3 --version 2>/dev/null || echo 'not found')"
echo "Protoc:   $(protoc --version 2>/dev/null || echo 'not found')"
echo ""
echo "=== All dependencies installed successfully ==="
