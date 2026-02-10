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

## .jsheet Design Principle

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

### Row Key Anchoring

- `cell_formulas`, `cell_styles`, and `comment_rows` use a designated row key field (e.g. `"id"`) instead of array index
- Prevents data misalignment when rows are added/deleted/reordered outside the editor
- If no suitable key column exists, falls back to row index with a warning

### .jsheet Full Format Reference

```jsonc
{
  "columns": {
    "hp": { "type": "number" },
    "name": { "type": "string" },
    "active": { "type": "bool" }
  },
  "column_order": ["id", "name", "hp", "level", "total_hp", "note"],
  "row_key": "id",
  "keyed_cell_formulas": {
    "1001": { "total_hp": "hp * level * 10" },
    "1002": { "total_hp": "hp * level * 10" }
  },
  "comment_columns": ["note"],
  "keyed_comment_rows": {
    "1001": { "note": "reviewed by QA" },
    "1002": { "note": "" }
  },
  "validation": {
    "hp": { "min": 0, "max": 9999 },
    "rarity": { "enum": ["common", "rare", "epic", "legendary"] }
  },
  "conditional_formats": [
    { "column": "hp", "rule": "< 100", "style": { "color": "#ff0000" } },
    { "column": "rarity", "rule": "== legendary", "style": { "background": "#ffd700" } }
  ],
  "frozen_columns": 2,
  "summaries": {
    "hp": "AVG",
    "total_hp": "SUM"
  },
  "keyed_cell_styles": {
    "1001": { "hp": { "color": "#ff4444" } },
    "1002": { "name": { "background": "#f0f0f0" } }
  }
}
```

### Export Behavior

- Save writes clean JSON with:
  - computed columns baked into output
  - comment columns excluded from output
- Type constraints ensure saved JSON values always match declared types

---

## Phase 5 -- .jsheet Core (Sidecar + Column Types + Column Order)

- [x] Read/write `.json.jsheet` sidecar file alongside `.json`
- [x] Auto-detect sidecar on file open
- [x] Column type constraints (`string`, `number`, `bool`, `null`) with input-time validation
- [x] Column display order stored in `.jsheet`, drag to reorder
- [x] Row key anchoring -- designate a key column for stable row references

## Phase 6 -- Formulas + Summaries

- [x] Cell-level formulas derived from same-row columns (e.g. `=hp * level * 10`)
- [x] Formula UX: type `=` in cell or right-click context menu
- [x] Batch formula: apply/clear formula for a selected range
- [x] Formulas recalculate immediately when dependencies change
- [x] Formulas are always baked into JSON on save
- [x] Summary statistics at table footer (SUM, AVG, COUNT, MIN, MAX)

## Phase 7 -- Styles + Comment Columns

- [x] Per-cell styles (text color, background color) via right-click context menu
- [x] Batch style: apply/clear for selected range
- [x] Conditional formatting rules (e.g. `hp < 100` -> red text)
- [x] Comment columns -- editable in table, stored in `.jsheet` only, excluded from JSON export
- [x] Styles are display-only, never affect JSON output

## Phase 8 -- Data Validation + Freeze Panes

- [ ] Range constraints for numeric columns (min/max)
- [ ] Enum dropdown for columns with fixed options
- [ ] Validation enforced at input time
- [ ] Freeze N left columns during horizontal scrolling

## Phase 9 -- Auto-fill + Multi-sheet

- [ ] Auto-fill drag handle (repeat value, increment numbers, copy formulas)
- [ ] Multi-sheet tab bar -- open multiple JSON files in one window
- [ ] Each tab has its own independent `.jsheet` sidecar

## Target Platform

- Primary: Windows
- Cross-platform via Dioxus Desktop (Linux, macOS)

