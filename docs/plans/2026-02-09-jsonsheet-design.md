# JsonSheet Design Document

## Overview

A desktop application that provides an Excel-like spreadsheet interface for editing JSON array files. Built with Dioxus Desktop (Rust) for learning purposes.

## Tech Stack

- **Dioxus Desktop** ??Rust GUI framework, handles UI and application logic
- **serde / serde_json** ??JSON serialization/deserialization
- **rfd** ??Native file dialogs (open/save)

## Architecture

Single Rust application with three-layer structure:

```
????????????????????????????
??  UI Layer (Dioxus)     ?? Table, toolbar, dialogs
????????????????????????????
??  State Layer           ?? Data model, Undo/Redo history, i18n
????????????????????????????
??  IO Layer              ?? JSON file read/write
????????????????????????????
```

### Data Model

- Each JSON file is loaded as `Vec<BTreeMap<String, serde_json::Value>>`
- Columns are derived from the union of all object keys
- Supports full JSON types: string, number, boolean, null

### Key Design Principle

State Layer logic is separated from UI to enable unit testing without GUI dependency.

## Phased Implementation Plan

### Phase 1 ??Basic Table (MVP)

- Open JSON file via native file dialog (rfd)
- Auto-detect columns from JSON keys
- Display data as a table
- Save back to JSON file

### Phase 2 ??Editing

- Click cell to enter edit mode
- Support string, number, boolean, null types
- Add / delete rows
- Add / delete columns

### Phase 3 ??Advanced Operations

- Undo / Redo (command pattern with operation history stack)
- Sort (click column header, toggle ascending/descending)
- Filter (filter displayed rows by column value)
- Search (full-table keyword search, highlight matching cells)

### Phase 4 ??Internationalization (i18n)

- Extract all UI strings to language files
- Default language: English
- Architecture supports adding new languages

## Testing Strategy

### Unit Tests (written in every Phase)

- **IO Layer** ??JSON read/write, malformed input handling
- **State Layer** ??Add/delete rows/columns, sort, filter, search logic
- **Undo/Redo** ??Operation history correctness
- **i18n** ??Language switching, missing key fallback

### E2E Tests (added at each Phase checkpoint)

- Use Dioxus testing utilities or simulated user interactions
- **Phase 1:** Open file ??table displays correctly ??save and verify content matches
- **Phase 2:** Edit cell ??add/delete row/column ??save and verify
- **Phase 3:** Sort order correct, filter results correct, search highlights correct
- **Phase 4:** Language switch updates UI text correctly

## CI/CD (GitHub Actions)

### Trigger

- Push to `main` branch
- All pull requests

### Matrix

- Windows (primary), Linux, macOS

### Pipeline Steps

1. `cargo fmt --check` ??Code formatting
2. `cargo clippy` ??Lint
3. `cargo test` ??Unit tests + E2E tests
4. `cargo build --release` ??Verify compilation

### Future (not in v1)

- Automated release binary packaging
- GitHub Release auto-publish

## Phase 5 -- .jsheet Project File

### Design Principle

**JSON is the single source of truth. `.jsheet` is the view layer.**

JsonSheet's core value is editing JSON with a spreadsheet UI, not replacing Excel. The `.json` file always stays clean and usable on its own.

### File Architecture -- Sidecar Pattern

```
ParkourItemData.json          -- Pure data, always clean JSON
ParkourItemData.json.jsheet   -- Auto-paired metadata (optional)
```

- Opening a `.json` file auto-detects the matching `.json.jsheet` sidecar
- If no `.jsheet` exists, the editor works normally without extra features
- Editing styles/formulas/types auto-saves the `.jsheet` file
- `.jsheet` files can be added to `.gitignore` depending on team preference

### .jsheet Format

JSON-based metadata file containing:

```jsonc
{
  // Column type constraints (enforced at input time in GUI)
  "columns": {
    "hp": { "type": "number" },
    "name": { "type": "string" },
    "active": { "type": "bool" }
  },

  // Computed columns (always baked into JSON on save)
  "computed_columns": {
    "total_hp": {
      "formula": "hp * level * 10"
    }
  },

  // Sidecar-only comments
  "comment_columns": ["note"],
  "comment_rows": [
    { "note": "reviewed by QA" },
    { "note": "" }
  ],

  // Summary statistics (displayed at table footer)
  "summaries": {
    "hp": "AVG",
    "total_hp": "SUM"
  },

  // Column styles
  "styles": {
    "hp": { "color": "#ff4444" },
    "name": { "background": "#f0f0f0" }
  }
}
```

### Column Type Constraints

- Each column can be assigned a type: `string`, `number`, `bool`, `null`
- Type validation is enforced at input time and invalid input is blocked in the GUI
- On save, values are coerced to match the declared type (e.g. `"3"` -> `3` for number columns)

### Computed Columns

- Formula-based columns derived from other columns (column-to-column rules)
- Examples: `damage = base_attack * weapon_multiplier`, `display_name = name + " Lv." + level`
- Computed columns are always baked into JSON on save
- Excel-like editing flow: typing `=` in a cell enters formula mode for that column

### Summary Statistics

- Aggregate calculations displayed at the table footer
- Supported functions: SUM, AVG, COUNT, MIN, MAX
- Applied per-column, works on both data columns and computed columns
- Display-only, never written to JSON

### Styles

- Per-column visual customization (text color, background color)
- Configured directly below column headers (type/summary/comment/color controls)
- Display-only, never affects JSON output

### Comment Columns

- Comment columns are editable in the table like normal columns
- Marking a column as comment stores its values in `.json.jsheet` sidecar only
- Comment columns are excluded from JSON export

### Export Behavior

- Save writes clean JSON with:
  - computed columns baked into output
  - comment columns excluded from output
- Type constraints ensure saved JSON values always match declared types

## Target Platform

- Primary: Windows
- Cross-platform via Dioxus Desktop (Linux, macOS)

