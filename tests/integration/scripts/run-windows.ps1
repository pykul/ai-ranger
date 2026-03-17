# run-windows.ps1 - Run standalone agent tests on Windows.
# Requires: cargo, python3, Administrator.
#
# GitHub Actions Windows runners cannot run Linux containers, so Docker Compose
# is not available. This script runs only the standalone agent tests that do not
# require a backend (captures_sni and stdout_mode). The enrollment test and all
# backend pipeline tests run on the Linux CI runner.

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = (Resolve-Path "$ScriptDir\..\..\..").Path

# -- Helpers ------------------------------------------------------------------
function Step($msg) { Write-Host "==> $msg" }

function Build-Agent {
    Step "Building agent (release)..."
    cargo build --release --manifest-path (Join-Path $RepoRoot "agent\Cargo.toml")
    if ($LASTEXITCODE -ne 0) { throw "Agent build failed" }
}

function Install-TestDeps {
    Step "Installing test dependencies..."
    pip install -q -r (Join-Path $RepoRoot "tests\integration\requirements.txt")
    if ($LASTEXITCODE -ne 0) { throw "Failed to install test dependencies" }
}

function Run-Tests {
    param([string]$AgentBinary)

    Step "Running standalone agent tests (no backend required)..."
    $env:AGENT_BINARY = $AgentBinary

    # Run only standalone tests that do not need Docker Compose.
    # test_real_agent_enrollment requires the gateway and is excluded.
    python -m pytest (Join-Path $RepoRoot "tests\integration\test_ingest_real_agent.py") `
        -v -k "captures_sni or stdout_mode"
    if ($LASTEXITCODE -ne 0) { throw "Agent tests failed" }

    Write-Host ""
    Step "All Windows agent tests passed."
}

# -- Main ---------------------------------------------------------------------
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
    [Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    throw "This script must be run as Administrator (SIO_RCVALL requires elevated privileges)"
}

Build-Agent
Install-TestDeps

$agentBinary = Join-Path $RepoRoot "target\release\ai-ranger.exe"
Run-Tests -AgentBinary $agentBinary
