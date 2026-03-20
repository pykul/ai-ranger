# AI Ranger - Windows update script
#
# Stops the running service, downloads the latest binary from GitHub Releases,
# verifies the checksum, replaces the existing binary, and restarts the service.
# Enrollment config is not touched -- the agent remembers its enrollment.
#
# Usage (run as Administrator):
#   powershell -ExecutionPolicy Bypass -File scripts\update\windows.ps1
#
# Requirements:
#   - Administrator privileges
#   - PowerShell 5.1+
#   - AI Ranger already installed via scripts\install\windows.ps1

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# -- Constants ----------------------------------------------------------------

# GitHub repository for release downloads.
$GithubRepo = "pykul/ai-ranger"

# Rust target triple for Windows x86_64.
$Target = "x86_64-pc-windows-msvc"

# Release asset filename.
$Archive = "ai-ranger-${Target}.zip"

# Base URL for the latest release download.
$ReleasesUrl = "https://github.com/${GithubRepo}/releases/latest/download"

# Checksums file published with every release.
$ChecksumsFile = "checksums.txt"

# Installation directory and binary path (must match install script).
$InstallDir = "C:\Program Files\AI Ranger"
$BinaryName = "ai-ranger.exe"
$BinaryPath = Join-Path $InstallDir $BinaryName

# Windows Service name (must match install script).
$ServiceName = "AIRanger"

# -- Preflight checks --------------------------------------------------------

# Verify running as Administrator.
$currentPrincipal = New-Object Security.Principal.WindowsPrincipal(
    [Security.Principal.WindowsIdentity]::GetCurrent()
)
if (-not $currentPrincipal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Error "This script must be run as Administrator."
    exit 1
}

$existingService = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if (-not $existingService) {
    Write-Error "Service ${ServiceName} not found. Is AI Ranger installed? Run scripts\install\windows.ps1 first."
    exit 1
}

# -- Stop service -------------------------------------------------------------

Write-Host "Stopping ${ServiceName} service..."
Stop-Service -Name $ServiceName -Force -ErrorAction SilentlyContinue

# -- Download and replace binary ----------------------------------------------

$TempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("ai-ranger-update-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $TempDir -Force | Out-Null

try {
    $ArchivePath = Join-Path $TempDir $Archive
    $ChecksumsPath = Join-Path $TempDir $ChecksumsFile

    Write-Host "Downloading ${Archive}..."
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    Invoke-WebRequest -Uri "${ReleasesUrl}/${Archive}" -OutFile $ArchivePath -UseBasicParsing

    Write-Host "Downloading ${ChecksumsFile}..."
    Invoke-WebRequest -Uri "${ReleasesUrl}/${ChecksumsFile}" -OutFile $ChecksumsPath -UseBasicParsing

    # Verify checksum.
    Write-Host "Verifying checksum..."
    $expectedLine = Get-Content $ChecksumsPath | Where-Object { $_ -match $Archive }
    if (-not $expectedLine) {
        Write-Error "Archive ${Archive} not found in ${ChecksumsFile}"
        exit 1
    }
    $expectedHash = ($expectedLine -split "\s+")[0]
    $actualHash = (Get-FileHash -Path $ArchivePath -Algorithm SHA256).Hash.ToLower()
    if ($expectedHash -ne $actualHash) {
        Write-Error "Checksum mismatch for ${Archive}.`nExpected: ${expectedHash}`nActual:   ${actualHash}"
        exit 1
    }
    Write-Host "Checksum verified."

    # Extract and replace binary.
    Write-Host "Extracting binary..."
    Expand-Archive -Path $ArchivePath -DestinationPath $TempDir -Force
    Copy-Item -Path (Join-Path $TempDir $BinaryName) -Destination $BinaryPath -Force

    Write-Host "Updated ai-ranger at ${BinaryPath}"
}
finally {
    Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
}

# -- Restart service ----------------------------------------------------------

Write-Host "Starting ${ServiceName} service..."
Start-Service -Name $ServiceName

# -- Done ---------------------------------------------------------------------

Write-Host ""
Write-Host "AI Ranger updated and running."
Write-Host ""
Get-Service -Name $ServiceName | Format-Table -Property Name, Status, StartType -AutoSize
