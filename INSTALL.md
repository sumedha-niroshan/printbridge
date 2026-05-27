# PXL Print Client — Installation Guide

## Quick Install (Recommended)

### Windows 10/11

1. **Navigate to the installation folder**
   - Open the folder where you extracted the PXL application files
   - Go to `scripts` folder

2. **Run the installer**
   - Right-click on `install.bat`
   - Select **"Run as administrator"**
   - Windows may ask for permission — click **"Yes"**

3. **Wait for completion**
   - The installer will:
     - Copy files to `C:\Program Files\PXL`
     - Create Start Menu shortcuts
     - Register in Control Panel (Add/Remove Programs)
     - Setup the application

4. **Launch the application**
   - Go to **Start Menu** → **PXL** → **PXL Print Client**
   - Or search for "PXL" in Windows Search

## Installation Options

### Install with Service (Background Mode)

If you want PXL to run as a background Windows Service:

```cmd
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "scripts\installer.ps1" -InstallService
```

Then:

- Service will start automatically with Windows
- Open `Services.msc` to manage it
- Can run GUI independently by clicking Start Menu shortcut

### Manual Installation (PowerShell)

If the batch file doesn't work, run directly:

```powershell
# As Administrator
Set-ExecutionPolicy -ExecutionPolicy Bypass -Scope Process -Force
& "C:\path\to\scripts\installer.ps1"
```

## Uninstallation

### Method 1: Control Panel (Easiest)

1. Open **Settings** → **Apps** → **Apps & Features**
2. Search for **"PXL"**
3. Click **Uninstall**

### Method 2: Installer Script

Run in PowerShell (as Administrator):

```powershell
& "C:\Program Files\PXL\Uninstall.bat"
```

Or double-click the uninstall shortcut in Start Menu:

- **Start Menu** → **PXL** → **Uninstall**

## After Installation

### Configuration

- Configuration file location: `%APPDATA%\PXL\config.toml`
- Edit to change:
  - WebSocket port (default: 8282)
  - CORS allowed origins
  - Logging level

### Verify Installation

1. Open Control Panel → Programs → Programs and Features
2. Look for **"PXL Print Client"** in the list
3. Click to see:
   - Installation location
   - Version number
   - Publisher info
   - Uninstall button

### First Run

1. Launch from Start Menu
2. **Select printer** from dropdown
3. Click **"Refresh Printers"** to update list
4. Click **"Run Test Print"** to verify it works

## WebSocket Connection

Once installed and running:

**URL:** `wss://localhost:8282`

**Example JavaScript:**

```javascript
const ws = new WebSocket("wss://localhost:8282");

ws.onmessage = (event) => {
  console.log("Server response:", event.data);
};

ws.send(
  JSON.stringify({
    type: "print",
    printer: "HP Officejet Pro",
    content: "data:image/png;base64,...",
  }),
);
```

## Troubleshooting

### "Run as administrator" not working?

- Right-click the script file
- Properties → General → Check "Unblock"
- Click "Apply" → "OK"
- Then try again

### Installer hangs?

- Close the window (Ctrl+C)
- Check if `pxl.exe` exists at: `target\x86_64-pc-windows-gnu\release\pxl.exe`
- Rebuild if needed: `cargo build --release --target x86_64-pc-windows-gnu`

### Application won't start?

- Ensure .NET Framework 3.5+ is installed
- Check Windows Firewall allows the app
- View logs in: `%APPDATA%\PXL\`

### Service won't start?

- Open `Services.msc`
- Right-click "PXL Print Service"
- Click "Start"
- Check error logs

## System Requirements

- **Windows 10 or later** (64-bit)
- **Administrator access** for installation
- At least one installed printer
- ~10 MB disk space

## File Locations After Install

```
C:\Program Files\PXL\
  ├── pxl.exe (main application)
  ├── config.toml (default config)
  ├── Uninstall.bat
  └── README.md

%APPDATA%\PXL\
  └── config.toml (user configuration)
```

## Support

For issues or questions:

- Check the README.md file in installation directory
- Review logs in `%APPDATA%\PXL\`
- Verify printer connectivity
