# PrintBridge Windows Service Uninstall Script
# Run as Administrator to remove PrintBridge Windows Service
#
# Usage: Right-click → Run with PowerShell

# Check if running as Administrator
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $isAdmin) {
    Write-Host "Error: This script must be run as Administrator" -ForegroundColor Red
    Write-Host "Please right-click this file and select 'Run with PowerShell'" -ForegroundColor Yellow
    exit 1
}

try {
    Write-Host "Uninstalling PrintBridge service..." -ForegroundColor Cyan
    
    # Check if service exists
    $service = Get-Service -Name "PrintBridge" -ErrorAction SilentlyContinue
    
    if (-not $service) {
        Write-Host "Service not found. Nothing to remove." -ForegroundColor Yellow
        exit 0
    }
    
    # Stop the service
    Write-Host "Stopping service..." -ForegroundColor Cyan
    Stop-Service -Name "PrintBridge" -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 2
    
    # Remove the service
    Write-Host "Removing service..." -ForegroundColor Cyan
    Remove-Service -Name "PrintBridge" -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 1
    
    Write-Host "✓ PrintBridge service removed successfully!" -ForegroundColor Green
}
catch {
    Write-Host "Error: Failed to uninstall service" -ForegroundColor Red
    Write-Host $_.Exception.Message -ForegroundColor Red
    exit 1
}
