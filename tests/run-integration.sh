#!/usr/bin/env bash
# run-integration.sh - Entry point for `make test-integration`.
# Detects the OS and delegates to the appropriate platform script.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

case "$(uname -s)" in
    Linux*)  exec bash "$SCRIPT_DIR/integration/scripts/run-linux.sh" "$@" ;;
    Darwin*) exec bash "$SCRIPT_DIR/integration/scripts/run-macos.sh" "$@" ;;
    MINGW*|MSYS*|CYGWIN*)
        echo "ERROR: Run this from WSL, not native Windows."
        echo "       Windows agents are tested via WSL with Docker Desktop."
        exit 1
        ;;
    *)
        echo "ERROR: Unsupported platform: $(uname -s)"
        exit 1
        ;;
esac
