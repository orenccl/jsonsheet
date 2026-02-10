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
echo "Setting up UI E2E dependencies (optional)..."
if command -v node &> /dev/null && command -v npm &> /dev/null; then
    UI_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../tests/ui_e2e" && pwd)"
    export PLAYWRIGHT_BROWSERS_PATH="$UI_DIR/.playwright"
    pushd "$UI_DIR" > /dev/null

    if [ ! -d "node_modules" ]; then
        npm install
    fi

    npx playwright install chromium
    popd > /dev/null
    echo "[OK] UI E2E dependencies ready"
else
    echo "[SKIP] Node/npm not found. UI E2E setup skipped."
fi

echo ""
echo "=== Setup complete! ==="
echo "Run 'cargo run' to start the app"
echo "Run 'cargo test' to run tests"
