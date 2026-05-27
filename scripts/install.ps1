# PXL — Install to Windows Startup (GUI always runs)
# Run as Administrator for certificate trust, or as normal user for startup only.

param(
    [string]$ExePath = "$PSScriptRoot\PXL.exe",
    [int]$Port = 8282,
    [switch]$Uninstall
)

$AppName = "PXL"

$ErrorActionPreference = "Stop"

# ── Uninstall ─────────────────────────────────────────────────────────────────
if ($Uninstall) {
    Write-Host "Removing PXL from Windows Startup..." -ForegroundColor Yellow

    # Remove from Registry Run key
    $regPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
    if (Get-ItemProperty -Path $regPath -Name $AppName -ErrorAction SilentlyContinue) {
        Remove-ItemProperty -Path $regPath -Name $AppName
        Write-Host "Removed from startup registry." -ForegroundColor Green
    }

    # Remove Startup shortcut if exists
    $startupFolder = [Environment]::GetFolderPath("Startup")
    $shortcut = Join-Path $startupFolder "$AppName.lnk"
    if (Test-Path $shortcut) {
        Remove-Item $shortcut -Force
        Write-Host "Removed startup shortcut." -ForegroundColor Green
    }

    Write-Host "PXL has been removed from Windows Startup." -ForegroundColor Green
    exit 0
}

# ── Validate ──────────────────────────────────────────────────────────────────
Write-Host ""
Write-Host "╔══════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║         PXL Print Client Setup       ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host ""

if (-not (Test-Path $ExePath)) {
    Write-Error "PXL.exe not found at $ExePath. Place this script in the same folder as PXL.exe."
    exit 1
}

$ExeFullPath = (Resolve-Path $ExePath).Path

# ── Step 1: Configure Port ────────────────────────────────────────────────────
Write-Host "[1/3] Configuring PXL on port $Port..." -ForegroundColor White

$AppDataDir = Join-Path $env:APPDATA "com.pxl\PXL\data"
if (-not (Test-Path $AppDataDir)) {
    New-Item -ItemType Directory -Path $AppDataDir -Force | Out-Null
}
$ConfigPath = Join-Path $AppDataDir "config.toml"

if (Test-Path $ConfigPath) {
    Write-Host "      Updating existing config with port $Port..." -ForegroundColor Gray
    $content = Get-Content $ConfigPath -Raw
    $content = $content -replace 'port\s*=\s*\d+', "port = $Port"
    Set-Content -Path $ConfigPath -Value $content
} else {
    Write-Host "      Writing default configuration..." -ForegroundColor Gray
    $defaultConfig = @"
[server]
port = $Port
host = "127.0.0.1"
allowed_origins = [
  "http://localhost",
  "http://localhost:3000",
  "http://localhost:8080",
  "http://localhost:5173",
  "http://127.0.0.1:5173",
  "*"
]

[cert]
path = "certs/printbridge.pfx"
common_name = "PXL Local"

[logging]
level = "info"
"@
    Set-Content -Path $ConfigPath -Value $defaultConfig
}

# ── Step 2: Add to Windows Startup ────────────────────────────────────────────
Write-Host "[2/3] Adding PXL to Windows Startup..." -ForegroundColor White

# Method 1: Registry Run key (most reliable)
$regPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
Set-ItemProperty -Path $regPath -Name $AppName -Value "`"$ExeFullPath`""
Write-Host "      Added to registry: HKCU\...\Run\$AppName" -ForegroundColor Gray

# Method 2: Also create a Startup folder shortcut (backup)
$startupFolder = [Environment]::GetFolderPath("Startup")
$shortcutPath = Join-Path $startupFolder "$AppName.lnk"
$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut($shortcutPath)
$shortcut.TargetPath = $ExeFullPath
$shortcut.WorkingDirectory = (Split-Path $ExeFullPath)
$shortcut.Description = "PXL Silent Print Client"
$shortcut.Save()
Write-Host "      Created shortcut: $shortcutPath" -ForegroundColor Gray

# ── Step 3: Generate TLS Certificate ─────────────────────────────────────────
Write-Host "[3/3] Generating TLS certificate (first run)..." -ForegroundColor White

# Run PXL briefly to generate the self-signed cert, then close
$proc = Start-Process $ExeFullPath -PassThru
Start-Sleep -Seconds 4
if (-not $proc.HasExited) {
    $proc.CloseMainWindow() | Out-Null
    Start-Sleep -Seconds 1
    if (-not $proc.HasExited) {
        $proc.Kill()
    }
}
Write-Host "      Certificate generated." -ForegroundColor Gray

# ── Done ──────────────────────────────────────────────────────────────────────
Write-Host ""
Write-Host "╔══════════════════════════════════════╗" -ForegroundColor Green
Write-Host "║        Installation Complete!        ║" -ForegroundColor Green
Write-Host "╚══════════════════════════════════════╝" -ForegroundColor Green
Write-Host ""
Write-Host "  Port    : $Port" -ForegroundColor White
Write-Host "  Config  : $ConfigPath" -ForegroundColor White
Write-Host "  Startup : Registry + Shortcut" -ForegroundColor White
Write-Host ""
Write-Host "  PXL will now auto-start with the GUI every time" -ForegroundColor Cyan
Write-Host "  you log in to Windows." -ForegroundColor Cyan
Write-Host ""
Write-Host "  Next steps:" -ForegroundColor Yellow
Write-Host "  1. Run install-cert.ps1 as Administrator to trust the TLS certificate." -ForegroundColor Yellow
Write-Host "  2. Double-click PXL.exe to start it now." -ForegroundColor Yellow
Write-Host ""
