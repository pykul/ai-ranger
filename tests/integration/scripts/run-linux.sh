#!/usr/bin/env bash
# run-linux.sh - Run integration tests on Linux (including WSL).
# Requires: cargo, docker compose, python3, curl, sudo.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/common.sh"

AGENT_BINARY="$REPO_ROOT/target/release/ai-ranger"

ensure_env_file
build_agent
start_stack
install_test_deps
run_tests "$AGENT_BINARY" "sudo"
