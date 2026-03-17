# run-windows.ps1 - Run real agent integration tests on Windows.
# Requires: cargo, docker compose (Docker Desktop), python3, Administrator.
#
# This script builds the Windows agent, starts the backend via Docker Compose,
# and runs only the real agent tests (test_ingest_real_agent.py).
# Synthetic/backend tests run on Linux CI and don't need a Windows runner.

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = (Resolve-Path "$ScriptDir\..\..\..").Path

# -- Config -------------------------------------------------------------------
$GatewayUrl = if ($env:GATEWAY_URL) { $env:GATEWAY_URL } else { "http://localhost:8080" }
$ApiUrl     = if ($env:API_URL)     { $env:API_URL }     else { "http://localhost:8081" }
$SeedToken  = if ($env:SEED_TOKEN)  { $env:SEED_TOKEN }  else { "tok_test_dev" }

# -- Helpers ------------------------------------------------------------------
function Step($msg) { Write-Host "==> $msg" }

function Ensure-EnvFile {
    $envFile = Join-Path $RepoRoot ".env"
    if (-not (Test-Path $envFile)) {
        Copy-Item (Join-Path $RepoRoot ".env.example") $envFile
        Write-Host "    Created .env from .env.example"
    }
}

function Build-Agent {
    Step "Building agent (release)..."
    cargo build --release --manifest-path (Join-Path $RepoRoot "agent\Cargo.toml")
    if ($LASTEXITCODE -ne 0) { throw "Agent build failed" }
}

function Start-Stack {
    Step "Building and starting Docker Compose stack (waiting for healthy)..."
    $envFile = Join-Path $RepoRoot ".env"
    $composeFile = Join-Path $RepoRoot "docker\docker-compose.yml"
    docker compose --env-file $envFile -f $composeFile up -d --build --wait
    if ($LASTEXITCODE -ne 0) { throw "Docker Compose up failed" }
}

function Install-TestDeps {
    Step "Installing test dependencies..."
    pip install -q -r (Join-Path $RepoRoot "tests\integration\requirements.txt")
    if ($LASTEXITCODE -ne 0) { throw "Failed to install test dependencies" }
}

function Run-Tests {
    param([string]$AgentBinary)

    Step "Running real agent integration tests..."
    $env:AGENT_BINARY = $AgentBinary
    $env:GATEWAY_URL  = $GatewayUrl
    $env:API_URL      = $ApiUrl
    $env:SEED_TOKEN   = $SeedToken

    python -m pytest (Join-Path $RepoRoot "tests\integration\test_ingest_real_agent.py") -v
    if ($LASTEXITCODE -ne 0) { throw "Integration tests failed" }

    Write-Host ""
    Step "All integration tests passed."
}

# -- Main ---------------------------------------------------------------------
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
    [Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    throw "This script must be run as Administrator (SIO_RCVALL requires elevated privileges)"
}

Ensure-EnvFile
Build-Agent
Start-Stack
Install-TestDeps

$agentBinary = Join-Path $RepoRoot "target\release\ai-ranger.exe"
Run-Tests -AgentBinary $agentBinary
