#!/usr/bin/env bash
# PXL Linux Startup Script

set -e

# Get the directory of this script
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "=========================================="
echo "          Starting PXL Server             "
echo "=========================================="

# 1. Check and configure USB printer permissions
if [ -e "/dev/usb/lp0" ]; then
    echo "[*] Found USB printer at /dev/usb/lp0"
    
    # Check if we have write access
    if [ ! -w "/dev/usb/lp0" ]; then
        echo "[!] No write permission to /dev/usb/lp0. Attempting to fix with sudo..."
        echo "[!] Please enter your password if prompted to allow printer access."
        sudo chmod 666 /dev/usb/lp0
        echo "[+] Successfully granted write permissions to /dev/usb/lp0"
    else
        echo "[+] Write permission to /dev/usb/lp0 is already OK"
    fi
else
    echo "[!] /dev/usb/lp0 not found. Make sure your USB thermal printer is plugged in and powered on."
fi

# 2. Build and run PXL using Cargo
echo "[*] Building and running PXL..."
cargo run --release
