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
Write-Host "Setting up UI E2E dependencies (optional)..." -ForegroundColor Yellow
$node = Get-Command node -ErrorAction SilentlyContinue
$npm = Get-Command npm -ErrorAction SilentlyContinue
if ($node -and $npm) {
    $uiDir = Join-Path $PSScriptRoot "..\\tests\\ui_e2e"
    $uiDir = Resolve-Path $uiDir
    Push-Location $uiDir

    $env:PLAYWRIGHT_BROWSERS_PATH = Join-Path $uiDir ".playwright"
    if (!(Test-Path "node_modules")) {
        npm install
        if ($LASTEXITCODE -ne 0) {
            Write-Host "UI E2E npm install failed." -ForegroundColor Red
            Pop-Location
            exit 1
        }
    }

    npx playwright install chromium
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Playwright install failed." -ForegroundColor Red
        Pop-Location
        exit 1
    }

    Pop-Location
    Write-Host "[OK] UI E2E dependencies ready" -ForegroundColor Green
} else {
    Write-Host "[SKIP] Node/npm not found. UI E2E setup skipped." -ForegroundColor Yellow
}

Write-Host ""
Write-Host "=== Setup complete! ===" -ForegroundColor Cyan
Write-Host "Run 'cargo run' to start the app"
Write-Host "Run 'cargo test' to run tests"
