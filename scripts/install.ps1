# Reco AI — instalación completa en Windows.
#   irm https://raw.githubusercontent.com/DarioDGR12/Reco-AI/main/scripts/install.ps1 | iex
#   .\scripts\install.ps1 -Cli
param(
    [switch]$Cli,
    [switch]$NoLlama,
    [switch]$NoDesktop
)

$ErrorActionPreference = "Stop"
$RepoUrl = if ($env:RECO_REPO_URL) { $env:RECO_REPO_URL } else { "https://github.com/DarioDGR12/Reco-AI" }
$BinDir = if ($env:RECO_BIN_DIR) { $env:RECO_BIN_DIR } else { Join-Path $env:LOCALAPPDATA "Reco\bin" }
$WantLlama = -not ($Cli -or $NoLlama)
$WantDesktop = -not ($Cli -or $NoDesktop)

function Write-Step($msg) { Write-Host "`n==> $msg" -ForegroundColor White }
function Write-Ok($msg) { Write-Host "    ✓ $msg" -ForegroundColor Green }
function Write-Warn($msg) { Write-Host "    ! $msg" -ForegroundColor Yellow }

New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
$env:Path = "$BinDir;$env:USERPROFILE\.cargo\bin;$env:Path"

function Ensure-Rust {
    if (Get-Command cargo -ErrorAction SilentlyContinue) {
        Write-Ok "Rust $(rustc --version)"
        return
    }
    Write-Step "Instalando Rust (rustup)…"
    Invoke-RestMethod https://sh.rustup.rs -OutFile "$env:TEMP\rustup-init.exe" -ErrorAction SilentlyContinue
    $init = "$env:USERPROFILE\.cargo\bin\rustup-init.exe"
    $url = "https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe"
    Invoke-WebRequest -Uri $url -OutFile "$env:TEMP\rustup-init.exe"
    & "$env:TEMP\rustup-init.exe" -y --default-toolchain stable
    $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw "rustup terminó pero cargo no está en PATH. Abre una terminal nueva."
    }
    Write-Ok "Rust $(rustc --version)"
}

function Install-Reco {
    Write-Step "Instalando reco…"
    $here = Split-Path -Parent $PSScriptRoot
    if (Test-Path (Join-Path $PSScriptRoot "..\crates\reco-cli\Cargo.toml")) {
        $root = Resolve-Path (Join-Path $PSScriptRoot "..")
        cargo install --path (Join-Path $root "crates\reco-cli") --locked --force
    } else {
        cargo install --git $RepoUrl --path crates/reco-cli --locked --force
    }
    $reco = Join-Path $env:USERPROFILE ".cargo\bin\reco.exe"
    if (Test-Path $reco) {
        Copy-Item $reco (Join-Path $BinDir "reco.exe") -Force
    }
    Write-Ok "reco → $reco"
}

function Install-Llama {
    if (-not $WantLlama) { return }
    if (Get-Command llama-cli -ErrorAction SilentlyContinue) {
        Write-Ok "llama-cli ya está: $((Get-Command llama-cli).Source)"
        reco config set llama-cli (Get-Command llama-cli).Source | Out-Null
        return
    }
    Write-Step "Descargando llama-cli (llama.cpp)…"
    $api = "https://api.github.com/repos/ggml-org/llama.cpp/releases/latest"
    $rel = Invoke-RestMethod -Uri $api -Headers @{ "User-Agent" = "Reco-AI-install" }
    $asset = $rel.assets | Where-Object { $_.name -like "llama-*-bin-win-cpu-x64.zip" } | Select-Object -First 1
    if (-not $asset) {
        Write-Warn "no encontré el zip win-cpu-x64 de llama.cpp"
        return
    }
    $zip = Join-Path $env:TEMP "reco-llama.zip"
    Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $zip
    $dest = Join-Path $env:LOCALAPPDATA "Reco\llama"
    if (Test-Path $dest) { Remove-Item $dest -Recurse -Force }
    Expand-Archive -Path $zip -DestinationPath $dest -Force
    $cli = Get-ChildItem -Path $dest -Recurse -Filter "llama-cli.exe" | Select-Object -First 1
    if (-not $cli) {
        Write-Warn "el zip no traía llama-cli.exe"
        return
    }
    Copy-Item $cli.FullName (Join-Path $BinDir "llama-cli.exe") -Force
    reco config set llama-cli (Join-Path $BinDir "llama-cli.exe") | Out-Null
    Write-Ok "llama-cli → $BinDir\llama-cli.exe"
}

function Install-Desktop {
    if (-not $WantDesktop) { return }
    Write-Warn "La ventana Tauri en Windows se compila con: cd crates\reco-desktop; npm install; npm run tauri build"
    Write-Warn "Mientras tanto: reco run --tui"
}

Ensure-Rust
Install-Reco
Install-Llama
Install-Desktop
Write-Step "Listo"
Write-Host ""
Write-Host "  Añade a PATH si hace falta: $BinDir  y  $env:USERPROFILE\.cargo\bin"
Write-Host ""
if (Get-Command reco -ErrorAction SilentlyContinue) {
    reco setup
}
Write-Host "  Siguiente:  reco   (menú · flechas + enter)   ·   reco setup"
