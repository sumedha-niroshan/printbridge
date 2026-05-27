# PXL — Install self-signed certificate into Windows Trusted Root
# Run as Administrator

param(
    [string]$CertPath = "$env:APPDATA\PXL\certs\printbridge.crt"
)

$ErrorActionPreference = "Stop"

Write-Host "PXL Certificate Installer" -ForegroundColor Cyan
Write-Host "=========================" -ForegroundColor Cyan

# Check admin
$principal = [Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Error "This script must be run as Administrator. Right-click and select 'Run as Administrator'."
    exit 1
}

# Wait for cert file (PXL generates it on first run)
if (-not (Test-Path $CertPath)) {
    Write-Host "Certificate not found at $CertPath" -ForegroundColor Yellow
    Write-Host "Starting PXL to generate certificate..." -ForegroundColor Yellow

    $exePath = Join-Path $PSScriptRoot "PXL.exe"
    if (Test-Path $exePath) {
        $proc = Start-Process $exePath -PassThru
        Start-Sleep -Seconds 3
        $proc.Kill()
    } else {
        Write-Error "PXL.exe not found. Run PXL.exe once first to generate the certificate."
        exit 1
    }
}

if (-not (Test-Path $CertPath)) {
    Write-Error "Certificate still not found at $CertPath. Please run PXL.exe manually first."
    exit 1
}

# Import into Trusted Root
Write-Host "Installing certificate from: $CertPath" -ForegroundColor Green
$cert = New-Object System.Security.Cryptography.X509Certificates.X509Certificate2($CertPath)
$store = New-Object System.Security.Cryptography.X509Certificates.X509Store(
    [System.Security.Cryptography.X509Certificates.StoreName]::Root,
    [System.Security.Cryptography.X509Certificates.StoreLocation]::LocalMachine
)
$store.Open([System.Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite)
$store.Add($cert)
$store.Close()

Write-Host ""
Write-Host "Certificate installed successfully!" -ForegroundColor Green
Write-Host "Thumbprint: $($cert.Thumbprint)" -ForegroundColor Gray
Write-Host ""
Write-Host "You can now use PXL from Chrome, Edge, and Firefox." -ForegroundColor Cyan
Write-Host "Restart your browser if it was already open." -ForegroundColor Yellow
