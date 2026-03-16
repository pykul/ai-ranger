# AI Ranger - install development dependencies on Windows via winget.
# Must be run as Administrator.
#Requires -RunAsAdministrator

Write-Host "=== AI Ranger: Windows dependency installer ===" -ForegroundColor Cyan
Write-Host ""

# -- Docker Desktop ------------------------------------------------------------
if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
    Write-Host "[1/5] Installing Docker Desktop..."
    winget install --id Docker.DockerDesktop --accept-source-agreements --accept-package-agreements
    Write-Host "       Please open Docker Desktop to finish setup."
} else {
    Write-Host "[1/5] Docker already installed - skipping"
}

# -- Rust (rustup) -------------------------------------------------------------
if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
    Write-Host "[2/5] Installing Rust via rustup..."
    winget install --id Rustlang.Rustup --accept-source-agreements --accept-package-agreements
} else {
    Write-Host "[2/5] Rust already installed - skipping"
}

# -- Go ------------------------------------------------------------------------
if (-not (Get-Command go -ErrorAction SilentlyContinue)) {
    Write-Host "[3/5] Installing Go..."
    winget install --id GoLang.Go --accept-source-agreements --accept-package-agreements
} else {
    Write-Host "[3/5] Go already installed - skipping"
}

# -- Python 3.12+ --------------------------------------------------------------
if (-not (Get-Command python3 -ErrorAction SilentlyContinue) -and -not (Get-Command python -ErrorAction SilentlyContinue)) {
    Write-Host "[4/5] Installing Python 3.12..."
    winget install --id Python.Python.3.12 --accept-source-agreements --accept-package-agreements
} else {
    Write-Host "[4/5] Python already installed - skipping"
}

# -- Protobuf compiler ---------------------------------------------------------
if (-not (Get-Command protoc -ErrorAction SilentlyContinue)) {
    Write-Host "[5/5] Installing protobuf compiler..."
    winget install --id Google.Protobuf --accept-source-agreements --accept-package-agreements
} else {
    Write-Host "[5/5] Protoc already installed - skipping"
}

# -- Verification --------------------------------------------------------------
Write-Host ""
Write-Host "=== Verification ===" -ForegroundColor Cyan
Write-Host "Docker:  " -NoNewline; try { docker --version } catch { Write-Host "not found - open Docker Desktop" }
Write-Host "Rust:    " -NoNewline; try { rustc --version } catch { Write-Host "not found" }
Write-Host "Cargo:   " -NoNewline; try { cargo --version } catch { Write-Host "not found" }
Write-Host "Go:      " -NoNewline; try { go version } catch { Write-Host "not found" }
Write-Host "Python:  " -NoNewline; try { python3 --version } catch { try { python --version } catch { Write-Host "not found" } }
Write-Host "Protoc:  " -NoNewline; try { protoc --version } catch { Write-Host "not found" }
Write-Host ""
Write-Host "=== All dependencies installed successfully ===" -ForegroundColor Green
