# Run JsonSheet UI E2E (Windows)
$ErrorActionPreference = "Stop"

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$uiDir = Join-Path $root "tests\ui_e2e"

if (!(Get-Command node -ErrorAction SilentlyContinue)) {
    Write-Host "Node is required to run UI E2E" -ForegroundColor Red
    exit 1
}
if (!(Get-Command npm -ErrorAction SilentlyContinue)) {
    Write-Host "npm is required to run UI E2E" -ForegroundColor Red
    exit 1
}

Push-Location $uiDir
$env:PLAYWRIGHT_BROWSERS_PATH = Join-Path $uiDir ".playwright"
if (!(Test-Path "node_modules")) {
    npm install
}

npx playwright install chromium
Pop-Location

Push-Location $root
cargo test --test ui_e2e_tests -- --ignored
Pop-Location
