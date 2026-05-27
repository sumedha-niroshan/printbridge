# PXL — Install as a Windows Service
# Run as Administrator

param(
    [string]$ExePath = "$PSScriptRoot\PXL.exe",
    [int]$Port = 8282,
    [switch]$Uninstall
)

$ServiceName = "PXL"
$DisplayName = "PXL Print Service"
$Description = "Silent browser print client. Provides local WebSocket API for web applications to print to Windows printers."

$ErrorActionPreference = "Stop"

# Check admin
$principal = [Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Error "Run as Administrator."
    exit 1
}

if ($Uninstall) {
    Write-Host "Uninstalling $ServiceName service..." -ForegroundColor Yellow
    $svc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
    if ($svc) {
        Stop-Service -Name $ServiceName -Force -ErrorAction SilentlyContinue
        & sc.exe delete $ServiceName | Out-Null
        Write-Host "Service removed." -ForegroundColor Green
    } else {
        Write-Host "Service not found." -ForegroundColor Gray
    }
    exit 0
}

Write-Host "Installing $DisplayName..." -ForegroundColor Cyan

if (-not (Test-Path $ExePath)) {
    Write-Error "PXL.exe not found at $ExePath"
    exit 1
}

# Update or create config.toml with the custom port
$AppDataDir = Join-Path $env:APPDATA "PXL"
if (-not (Test-Path $AppDataDir)) {
    New-Item -ItemType Directory -Path $AppDataDir | Out-Null
}
$ConfigPath = Join-Path $AppDataDir "config.toml"

if (Test-Path $ConfigPath) {
    Write-Host "Updating existing configuration at $ConfigPath with port $Port..." -ForegroundColor Gray
    $content = Get-Content $ConfigPath -Raw
    $content = $content -replace 'port\s*=\s*\d+', "port = $Port"
    Set-Content -Path $ConfigPath -Value $content
} else {
    Write-Host "Writing new default configuration with port $Port to $ConfigPath..." -ForegroundColor Gray
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

# Remove existing service if present
$existing = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
if ($existing) {
    Write-Host "Removing existing service..." -ForegroundColor Yellow
    Stop-Service -Name $ServiceName -Force -ErrorAction SilentlyContinue
    & sc.exe delete $ServiceName | Out-Null
    Start-Sleep -Seconds 1
}

# Create service
New-Service `
    -Name $ServiceName `
    -DisplayName $DisplayName `
    -Description $Description `
    -BinaryPathName "`"$ExePath`" --service" `
    -StartupType Automatic `
    | Out-Null

# Start it
Write-Host "Starting service..." -ForegroundColor Green
Start-Service -Name $ServiceName

$svc = Get-Service -Name $ServiceName
Write-Host ""
Write-Host "Service status: $($svc.Status)" -ForegroundColor $(if ($svc.Status -eq 'Running') { 'Green' } else { 'Red' })
Write-Host ""
Write-Host "PXL is now running as a Windows Service on port $Port." -ForegroundColor Cyan
Write-Host "It will start automatically with Windows." -ForegroundColor Cyan
Write-Host ""
Write-Host "Next: run install-cert.ps1 as Administrator to trust the TLS certificate." -ForegroundColor Yellow
