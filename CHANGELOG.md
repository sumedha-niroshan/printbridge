# Changelog

All notable changes to PrintBridge will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial public release

### Changed

- N/A

### Deprecated

- N/A

### Removed

- N/A

### Fixed

- N/A

### Security

- N/A

## [1.0.0] - 2024-06-09

### Added

- Silent printing functionality for Windows
- ESC/POS thermal printer support
- WebSocket-based browser communication
- TLS/SSL encrypted connections
- Browser SDK (TypeScript/JavaScript)
- Windows Service integration
- System tray UI
- Configuration file support
- Certificate auto-generation
- Multiple printer support
- Print job queuing

### Features

- **Desktop Client (Rust)**
  - Multi-threaded async architecture
  - Windows Service support
  - Real-time UI status monitoring
  - Printer device management
  - TLS certificate generation

- **Browser SDK**
  - TypeScript type definitions
  - Multiple format exports (IIFE, ESM, CommonJS)
  - Auto-reconnection logic
  - Error handling and logging
  - Cross-origin support

- **Installation**
  - Automated installer executable
  - PowerShell installation scripts
  - Certificate management scripts
  - Configuration templates

### Security

- Self-signed TLS certificates
- Secure WebSocket (WSS) connections
- Localhost-only default binding
- Configurable CORS policies

---

## Version Template (for future releases)

```markdown
## [X.Y.Z] - YYYY-MM-DD

### Added

- New features

### Changed

- Changes in existing functionality

### Deprecated

- Features that will be removed

### Removed

- Removed features

### Fixed

- Bug fixes

### Security

- Security fixes and improvements
```

## Notes

- All dates in YYYY-MM-DD format
- Link style: [version] at bottom for unreleased, with date for released
- Keep latest version at the top
- Document breaking changes clearly

---

For more details, see the [GitHub Releases](https://github.com/printbridge/printbridge/releases) page.
