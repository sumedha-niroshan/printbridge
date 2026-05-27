# PXL Print Client — Full Windows Installer
# This script installs PXL as a proper Windows application
# that appears in Control Panel > Programs and Features

param(
    [string]$ExePath = "$PSScriptRoot\..\target\x86_64-pc-windows-gnu\release\pxl.exe",
    [switch]$Uninstall,
    [switch]$InstallService
)

$ErrorActionPreference = "Stop"

# Configuration
$AppName = "PXL Print Client"
$AppVersion = "1.0.0"
$Publisher = "PXL"
$ProgramFilesDir = "C:\Program Files\PXL"
$UninstallerName = "PXLUninstaller.exe"
$UninstallerPath = Join-Path $ProgramFilesDir $UninstallerName

# Registry paths
$RegPath = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\PXL"
$RegPath32 = "HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\PXL"

# Check admin rights
$principal = [Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Error "This installer must run as Administrator."
    exit 1
}

if ($Uninstall) {
    Write-Host "Uninstalling PXL Print Client..." -ForegroundColor Yellow
    
    # Stop service if running
    $service = Get-Service -Name "PXL" -ErrorAction SilentlyContinue
    if ($service) {
        Stop-Service -Name "PXL" -Force -ErrorAction SilentlyContinue
        & sc.exe delete "PXL" | Out-Null
        Write-Host "Service removed." -ForegroundColor Gray
    }
    
    # Remove registry entries
    if (Test-Path $RegPath) {
        Remove-Item -Path $RegPath -Force -ErrorAction SilentlyContinue
    }
    if (Test-Path $RegPath32) {
        Remove-Item -Path $RegPath32 -Force -ErrorAction SilentlyContinue
    }
    
    # Remove Start Menu shortcuts
    $StartMenuPath = Join-Path $env:PROGRAMDATA "Microsoft\Windows\Start Menu\Programs\PXL"
    if (Test-Path $StartMenuPath) {
        Remove-Item -Path $StartMenuPath -Recurse -Force -ErrorAction SilentlyContinue
    }
    
    $UserStartMenuPath = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\PXL"
    if (Test-Path $UserStartMenuPath) {
        Remove-Item -Path $UserStartMenuPath -Recurse -Force -ErrorAction SilentlyContinue
    }
    
    # Remove program files
    if (Test-Path $ProgramFilesDir) {
        Remove-Item -Path $ProgramFilesDir -Recurse -Force -ErrorAction SilentlyContinue
    }
    
    Write-Host "PXL Print Client uninstalled successfully." -ForegroundColor Green
    exit 0
}

# Installation
Write-Host "`n╔════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║  PXL Print Client - Windows Installer  ║" -ForegroundColor Cyan
Write-Host "╚════════════════════════════════════════╝`n" -ForegroundColor Cyan

# Validate executable
if (-not (Test-Path $ExePath)) {
    Write-Error "Executable not found at: $ExePath`nBuild the project first: cargo build --release --target x86_64-pc-windows-gnu"
    exit 1
}

# Create installation directory
Write-Host "[1/5] Creating installation directory..." -ForegroundColor Cyan
if (Test-Path $ProgramFilesDir) {
    Write-Host "      Removing existing installation..." -ForegroundColor Gray
    Remove-Item -Path $ProgramFilesDir -Recurse -Force -ErrorAction SilentlyContinue
}
New-Item -ItemType Directory -Path $ProgramFilesDir -Force | Out-Null
Write-Host "      ✓ Created $ProgramFilesDir" -ForegroundColor Green

# Copy executable and config
Write-Host "[2/5] Copying application files..." -ForegroundColor Cyan
Copy-Item -Path $ExePath -Destination (Join-Path $ProgramFilesDir "pxl.exe") -Force
Copy-Item -Path "$PSScriptRoot\..\config.toml" -Destination (Join-Path $ProgramFilesDir "config.toml") -Force -ErrorAction SilentlyContinue
Copy-Item -Path "$PSScriptRoot\..\README.md" -Destination (Join-Path $ProgramFilesDir "README.md") -Force -ErrorAction SilentlyContinue
Write-Host "      ✓ Files copied" -ForegroundColor Green

# Create App Data directory
Write-Host "[3/5] Initializing application data..." -ForegroundColor Cyan
$AppDataDir = Join-Path $env:APPDATA "PXL"
New-Item -ItemType Directory -Path $AppDataDir -Force | Out-Null
if (-not (Test-Path (Join-Path $AppDataDir "config.toml"))) {
    Copy-Item -Path (Join-Path $ProgramFilesDir "config.toml") -Destination (Join-Path $AppDataDir "config.toml")
}
Write-Host "      ✓ Data directory: $AppDataDir" -ForegroundColor Green

# Create Uninstaller
Write-Host "[4/5] Creating uninstaller..." -ForegroundColor Cyan
$UninstallerScript = @"
# PXL Uninstaller
param([switch]`$Silent)

`$ErrorActionPreference = "Stop"
`$principal = [Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
if (-not `$principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    if (-not `$Silent) { [System.Windows.Forms.MessageBox]::Show("Run as Administrator.", "Error") }
    exit 1
}

# Stop service
Get-Service -Name "PXL" -ErrorAction SilentlyContinue | Stop-Service -Force -ErrorAction SilentlyContinue
& sc.exe delete "PXL" 2>$null

# Remove registry
Remove-Item -Path "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\PXL" -Force -ErrorAction SilentlyContinue
Remove-Item -Path "HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\PXL" -Force -ErrorAction SilentlyContinue

# Remove folders
Remove-Item -Path "$ProgramFilesDir" -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -Path "$$env:APPDATA\Microsoft\Windows\Start Menu\Programs\PXL" -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -Path "$env:PROGRAMDATA\Microsoft\Windows\Start Menu\Programs\PXL" -Recurse -Force -ErrorAction SilentlyContinue

if (-not `$Silent) {
    [System.Windows.Forms.MessageBox]::Show("PXL Print Client has been uninstalled.", "Uninstall Complete")
}
"@

$UninstallerScriptPath = Join-Path $ProgramFilesDir "Uninstall.ps1"
Set-Content -Path $UninstallerScriptPath -Value $UninstallerScript

# Create batch wrapper for uninstaller
$UninstallerBatch = @"
@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$UninstallerScriptPath" -Silent
"@
$UninstallerBatchPath = Join-Path $ProgramFilesDir "Uninstall.bat"
Set-Content -Path $UninstallerBatchPath -Value $UninstallerBatch

Write-Host "      ✓ Uninstaller created" -ForegroundColor Green

# Register in Control Panel (Add/Remove Programs)
Write-Host "[5/5] Registering in Control Panel..." -ForegroundColor Cyan

# Get file info for version
$exe = Get-Item (Join-Path $ProgramFilesDir "pxl.exe")
$fileVersion = $exe.VersionInfo.FileVersion

# Create registry entries
New-Item -Path "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall" -Name "PXL" -Force | Out-Null

$regParams = @{
    Path  = $RegPath
    Force = $true
}

New-ItemProperty @regParams -Name "DisplayName" -Value $AppName -PropertyType String | Out-Null
New-ItemProperty @regParams -Name "DisplayVersion" -Value $AppVersion -PropertyType String | Out-Null
New-ItemProperty @regParams -Name "Publisher" -Value $Publisher -PropertyType String | Out-Null
New-ItemProperty @regParams -Name "UninstallString" -Value "`"$UninstallerBatchPath`"" -PropertyType String | Out-Null
New-ItemProperty @regParams -Name "DisplayIcon" -Value (Join-Path $ProgramFilesDir "pxl.exe") -PropertyType String | Out-Null
New-ItemProperty @regParams -Name "InstallLocation" -Value $ProgramFilesDir -PropertyType String | Out-Null
New-ItemProperty @regParams -Name "NoModify" -Value 1 -PropertyType DWORD | Out-Null
New-ItemProperty @regParams -Name "NoRepair" -Value 1 -PropertyType DWORD | Out-Null
New-ItemProperty @regParams -Name "EstimatedSize" -Value 6400 -PropertyType DWORD | Out-Null

Write-Host "      ✓ Registered in Control Panel" -ForegroundColor Green

# Create Start Menu shortcuts
Write-Host "`nCreating Start Menu shortcuts..." -ForegroundColor Gray
$StartMenuPath = Join-Path $env:PROGRAMDATA "Microsoft\Windows\Start Menu\Programs\PXL"
New-Item -ItemType Directory -Path $StartMenuPath -Force | Out-Null

$WshShell = New-Object -ComObject WScript.Shell
$shortcut = $WshShell.CreateShortcut((Join-Path $StartMenuPath "PXL Print Client.lnk"))
$shortcut.TargetPath = Join-Path $ProgramFilesDir "pxl.exe"
$shortcut.WorkingDirectory = $ProgramFilesDir
$shortcut.Description = "Enterprise Silent Printing Client"
$shortcut.IconLocation = Join-Path $ProgramFilesDir "pxl.exe"
$shortcut.Save()

# Create uninstall shortcut
$uninstallShortcut = $WshShell.CreateShortcut((Join-Path $StartMenuPath "Uninstall.lnk"))
$uninstallShortcut.TargetPath = "cmd.exe"
$uninstallShortcut.Arguments = "/c `"$UninstallerBatchPath`""
$uninstallShortcut.WorkingDirectory = $ProgramFilesDir
$uninstallShortcut.Description = "Uninstall PXL Print Client"
$uninstallShortcut.Save()

# Optional: Install as service
if ($InstallService) {
    Write-Host "`nInstalling as Windows Service..." -ForegroundColor Cyan
    & sc.exe create PXL binPath= "`"$(Join-Path $ProgramFilesDir 'pxl.exe') --service`"" start= auto DisplayName= "PXL Print Service" | Out-Null
    & sc.exe description PXL "Silent browser print client. Provides WebSocket API for web applications to print to Windows printers." | Out-Null
    Write-Host "      ✓ Service installed" -ForegroundColor Green
}

Write-Host "`n╔════════════════════════════════════════╗" -ForegroundColor Green
Write-Host "║    Installation Successful! ✓         ║" -ForegroundColor Green
Write-Host "╚════════════════════════════════════════╝" -ForegroundColor Green

Write-Host "`nℹ️  Application Details:" -ForegroundColor Cyan
Write-Host "   Name:            $AppName"
Write-Host "   Version:         $AppVersion"
Write-Host "   Install Path:    $ProgramFilesDir"
Write-Host "   Config Path:     $AppDataDir"
Write-Host "   Start Menu:      Programs > PXL"
Write-Host ""
Write-Host "🚀 To launch:" -ForegroundColor Cyan
Write-Host "   • Use Start Menu shortcut"
Write-Host "   • Or run: $ProgramFilesDir\pxl.exe"
if ($InstallService) {
    Write-Host "   • Service is running (Manage via Services.msc)"
}

Write-Host "`n✓ The application now appears in:" -ForegroundColor Green
Write-Host "  Control Panel > Programs > Programs and Features" -ForegroundColor Green
