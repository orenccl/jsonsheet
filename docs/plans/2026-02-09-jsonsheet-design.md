# JsonSheet Design Document

## Overview

A desktop application that provides an Excel-like spreadsheet interface for editing JSON array files. Built with Dioxus Desktop (Rust) for learning purposes.

## Tech Stack

- **Dioxus Desktop** — Rust GUI framework, handles UI and application logic
- **serde / serde_json** — JSON serialization/deserialization
- **rfd** — Native file dialogs (open/save)

## Architecture

Single Rust application with three-layer structure:

```
┌─────────────────────────┐
│   UI Layer (Dioxus)     │  Table, toolbar, dialogs
├─────────────────────────┤
│   State Layer           │  Data model, Undo/Redo history, i18n
├─────────────────────────┤
│   IO Layer              │  JSON file read/write
└─────────────────────────┘
```

### Data Model

- Each JSON file is loaded as `Vec<BTreeMap<String, serde_json::Value>>`
- Columns are derived from the union of all object keys
- Supports full JSON types: string, number, boolean, null

### Key Design Principle

State Layer logic is separated from UI to enable unit testing without GUI dependency.

## Phased Implementation Plan

### Phase 1 — Basic Table (MVP)

- Open JSON file via native file dialog (rfd)
- Auto-detect columns from JSON keys
- Display data as a table
- Save back to JSON file

### Phase 2 — Editing

- Click cell to enter edit mode
- Support string, number, boolean, null types
- Add / delete rows
- Add / delete columns

### Phase 3 — Advanced Operations

- Undo / Redo (command pattern with operation history stack)
- Sort (click column header, toggle ascending/descending)
- Filter (filter displayed rows by column value)
- Search (full-table keyword search, highlight matching cells)

### Phase 4 — Internationalization (i18n)

- Extract all UI strings to language files
- Default language: English
- Architecture supports adding new languages

## Testing Strategy

### Unit Tests (written in every Phase)

- **IO Layer** — JSON read/write, malformed input handling
- **State Layer** — Add/delete rows/columns, sort, filter, search logic
- **Undo/Redo** — Operation history correctness
- **i18n** — Language switching, missing key fallback

### E2E Tests (added at each Phase checkpoint)

- Use Dioxus testing utilities or simulated user interactions
- **Phase 1:** Open file → table displays correctly → save and verify content matches
- **Phase 2:** Edit cell → add/delete row/column → save and verify
- **Phase 3:** Sort order correct, filter results correct, search highlights correct
- **Phase 4:** Language switch updates UI text correctly

## CI/CD (GitHub Actions)

### Trigger

- Push to `main` branch
- All pull requests

### Matrix

- Windows (primary), Linux, macOS

### Pipeline Steps

1. `cargo fmt --check` — Code formatting
2. `cargo clippy` — Lint
3. `cargo test` — Unit tests + E2E tests
4. `cargo build --release` — Verify compilation

### Future (not in v1)

- Automated release binary packaging
- GitHub Release auto-publish

## Target Platform

- Primary: Windows
- Cross-platform via Dioxus Desktop (Linux, macOS)
