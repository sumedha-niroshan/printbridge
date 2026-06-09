# PrintBridge Certificate Installation Script
# Run as Administrator to trust the self-signed certificate
#
# Usage: Right-click → Run with PowerShell

param(
    [string]$CertPath = "$env:APPDATA\PXL\server.crt"
)

# Check if running as Administrator
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $isAdmin) {
    Write-Host "Error: This script must be run as Administrator" -ForegroundColor Red
    Write-Host "Please right-click this file and select 'Run with PowerShell'" -ForegroundColor Yellow
    exit 1
}

if (-not (Test-Path $CertPath)) {
    Write-Host "Error: Certificate not found at $CertPath" -ForegroundColor Red
    Write-Host "Please run PrintBridge.exe first to generate the certificate" -ForegroundColor Yellow
    exit 1
}

try {
    Write-Host "Installing certificate to Trusted Root..." -ForegroundColor Cyan
    
    # Import certificate to Trusted Root Store
    Import-Certificate -FilePath $CertPath -CertStoreLocation "Cert:\LocalMachine\Root" -Confirm:$false
    
    Write-Host "✓ Certificate installed successfully!" -ForegroundColor Green
    Write-Host "The PrintBridge service can now be accessed securely from your browser" -ForegroundColor Green
}
catch {
    Write-Host "Error: Failed to install certificate" -ForegroundColor Red
    Write-Host $_.Exception.Message -ForegroundColor Red
    exit 1
}
