# AI Ranger - Windows installer
#
# Downloads the latest agent binary, enrolls with a backend, registers a
# Windows Service, and starts it. After running this script the agent runs
# in the background, starts on boot, and reports to the configured backend.
#
# Usage (run as Administrator):
#   powershell -ExecutionPolicy Bypass -File scripts\install\windows.ps1 -Token TOK -Backend https://ranger.example.com/ingest
#
# Requirements:
#   - Administrator privileges (raw socket capture requires Administrator)
#   - PowerShell 5.1+ (built into Windows 10+)
#   - x86_64 architecture

param(
    [Parameter(Mandatory = $true)]
    [string]$Token,

    [Parameter(Mandatory = $true)]
    [string]$Backend
)

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

# Installation directory and binary path.
$InstallDir = "C:\Program Files\AI Ranger"
$BinaryName = "ai-ranger.exe"
$BinaryPath = Join-Path $InstallDir $BinaryName

# Windows Service name. Must match the SERVICE_NAME constant in the Rust agent.
$ServiceName = "AIRanger"
$ServiceDisplayName = "AI Ranger Agent"
$ServiceDescription = "AI Ranger passive AI provider detection agent"

# Seconds to wait after deleting an existing service for SCM cleanup.
$ServiceDeleteWaitSecs = 2

# -- Preflight checks --------------------------------------------------------

# Verify running as Administrator.
$currentPrincipal = New-Object Security.Principal.WindowsPrincipal(
    [Security.Principal.WindowsIdentity]::GetCurrent()
)
if (-not $currentPrincipal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Error "This script must be run as Administrator."
    exit 1
}

# -- Download binary ----------------------------------------------------------

$TempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("ai-ranger-install-" + [guid]::NewGuid().ToString("N"))
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

    # Extract binary.
    Write-Host "Extracting binary..."
    if (-not (Test-Path $InstallDir)) {
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    }
    Expand-Archive -Path $ArchivePath -DestinationPath $TempDir -Force
    Copy-Item -Path (Join-Path $TempDir $BinaryName) -Destination $BinaryPath -Force

    Write-Host "Installed ai-ranger to ${BinaryPath}"
}
finally {
    Remove-Item -Recurse -Force $TempDir -ErrorAction SilentlyContinue
}

# -- Enroll -------------------------------------------------------------------

Write-Host "Enrolling with backend at ${Backend}..."
& $BinaryPath --enroll --token=$Token --backend=$Backend
if ($LASTEXITCODE -ne 0) {
    Write-Error "Enrollment failed with exit code ${LASTEXITCODE}"
    exit 1
}

# Print the enrollment config path so the admin knows where it landed.
# Running as Administrator, the config lands under the current user's AppData
# unless running as SYSTEM. For a typical admin install this is:
#   C:\Users\<admin>\AppData\Roaming\ai-ranger\config.json
# When the service runs as LocalSystem it will be:
#   C:\Windows\System32\config\systemprofile\AppData\Roaming\ai-ranger\config.json
$AdminConfigPath = Join-Path $env:APPDATA "ai-ranger\config.json"
$SystemConfigDir = "C:\Windows\System32\config\systemprofile\AppData\Roaming\ai-ranger"
Write-Host "Enrollment config saved to: ${AdminConfigPath}"

# Copy enrollment config to the LocalSystem profile so the service can find it.
# The service runs as LocalSystem, which has a different APPDATA path.
Write-Host "Copying enrollment config to LocalSystem profile..."
if (-not (Test-Path $SystemConfigDir)) {
    New-Item -ItemType Directory -Path $SystemConfigDir -Force | Out-Null
}
Copy-Item -Path $AdminConfigPath -Destination (Join-Path $SystemConfigDir "config.json") -Force
Write-Host "Service config path: $(Join-Path $SystemConfigDir 'config.json')"

# -- Register Windows Service -------------------------------------------------

Write-Host "Registering Windows Service..."

# Remove existing service if present (handles reinstall).
$existingService = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if ($existingService) {
    Write-Host "Stopping existing service..."
    Stop-Service -Name $ServiceName -Force -ErrorAction SilentlyContinue
    # sc.exe delete is the reliable way to remove a service registration.
    & sc.exe delete $ServiceName | Out-Null
    # Brief pause to let the SCM finish cleanup.
    Start-Sleep -Seconds $ServiceDeleteWaitSecs
}

New-Service `
    -Name $ServiceName `
    -BinaryPathName $BinaryPath `
    -DisplayName $ServiceDisplayName `
    -Description $ServiceDescription `
    -StartupType Automatic | Out-Null

Write-Host "Starting service..."
Start-Service -Name $ServiceName

# -- Done ---------------------------------------------------------------------

Write-Host ""
Write-Host "AI Ranger installed and running."
Write-Host ""
Get-Service -Name $ServiceName | Format-Table -Property Name, Status, StartType -AutoSize
