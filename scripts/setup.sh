#!/bin/bash
# JsonSheet Development Setup (Linux/macOS)
set -e

echo "=== JsonSheet Setup ==="

# Check Rust
if ! command -v cargo &> /dev/null; then
    echo "[ERROR] Rust not found. Install from: https://www.rust-lang.org/tools/install"
    exit 1
fi
echo "[OK] Rust: $(rustc --version)"

# Install Linux dependencies
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    echo "Installing Linux dependencies..."
    sudo apt-get update
    sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev libxdo-dev
    echo "[OK] Linux dependencies installed"
fi

# Configure git hooks
git config core.hooksPath .githooks
echo "[OK] Git hooks configured"

# Build project
echo "Building project (first time may take a few minutes)..."
cargo build
echo "[OK] Build successful"

echo ""
echo "=== Setup complete! ==="
echo "Run 'cargo run' to start the app"
echo "Run 'cargo test' to run tests"
