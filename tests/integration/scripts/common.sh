#!/usr/bin/env bash
# common.sh - Shared functions for integration test scripts.
# Sourced by the platform-specific run-*.sh scripts.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

# Ports and URLs (match .env.example defaults).
GATEWAY_URL="${GATEWAY_URL:-http://localhost:8080}"
API_URL="${API_URL:-http://localhost:8081}"
SEED_TOKEN="${SEED_TOKEN:-tok_test_dev}"

step() {
    echo "==> $1"
}

ensure_env_file() {
    if [ ! -f "$REPO_ROOT/.env" ]; then
        cp "$REPO_ROOT/.env.example" "$REPO_ROOT/.env"
        echo "    Created .env from .env.example"
    fi
}

build_agent() {
    step "Building agent (release)..."
    cargo build --release --manifest-path "$REPO_ROOT/agent/Cargo.toml"
}

start_stack() {
    step "Building and starting Docker Compose stack (waiting for healthy)..."
    DOCKER_BUILDKIT=0 docker compose \
        --env-file "$REPO_ROOT/.env" \
        -f "$REPO_ROOT/docker/docker-compose.yml" \
        up -d --build --wait
}

install_test_deps() {
    step "Installing test dependencies..."
    pip install -q -r "$REPO_ROOT/tests/integration/requirements.txt" 2>/dev/null ||
        pip install -q --break-system-packages -r "$REPO_ROOT/tests/integration/requirements.txt"
}

run_tests() {
    local agent_binary="$1"
    local sudo_cmd="${2:-}"

    step "Running integration tests..."
    # Use sudo -E to preserve PATH so the same python3 (with installed deps) is found.
    $sudo_cmd -E env \
        AGENT_BINARY="$agent_binary" \
        GATEWAY_URL="$GATEWAY_URL" \
        API_URL="$API_URL" \
        SEED_TOKEN="$SEED_TOKEN" \
        "$(which python3)" -m pytest "$REPO_ROOT/tests/integration/" -v

    echo ""
    step "All integration tests passed."
}
