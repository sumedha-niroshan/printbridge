# PrintBridge Windows Service Installation Script
# Run as Administrator to install PrintBridge as a Windows Service
#
# Usage: Right-click → Run with PowerShell

param(
    [string]$ExePath = "$env:ProgramFiles\PrintBridge\PrintBridge.exe"
)

# Check if running as Administrator
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $isAdmin) {
    Write-Host "Error: This script must be run as Administrator" -ForegroundColor Red
    Write-Host "Please right-click this file and select 'Run with PowerShell'" -ForegroundColor Yellow
    exit 1
}

# Check if executable exists
if (-not (Test-Path $ExePath)) {
    Write-Host "Error: PrintBridge.exe not found at $ExePath" -ForegroundColor Red
    Write-Host "Please install PrintBridge from the installer first" -ForegroundColor Yellow
    exit 1
}

try {
    Write-Host "Installing PrintBridge as Windows Service..." -ForegroundColor Cyan
    
    # Check if service already exists
    $service = Get-Service -Name "PrintBridge" -ErrorAction SilentlyContinue
    
    if ($service) {
        Write-Host "Service already exists. Removing old service..." -ForegroundColor Yellow
        Stop-Service -Name "PrintBridge" -Force -ErrorAction SilentlyContinue
        Start-Sleep -Seconds 2
        Remove-Service -Name "PrintBridge" -Force -ErrorAction SilentlyContinue
        Start-Sleep -Seconds 2
    }
    
    # Create new service
    Write-Host "Creating PrintBridge service..." -ForegroundColor Cyan
    New-Service -Name "PrintBridge" `
                -DisplayName "PrintBridge Print Client" `
                -Description "Silent browser-based print client for Windows" `
                -BinaryPathName $ExePath `
                -StartupType Automatic | Out-Null
    
    # Start the service
    Write-Host "Starting service..." -ForegroundColor Cyan
    Start-Service -Name "PrintBridge" -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 2
    
    # Verify service is running
    $serviceStatus = Get-Service -Name "PrintBridge" | Select-Object -ExpandProperty Status
    
    if ($serviceStatus -eq "Running") {
        Write-Host "✓ PrintBridge service installed and started successfully!" -ForegroundColor Green
        Write-Host "Service will auto-start on system reboot" -ForegroundColor Green
        Write-Host "`nWebSocket URL: wss://127.0.0.1:8282" -ForegroundColor Cyan
    } else {
        Write-Host "⚠ Service was created but failed to start" -ForegroundColor Yellow
        Write-Host "Check Event Viewer for details" -ForegroundColor Yellow
    }
}
catch {
    Write-Host "Error: Failed to install service" -ForegroundColor Red
    Write-Host $_.Exception.Message -ForegroundColor Red
    exit 1
}
