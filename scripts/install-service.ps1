# PrintBridge — Install as a Windows Service
# Run as Administrator

param(
    [string]$ExePath = "$PSScriptRoot\PrintBridge.exe",
    [switch]$Uninstall
)

$ServiceName = "PrintBridge"
$DisplayName = "PrintBridge Print Service"
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
    Write-Error "PrintBridge.exe not found at $ExePath"
    exit 1
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
Write-Host "PrintBridge is now running as a Windows Service." -ForegroundColor Cyan
Write-Host "It will start automatically with Windows." -ForegroundColor Cyan
Write-Host ""
Write-Host "Next: run install-cert.ps1 as Administrator to trust the TLS certificate." -ForegroundColor Yellow
