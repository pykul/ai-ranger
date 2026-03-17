#!/usr/bin/env bash
# run-macos.sh - Run integration tests on macOS.
# Requires: cargo, docker compose (via Docker Desktop or Colima), python3, curl, sudo.
# Note: sudo is needed for BPF device access on macOS.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/common.sh"

AGENT_BINARY="$REPO_ROOT/target/release/ai-ranger"

ensure_env_file
build_agent
start_stack
install_test_deps
run_tests "$AGENT_BINARY" "sudo"
