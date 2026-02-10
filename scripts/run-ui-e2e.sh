#!/bin/bash
set -e

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
UI_DIR="$ROOT_DIR/tests/ui_e2e"

if ! command -v node &> /dev/null; then
  echo "Node is required to run UI E2E" >&2
  exit 1
fi
if ! command -v npm &> /dev/null; then
  echo "npm is required to run UI E2E" >&2
  exit 1
fi

export PLAYWRIGHT_BROWSERS_PATH="$UI_DIR/.playwright"

pushd "$UI_DIR" > /dev/null
if [ ! -d "node_modules" ]; then
  npm install
fi
npx playwright install chromium
popd > /dev/null

pushd "$ROOT_DIR" > /dev/null
cargo test --test ui_e2e_tests -- --ignored
popd > /dev/null
