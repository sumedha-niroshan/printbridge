@echo off
REM ═══════════════════════════════════════
REM   PXL Print Client — Windows Build
REM ═══════════════════════════════════════

echo.
echo ╔══════════════════════════════════════╗
echo ║     PXL Print Client — Build        ║
echo ╚══════════════════════════════════════╝
echo.

REM Check for Rust
where cargo >nul 2>nul
if %ERRORLEVEL% neq 0 (
    echo [ERROR] Rust/Cargo not found!
    echo Install from: https://rustup.rs
    pause
    exit /b 1
)

REM Build release binary
echo [1/3] Building PXL release binary...
cargo build --release
if %ERRORLEVEL% neq 0 (
    echo [ERROR] Build failed!
    pause
    exit /b 1
)
echo       Done.

REM Create dist folder
echo [2/3] Packaging distribution...
if not exist dist\PXL mkdir dist\PXL

copy /Y target\release\pxl.exe         dist\PXL\PXL.exe
copy /Y config.toml                     dist\PXL\config.toml
copy /Y scripts\install.ps1            dist\PXL\install.ps1
copy /Y scripts\install-cert.ps1       dist\PXL\install-cert.ps1
copy /Y README.md                       dist\PXL\README.md 2>nul

echo       Done.

REM Summary
echo [3/3] Build complete!
echo.
echo ╔══════════════════════════════════════╗
echo ║          Build Complete!             ║
echo ╚══════════════════════════════════════╝
echo.
echo   EXE    : dist\PXL\PXL.exe
echo   Config : dist\PXL\config.toml
echo   Install: dist\PXL\install.ps1
echo.
echo   To deploy on a Windows machine:
echo   1. Copy the dist\PXL folder to the target PC
echo   2. Run install.ps1 in PowerShell
echo   3. Run install-cert.ps1 as Administrator
echo   4. Double-click PXL.exe to start
echo.
pause
