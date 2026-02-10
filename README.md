# JsonSheet

A desktop application that provides a spreadsheet-like interface for editing JSON array files. Built with Rust and Dioxus.

## Features

- Open JSON files via native file dialog
- Auto-detect columns from JSON keys
- Display data as a table with sortable columns
- Save back to JSON file

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (1.70+)
- **Windows**: Visual Studio Build Tools with "Desktop development with C++"
- **Linux**: `libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev libxdo-dev`
- **macOS**: Xcode Command Line Tools

## Setup

**Windows:**
```powershell
.\scripts\setup.ps1
```

**Linux / macOS:**
```bash
./scripts/setup.sh
```

## UI E2E (optional)

UI E2E tests are ignored by default. Use the helper scripts to run them:

**Windows:**
```powershell
.\scripts\run-ui-e2e.ps1
```

**Linux / macOS:**
```bash
./scripts/run-ui-e2e.sh
```

## Usage

```bash
cargo run        # Start the app
cargo test       # Run tests
cargo clippy     # Lint check
cargo fmt        # Format code
```

## Project Structure

```
src/
├── main.rs              # Entry point, window config
├── lib.rs               # Public module exports
├── io/json_io.rs        # JSON file read/write
├── state/data_model.rs  # Row, TableData, derive_columns
└── ui/
    ├── app.rs           # Root component
    ├── toolbar.rs       # Open/Save buttons
    └── table.rs         # Table display
```

## License

MIT
