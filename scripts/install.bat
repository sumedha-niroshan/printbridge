@echo off
REM PXL Print Client — Windows Installer
REM Right-click > Run as Administrator

setlocal enabledelayedexpansion

REM Check if running as Administrator
net session >nul 2>&1
if %errorlevel% neq 0 (
    echo.
    echo ERROR: This installer must run as Administrator.
    echo.
    echo Please:
    echo  1. Right-click this file
    echo  2. Select "Run as administrator"
    echo.
    pause
    exit /b 1
)

REM Detect installation mode
set "MODE=install"
if "%~1"=="--uninstall" set "MODE=uninstall"
if "%~1"=="/u" set "MODE=uninstall"

REM Run the PowerShell installer
echo.
if "%MODE%"=="uninstall" (
    echo Uninstalling PXL Print Client...
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0installer.ps1" -Uninstall
) else (
    echo Installing PXL Print Client...
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0installer.ps1"
)

pause
