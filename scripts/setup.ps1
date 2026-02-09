# JsonSheet Development Setup (Windows)
Write-Host "=== JsonSheet Setup ===" -ForegroundColor Cyan

# Check Rust
if (!(Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "Rust not found. Install from: https://www.rust-lang.org/tools/install" -ForegroundColor Red
    exit 1
}
Write-Host "[OK] Rust: $(rustc --version)" -ForegroundColor Green

# Configure git hooks
git config core.hooksPath .githooks
Write-Host "[OK] Git hooks configured" -ForegroundColor Green

# Build project (downloads dependencies)
Write-Host "Building project (first time may take a few minutes)..." -ForegroundColor Yellow
cargo build
if ($LASTEXITCODE -ne 0) {
    Write-Host "Build failed. Check error messages above." -ForegroundColor Red
    exit 1
}
Write-Host "[OK] Build successful" -ForegroundColor Green

Write-Host ""
Write-Host "=== Setup complete! ===" -ForegroundColor Cyan
Write-Host "Run 'cargo run' to start the app"
Write-Host "Run 'cargo test' to run tests"
