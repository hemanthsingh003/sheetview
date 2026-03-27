# SheetView - CSV/Excel TUI Tool

A fast, feature-rich terminal spreadsheet viewer and editor written in Rust.

## Tech Stack

| Crate | Version | Purpose |
|-------|---------|---------|
| ratatui | 0.30 | TUI framework |
| crossterm | 0.29 | Terminal backend |
| clap | 4.6 | CLI argument parsing |
| csv | 1.4 | CSV streaming reads |
| calamine | 0.34 | Excel (.xlsx/.xls) reading |
| arboard | 3.6 | Clipboard support |
| regex | 1.11 | Regex search |

## Keybindings

### Navigation
| Key | Action |
|-----|--------|
| `↑` / `k` | Move up |
| `↓` / `j` | Move down |
| `←` / `h` | Move left |
| `→` / `l` / `;` | Move right |
| `Ctrl+B` | Page up (backward) |
| `Ctrl+F` | Page down (forward) |
| `gg` | First row |
| `G` | Go to row N |
| `0` | First column |
| `$` | Last column |
| `Num + j/k/h/l` | Move by N cells (e.g., 10j) |

### Search
| Key | Action |
|-----|--------|
| `/` | Start search |
| `n` | Next match |
| `N` | Previous match |
| `R` | Toggle regex mode |
| `c` | Toggle column filter (search current column only) |

### Sort
| Key | Action |
|-----|--------|
| `s` | Sort by selected column (toggle asc/desc) |
| `R` | Clear sort |

### Row/Column Operations
| Key | Action |
|-----|--------|
| `o` | Insert row below |
| `O` | Insert row above |
| `x` | Delete row |
| `Y` | Copy entire row |
| `P` | Paste row |

### Editing
| Key | Action |
|-----|--------|
| `Enter` / `i` | Edit cell |
| `y` | Copy cell |
| `d` | Cut cell |
| `p` | Paste |
| `u` | Undo |
| `r` | Redo |

### Actions
| Key | Action |
|-----|--------|
| `?` | Toggle help |
| `Ctrl+S` / `:w` | Save |
| `:q` | Quit (warn if unsaved) |
| `:q!` | Force quit |
| `Ctrl+P` | Command palette |

## Phase Roadmap

### Phase 1 — MVP (Core Viewer) ✅
- Open CSV/TSV/XLSX files
- Scrollable table view (virtual scrolling)
- Column headers, row numbers
- Cell selection with arrow keys + vim keys
- Auto-detect delimiter & encoding
- Column auto-width
- Row/column count display
- Keybinding help (`?`)
- Save

### Phase 2 — Search, Sort, Filter ✅
- Text search
- Regex search
- Column-scoped search
- Sort by column
- Multi-column sort
- Row filter (partial - search acts as filter)

### Phase 3 — Editing ✅
- Inline cell editing
- Insert/delete rows & columns
- Undo/redo
- Copy/cut/paste
- Copy row to clipboard
- Find and Replace (`:replace`, `:replaceall`)

### Phase 4 — Analysis ✅ (COMPLETED)
- Column stats (min, max, avg, sum, count, null count)
- Duplicate row detection

### Phase 5 — Advanced ✅ (COMPLETED)
- Multi-sheet support (XLSX)
- Command palette
- Export filtered data

### Phase 6 — Future (Not Started)
- Data type detection
- Highlight null/empty cells
- Freeze panes
- Auto-filter dropdowns
- Theming
- Mouse support
