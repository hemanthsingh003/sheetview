# SheetView

A fast, feature-rich terminal spreadsheet viewer and editor written in Rust.

## Features

- **Multiple Format Support**: Open CSV, TSV, and Excel (.xlsx/.xls) files
- **Vim-style Navigation**: Navigate with `h/j/k/l`, arrow keys, and more
- **Search & Replace**: Text search, regex support, column-scoped search, find & replace
- **Sort & Filter**: Sort by columns, filter rows by content
- **Inline Editing**: Edit cells directly in the terminal
- **Row/Column Operations**: Insert, delete, copy rows and columns
- **Undo/Redo**: Full undo/redo support for all edits
- **Multi-sheet Support**: Navigate between Excel sheets
- **Export**: Export to CSV or Excel format
- **Command Palette**: Quick access to commands with `Ctrl+P`
- **Column Statistics**: View min, max, avg, sum, count for selected column
- **Duplicate Detection**: Highlight duplicate rows

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/hemanthsingh003/sheetview.git
cd sheetview

# Build and install
cargo install --path .
```

### Build Release Binary

```bash
cargo build --release
./target/release/sheetview <file>
```

### Pre-built Binary (coming soon)

Download pre-built binaries from the releases page.

## Usage

```bash
# Open a file
sheetview data.csv
sheetview spreadsheet.xlsx

# Specify delimiter for CSV files
sheetview data.csv -d ','

# Limit rows to preview
sheetview data.csv -l 100
```

## Keybindings

### Navigation

| Key             | Action                         |
| --------------- | ------------------------------ |
| `↑` / `k`       | Move up                        |
| `↓` / `j`       | Move down                      |
| `←` / `h`       | Move left                      |
| `→` / `l` / `;` | Move right                     |
| `Ctrl+B`        | Page up                        |
| `Ctrl+F`        | Page down                      |
| `gg`            | First row                      |
| `G`             | Last row (or `10G` for row 10) |
| `0`             | First column                   |
| `$`             | Last column                    |
| `Num + j/k/h/l` | Move by N cells (e.g., `10j`)  |

### Search

| Key | Action                                            |
| --- | ------------------------------------------------- |
| `/` | Start search                                      |
| `n` | Next match                                        |
| `N` | Previous match                                    |
| `R` | Toggle regex mode                                 |
| `c` | Toggle column filter (search current column only) |

### Sort

| Key | Action                                    |
| --- | ----------------------------------------- |
| `s` | Sort by selected column (toggle asc/desc) |
| `R` | Clear sort                                |

### Row/Column Operations

| Key | Action           |
| --- | ---------------- |
| `o` | Insert row below |
| `O` | Insert row above |
| `x` | Delete row       |
| `Y` | Copy entire row  |
| `P` | Paste row        |

### Editing

| Key           | Action    |
| ------------- | --------- |
| `Enter` / `i` | Edit cell |
| `y`           | Copy cell |
| `d`           | Cut cell  |
| `p`           | Paste     |
| `u`           | Undo      |
| `r`           | Redo      |

### Sheets (Excel)

| Key | Action |
|-----|--------|
| `t` | Next sheet |
| `T` | Previous sheet |

### Analysis

| Key | Action |
|-----|--------|
| `U` | Toggle duplicate highlight |

### Actions

| Key             | Action                 |
| --------------- | ---------------------- |
| `?`             | Toggle help            |
| `Ctrl+P`        | Command palette        |
| `Ctrl+S` / `:w` | Save                   |
| `:q`            | Quit (warn if unsaved) |
| `:q!`           | Force quit             |

## Commands

Enter command mode with `:` then type:

| Command               | Description            |
| --------------------- | ---------------------- |
| `:w`                  | Save file              |
| `:q`                  | Quit (warn if unsaved) |
| `:q!`                 | Force quit             |
| `:wq`                 | Save and quit          |
| `:export <path>`      | Export to CSV/Excel    |
| `:replace <query>`    | Find and replace       |
| `:replaceall <query>` | Replace all matches    |
| `:f`                  | Toggle column filter   |

## Development

```bash
# Run in development mode
cargo run -- data.csv

# Run tests
cargo test

# Build for release
cargo build --release

# Lint
cargo clippy
```

## Tech Stack

| Crate           | Version | Purpose                    |
| --------------- | ------- | -------------------------- |
| ratatui         | 0.30    | TUI framework              |
| crossterm       | 0.29    | Terminal backend           |
| clap            | 4.6     | CLI argument parsing       |
| csv             | 1.4     | CSV streaming reads        |
| calamine        | 0.34    | Excel (.xlsx/.xls) reading |
| arboard         | 3.6     | Clipboard support          |
| regex           | 1.11    | Regex search               |
| rust_xlsxwriter | 0.28    | Excel writing              |

## Roadmap

Future planned features:

- Data type detection
- Highlight null/empty cells
- Freeze panes
- Auto-filter dropdowns
- Theming
- Mouse support

## License

MIT License - see [LICENSE](LICENSE) for details.
