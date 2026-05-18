# PrintBridge

A silent browser print client for Windows — like QZ Tray, built with **Rust**.

Runs as a local Windows Service, exposes a secure WebSocket (`wss://127.0.0.1:8282`), and lets any web page send jobs to ESC/POS thermal printers without browser print dialogs.

## Architecture

```
Browser (JS SDK)  ──wss://127.0.0.1:8282──►  PrintBridge.exe (Windows Service)
                                                      │
                                              Windows Print Spooler
                                                      │
                                              Thermal / USB / LAN printer
```

## Quick Start (Windows)

```powershell
# 1. Run once to generate TLS cert
.\PrintBridge.exe

# 2. Trust the cert (run as Administrator)
.\install-cert.ps1

# 3. Install as Windows Service (run as Administrator)
.\install-service.ps1
```

## Browser SDK

### Script tag
```html
<script src="printbridge.min.js"></script>
<script>
  const client = new PrintBridge.PrintClient();
  await client.connect();
  const printers = await client.listPrinters();
  await client.print({
    printer: printers[0].name,
    type: 'escpos',
    data: btoa(escposBytes),  // base64-encoded ESC/POS bytes
    copies: 1
  });
</script>
```

### NPM / ESM
```js
import { PrintClient } from './printbridge.esm.js';

const client = new PrintClient({ url: 'wss://127.0.0.1:8282' });
await client.connect();
```

## Build from Source (Fedora)

```bash
chmod +x build.sh
./build.sh
# Output: dist/PrintBridge-windows-x64.zip
```

## Message Protocol

All messages are JSON over WebSocket.

### listPrinters
```json
// Request
{ "action": "listPrinters", "id": "req1" }

// Response
{ "id": "req1", "success": true, "data": {
    "printers": [
      { "name": "EPSON TM-T20III", "isDefault": true, "isOnline": true }
    ]
}}
```

### print
```json
// Request
{ "action": "print", "id": "req2", "payload": {
    "printer": "EPSON TM-T20III",
    "type": "escpos",
    "data": "<base64-encoded bytes>",
    "copies": 1
}}

// Response
{ "id": "req2", "success": true, "data": { "printed": true, "copies": 1 }}
```

### status / ping
```json
{ "action": "status", "id": "req3" }
{ "action": "ping",   "id": "req4" }
```

## Configuration

Edit `config.toml` in `%APPDATA%\PrintBridge\`:

```toml
[server]
port = 8282
host = "127.0.0.1"
allowed_origins = ["https://yourapp.com", "http://localhost:3000"]

[logging]
level = "info"  # trace | debug | info | warn | error
```

## CI/CD

Every push to `main` → builds Windows `.exe` → downloadable from GitHub Actions tab.  
`git push tag v1.0.0` → creates a GitHub Release with the ZIP attached.

## License

MIT
