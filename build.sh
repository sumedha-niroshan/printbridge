#!/usr/bin/env bash
set -euo pipefail

echo ""
echo "╔══════════════════════════════════════╗"
echo "║   PrintBridge Build Script (Fedora)  ║"
echo "╚══════════════════════════════════════╝"
echo ""

# ── Deps check ────────────────────────────────────────────────────────────────
if ! command -v rustup &>/dev/null; then
  echo "Installing Rust..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  source "$HOME/.cargo/env"
fi

if ! rustup target list --installed | grep -q "x86_64-pc-windows-gnu"; then
  echo "Adding Windows cross-compile target..."
  rustup target add x86_64-pc-windows-gnu
fi

if ! command -v x86_64-w64-mingw32-gcc &>/dev/null; then
  echo "Installing MinGW cross-compiler..."
  sudo dnf install -y mingw64-gcc mingw64-gcc-c++ mingw64-winpthreads-static
fi

if ! command -v node &>/dev/null; then
  echo "Installing Node.js..."
  sudo dnf install -y nodejs npm
fi

# ── SDK build ─────────────────────────────────────────────────────────────────
echo ""
echo "▶ Building TypeScript SDK..."
cd sdk
npm install --silent
npm run build
cd ..
echo "✓ SDK built → sdk/dist/"

# ── Rust build ────────────────────────────────────────────────────────────────
echo ""
echo "▶ Cross-compiling Rust → Windows x64..."

# Configure cargo for mingw linker
mkdir -p .cargo
cat > .cargo/config.toml << 'EOF'
[target.x86_64-pc-windows-gnu]
linker = "x86_64-w64-mingw32-gcc"
ar = "x86_64-w64-mingw32-ar"
EOF

cargo build --release --target x86_64-pc-windows-gnu
echo "✓ Rust built"

# ── Package ───────────────────────────────────────────────────────────────────
echo ""
echo "▶ Packaging release..."
mkdir -p dist/PrintBridge-windows-x64

cp target/x86_64-pc-windows-gnu/release/printbridge.exe  dist/PrintBridge-windows-x64/PrintBridge.exe
cp sdk/dist/printbridge.min.js                            dist/PrintBridge-windows-x64/
cp sdk/dist/printbridge.esm.js                            dist/PrintBridge-windows-x64/
cp scripts/install-cert.ps1                               dist/PrintBridge-windows-x64/
cp scripts/install-service.ps1                            dist/PrintBridge-windows-x64/
cp config.toml                                            dist/PrintBridge-windows-x64/
cp README.md                                              dist/PrintBridge-windows-x64/ 2>/dev/null || true

cd dist
zip -r PrintBridge-windows-x64.zip PrintBridge-windows-x64/
cd ..

echo ""
echo "╔══════════════════════════════════════╗"
echo "║            Build Complete!            ║"
echo "╚══════════════════════════════════════╝"
echo ""
echo "  EXE  → dist/PrintBridge-windows-x64/PrintBridge.exe"
echo "  SDK  → dist/PrintBridge-windows-x64/printbridge.min.js"
echo "  ZIP  → dist/PrintBridge-windows-x64.zip"
echo ""
echo "  On Windows:"
echo "  1. Extract the ZIP"
echo "  2. Run PrintBridge.exe once (generates TLS cert)"
echo "  3. Run install-cert.ps1 as Administrator"
echo "  4. Run install-service.ps1 as Administrator"
echo ""
