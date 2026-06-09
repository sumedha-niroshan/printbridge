# PrintBridge

A modern, silent browser-based print client for Windows with ESC/POS thermal printer support. PrintBridge enables web applications to send print jobs directly to printers without showing print dialogs.

## Features

- **Silent Printing**: Print without user interaction or print dialogs
- **WebSocket Communication**: Real-time bidirectional communication between browser and desktop client
- **Thermal Printer Support**: Full ESC/POS command support for thermal printers
- **Cross-Domain**: Works seamlessly across different web origins
- **Secure**: TLS/SSL encrypted WebSocket connections with certificate management
- **Browser SDK**: Easy-to-use TypeScript/JavaScript SDK for web applications
- **Service Installation**: Windows service support for automatic startup and reliability

## Architecture

PrintBridge consists of two main components:

### 1. Desktop Client (Rust)

A Windows desktop application that:

- Communicates with web browsers via secure WebSocket
- Manages printer connections and ESC/POS commands
- Handles certificate generation and TLS encryption
- Runs as a Windows service for background operation
- Provides a UI for configuration and monitoring

**Key modules:**

- `printer.rs` - Printer device management
- `websocket.rs` - WebSocket server and message handling
- `protocol.rs` - ESC/POS protocol implementation
- `cert.rs` - TLS certificate generation and management
- `config.rs` - Configuration file handling
- `gui.rs` - System tray and UI components
- `service.rs` - Windows service integration

### 2. Browser SDK (TypeScript)

A lightweight JavaScript library that:

- Provides simple API for web applications
- Handles WebSocket connection to desktop client
- Manages print job queuing and retry logic
- Supports TypeScript with full type definitions

**Usage:**

```typescript
import { PrintClient } from "printbridge-sdk";

const client = new PrintClient("ws://localhost:9100");

// Connect to the desktop client
await client.connect();

// Send print data
await client.print({
  data: Buffer.from("\x1b@\x1b\x45\x01Hello World\x0a"),
  printerName: "THERMAL_PRINTER",
});
```

## Getting Started

### Prerequisites

- Windows 7 or later
- Node.js 16+ (for SDK development)
- Rust 1.70+ (for building from source)

### Installation

1. **Download and Install Desktop Client**

   ```bash
   # Run the installer
   pxl-installer.exe
   ```

2. **Install Browser SDK**
   ```bash
   npm install printbridge-sdk
   ```

### Quick Start

#### Desktop Client Configuration

1. The desktop client runs as a Windows service after installation
2. Default WebSocket server runs on `ws://localhost:9100`
3. Configuration file: `%APPDATA%\PXL\config.toml`

#### Web Application Integration

```html
<!DOCTYPE html>
<html>
  <head>
    <script src="dist/printbridge.min.js"></script>
  </head>
  <body>
    <button onclick="printText()">Print</button>

    <script>
      const client = new PrintBridge("ws://localhost:9100");

      async function printText() {
        try {
          await client.connect();
          const data = new Uint8Array([
            0x1b,
            0x40, // Initialize printer
            0x1b,
            0x45,
            0x01, // Enable bold
            // Print "Hello"
            0x48,
            0x65,
            0x6c,
            0x6c,
            0x6f,
            0x0a,
          ]);
          await client.print({
            data: data,
            printerName: "YOUR_PRINTER",
          });
        } catch (error) {
          console.error("Print failed:", error);
        }
      }
    </script>
  </body>
</html>
```

## Development

### Building from Source

#### Desktop Client (Rust)

```bash
# Install dependencies
cargo build --release

# Run in development mode
cargo run

# Build installer
cargo build --release --bin installer
```

#### Browser SDK

```bash
cd sdk

# Install dependencies
npm install

# Build all formats (IIFE, ESM, CommonJS)
npm run build

# Watch mode for development
npm run dev
```

### Project Structure

```
printbridge/
├── src/                    # Rust desktop client source
│   ├── main.rs            # Application entry point
│   ├── printer.rs         # Printer device management
│   ├── websocket.rs       # WebSocket server implementation
│   ├── protocol.rs        # ESC/POS protocol
│   ├── cert.rs            # TLS certificate handling
│   ├── config.rs          # Configuration management
│   ├── gui.rs             # GUI and system tray
│   ├── service.rs         # Windows service integration
│   └── bin/
│       └── installer.rs   # Windows installer executable
├── sdk/                    # TypeScript/JavaScript SDK
│   ├── src/
│   │   ├── index.ts       # SDK entry point
│   │   ├── PrintClient.ts # Main PrintClient class
│   │   └── types.ts       # TypeScript type definitions
│   └── package.json
├── icons/                  # Application icons
├── logo/                   # Project logos
├── .github/
│   └── workflows/         # CI/CD workflows
├── Cargo.toml             # Rust project manifest
├── build.rs               # Rust build script
└── README.md              # This file
```

### Configuration

The desktop client reads configuration from `config.toml`:

```toml
[server]
host = "0.0.0.0"
port = 9100

[security]
tls_enabled = true
cert_file = "certs/server.crt"
key_file = "certs/server.key"

[printer]
default_printer = ""

[service]
auto_start = true
```

### Running Tests

```bash
# Run Rust tests
cargo test

# Run SDK tests
cd sdk && npm test
```

## API Reference

### PrintClient (SDK)

#### Methods

- **`connect(): Promise<void>`**
  - Establishes connection to desktop client
- **`disconnect(): Promise<void>`**
  - Closes WebSocket connection

- **`print(options: PrintOptions): Promise<void>`**
  - Sends print job to specified printer
  - Options: `data` (Uint8Array), `printerName` (string), `timeout` (number)

- **`getPrinters(): Promise<string[]>`**
  - Retrieves list of available printers

- **`getStatus(): Promise<PrinterStatus>`**
  - Gets current printer status

#### Events

- `connected` - Emitted when connected to desktop client
- `disconnected` - Emitted when disconnected
- `error` - Emitted on error
- `printerStatusChanged` - Emitted when printer status changes

## Security Considerations

- **TLS Encryption**: All WebSocket connections are encrypted by default
- **Certificate Management**: Auto-generated self-signed certificates
- **CORS Support**: Configurable cross-origin requests
- **Local Only**: Desktop client only listens on localhost by default for security

## Troubleshooting

### Connection Issues

1. Ensure desktop client is running (check Services)
2. Verify WebSocket port is accessible (default: 9100)
3. Check Windows Firewall settings

### Printer Not Found

1. Verify printer is installed and ready in Windows Settings
2. Check printer name in configuration
3. Test printer connectivity with Windows print dialog

### Print Jobs Failing

1. Verify ESC/POS command format
2. Check printer error logs in UI
3. Test with simple initialization sequence

## Contributing

We welcome contributions! Please follow these steps:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Development Guidelines

- Follow Rust naming conventions and use `cargo fmt`
- TypeScript code should pass `npm run lint`
- Add tests for new features
- Update documentation for API changes
- Keep commits atomic and write descriptive messages

## Roadmap

- [ ] Cross-platform support (macOS, Linux)
- [ ] USB printer support
- [ ] Print preview functionality
- [ ] Advanced scheduling and queuing
- [ ] Printer health monitoring
- [ ] Web UI for configuration
- [ ] Python and Go SDK variants

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Support

For issues, feature requests, or questions:

- Open an issue on GitHub
- Check existing documentation
- Review troubleshooting section above

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for version history and updates.

## Acknowledgments

- Built with Rust, Tokio, and egui
- Thanks to the open-source community for excellent libraries

---

**PrintBridge** - Making silent printing on Windows simple and reliable.
