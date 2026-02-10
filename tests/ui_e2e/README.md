# JsonSheet UI E2E

This folder contains Playwright-based UI smoke tests for JsonSheet Desktop.

## Setup

The setup scripts will install dependencies if Node is available:

```powershell
.\scripts\setup.ps1
```

```bash
./scripts/setup.sh
```

## Run (explicit)

```powershell
.\scripts\run-ui-e2e.ps1
```

```bash
./scripts/run-ui-e2e.sh
```

Notes:
- Tests are ignored by default and only run via the helper scripts.
- The app is launched with `JSONSHEET_OPEN` pointing to `tests/data/types.json`.
- WebView2 CDP is enabled via `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222`.
