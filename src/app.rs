use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::data::{open_file, DataSource, DataSourceType};
use crate::ui;

#[derive(Clone)]
pub(crate) struct ColumnStats {
    pub(crate) count: usize,
    pub(crate) null_count: usize,
    pub(crate) numeric_count: usize,
    pub(crate) min: Option<f64>,
    pub(crate) max: Option<f64>,
    pub(crate) sum: f64,
}

impl ColumnStats {
    pub(crate) fn new() -> Self {
        Self {
            count: 0,
            null_count: 0,
            numeric_count: 0,
            min: None,
            max: None,
            sum: 0.0,
        }
    }

    pub(crate) fn add(&mut self, value: &str) {
        self.count += 1;
        if value.is_empty() {
            self.null_count += 1;
            return;
        }
        if let Ok(num) = value.parse::<f64>() {
            self.numeric_count += 1;
            self.sum += num;
            self.min = Some(self.min.map(|m| m.min(num)).unwrap_or(num));
            self.max = Some(self.max.map(|m| m.max(num)).unwrap_or(num));
        }
    }

    pub(crate) fn avg(&self) -> Option<f64> {
        if self.numeric_count > 0 {
            Some(self.sum / self.numeric_count as f64)
        } else {
            None
        }
    }
}

#[derive(Clone)]
enum EditAction {
    CellChange { row: usize, col: usize, old_value: String, new_value: String },
    RowInsert { row: usize, data: Vec<String> },
}

pub struct App {
    data: DataSourceType,
    selected_row: usize,
    selected_col: usize,
    scroll_offset: usize,
    show_help: bool,
    should_quit: bool,
    message: Option<String>,
    last_g_press: Option<Instant>,
    
    search_query: String,
    search_results: Vec<(usize, usize)>,
    current_search_index: usize,
    search_active: bool,
    case_sensitive: bool,
    regex_mode: bool,
    search_column: Option<usize>,
    
    replace_query: String,
    replace_with: String,
    replace_active: bool,
    replace_all_mode: bool,
    replace_cursor: usize,
    
    sort_columns: Vec<(usize, bool)>,
    row_order: Vec<usize>,
    
    filter_active: bool,
    filter_col: Option<usize>,
    filter_values: Vec<String>,
    filter_selected: Vec<bool>,
    filter_cursor: usize,
    filter_mode: bool,
    highlight_duplicates: bool,
    duplicate_rows: Vec<(usize, Vec<usize>)>,
    
    edit_mode: bool,
    edit_buffer: String,
    undo_stack: Vec<EditAction>,
    redo_stack: Vec<EditAction>,
    clipboard: Option<String>,
    clipboard_row: Option<Vec<String>>,
    command_mode: bool,
    command_buffer: String,
    command_palette_mode: bool,
    command_palette_query: String,
    command_palette_cursor: usize,
    force_quit: bool,
    pending_count: Option<usize>,
}

impl App {
    pub fn new(file_path: &std::path::Path, delimiter: Option<char>) -> io::Result<Self> {
        let data = match open_file(file_path, delimiter) {
            Ok(d) => d,
            Err(e) => {
                let msg = e.to_string();
                return Err(io::Error::new(io::ErrorKind::Other, msg));
            }
        };

        let row_count = data.row_count();
        let row_order: Vec<usize> = (0..row_count).collect();

        Ok(Self {
            data,
            selected_row: 0,
            selected_col: 0,
            scroll_offset: 0,
            show_help: false,
            should_quit: false,
            message: None,
            last_g_press: None,
             search_query: String::new(),
             search_results: Vec::new(),
             current_search_index: 0,
             search_active: false,
             case_sensitive: false,
             regex_mode: false,
             search_column: None,
             replace_query: String::new(),
             replace_with: String::new(),
             replace_active: false,
             replace_all_mode: false,
             replace_cursor: 0,
            sort_columns: Vec::new(),
            row_order,
            filter_active: false,
            filter_col: None,
            filter_values: Vec::new(),
            filter_selected: Vec::new(),
            filter_cursor: 0,
            filter_mode: false,
            highlight_duplicates: false,
            duplicate_rows: Vec::new(),
            edit_mode: false,
            edit_buffer: String::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            clipboard: None,
            clipboard_row: None,
            command_mode: false,
            command_buffer: String::new(),
            command_palette_mode: false,
            command_palette_query: String::new(),
            command_palette_cursor: 0,
            force_quit: false,
            pending_count: None,
        })
    }

    pub fn run(&mut self) -> io::Result<()> {
        let mut terminal = ratatui::init();

        loop {
            terminal.draw(|frame| {
                ui::render(frame, self);
            })?;

            if self.should_quit {
                if !self.undo_stack.is_empty() && !self.force_quit {
                    self.message = Some("Warning: Unsaved changes. Use :q! to force quit.".to_string());
                    self.should_quit = false;
                    self.force_quit = false;
                    continue;
                }
                break;
            }

            if let Event::Key(key) = event::read()? {
                self.handle_key(key);
            }
        }

        ratatui::restore();
        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        if self.show_help {
            if key.code == KeyCode::Char('?') || key.code == KeyCode::Esc {
                self.show_help = false;
            }
            return;
        }

        if self.search_active {
            self.handle_search_key(key);
            return;
        }

        if self.replace_active {
            self.handle_replace_key(key);
            return;
        }

        if self.edit_mode {
            self.handle_edit_key(key);
            return;
        }

        if self.command_mode {
            self.handle_command_key(key);
            return;
        }

        if self.filter_mode {
            self.handle_filter_key(key);
            return;
        }

        if self.command_palette_mode {
            self.handle_command_palette_key(key);
            return;
        }

        // Handle number input for count prefix (except 0 which goes to first column)
        if let KeyCode::Char(c) = key.code {
            if c.is_ascii_digit() && c != '0' {
                let digit = c.to_digit(10).unwrap() as usize;
                if let Some(count) = self.pending_count {
                    self.pending_count = Some(count * 10 + digit);
                } else {
                    self.pending_count = Some(digit);
                }
                return;
            }
        }

        match key.code {
            KeyCode::Char('q') => {
                if self.undo_stack.is_empty() {
                    self.should_quit = true;
                }
            }
            KeyCode::Esc => {
                if self.undo_stack.is_empty() {
                    self.should_quit = true;
                }
            }
            KeyCode::Char(':') => {
                self.command_mode = true;
                self.command_buffer.clear();
            }

            KeyCode::Char('?') => {
                self.show_help = true;
            }
            KeyCode::Char('/') => {
                self.search_active = true;
                self.search_query.clear();
                self.search_results.clear();
                self.current_search_index = 0;
                self.regex_mode = false;
                self.search_column = None;
            }
            KeyCode::Char('n') => {
                self.jump_to_next_search_result();
            }
            KeyCode::Char('N') => {
                self.jump_to_prev_search_result();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let count = self.pending_count.unwrap_or(1);
                self.selected_row = self.selected_row.saturating_sub(count);
                self.pending_count = None;
                self.ensure_selection_visible();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let count = self.pending_count.unwrap_or(1);
                let max_row = self.effective_row_count().saturating_sub(1);
                self.selected_row = (self.selected_row + count).min(max_row);
                self.pending_count = None;
                self.ensure_selection_visible();
            }
            KeyCode::Left | KeyCode::Char('h') => {
                let count = self.pending_count.unwrap_or(1);
                self.selected_col = self.selected_col.saturating_sub(count);
                self.pending_count = None;
            }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Char(';') => {
                let count = self.pending_count.unwrap_or(1);
                self.selected_col = (self.selected_col + count).min(self.data.column_count().saturating_sub(1));
                self.pending_count = None;
            }
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.page_up();
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.page_down();
            }
            KeyCode::Char('g') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.selected_row = 0;
                self.scroll_offset = 0;
            }
            KeyCode::Char('g') => {
                let now = Instant::now();
                if let Some(last_time) = self.last_g_press {
                    if now.duration_since(last_time) < Duration::from_millis(500) {
                        self.selected_row = 0;
                        self.scroll_offset = 0;
                        self.last_g_press = None;
                        return;
                    }
                }
                self.last_g_press = Some(now);
            }
            KeyCode::Char('G') => {
                if let Some(count) = self.pending_count {
                    self.selected_row = (count - 1).min(self.effective_row_count().saturating_sub(1));
                } else {
                    self.selected_row = self.effective_row_count().saturating_sub(1);
                }
                self.pending_count = None;
                self.ensure_selection_visible();
            }
            KeyCode::Char('0') => {
                // 0 as count prefix for commands (like 10j), but not for first column
                if let Some(count) = self.pending_count {
                    self.pending_count = Some(count * 10);
                } else {
                    self.selected_col = 0;
                }
            }
            KeyCode::Char('$') => {
                self.selected_col = self.data.column_count().saturating_sub(1);
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.save();
            }
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.command_palette_mode = true;
                self.command_palette_query.clear();
                self.command_palette_cursor = 0;
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.case_sensitive = !self.case_sensitive;
                if !self.search_query.is_empty() {
                    self.perform_search();
                }
            }
            KeyCode::Char('w') => {
                self.save();
            }
            KeyCode::Enter => {
                self.start_edit();
            }
            KeyCode::Char('i') => {
                self.start_edit();
            }
            KeyCode::Char('y') => {
                self.copy_selection();
            }
            KeyCode::Char('d') => {
                self.cut_selection();
            }
            KeyCode::Char('p') => {
                self.paste();
            }
            KeyCode::Char('u') => {
                self.undo();
            }
            KeyCode::Char('r') => {
                self.redo();
            }
            KeyCode::Char('R') => {
                self.clear_sort();
            }
            KeyCode::Char('s') => {
                self.toggle_sort();
            }
            KeyCode::Char('o') => {
                self.insert_row();
            }
            KeyCode::Char('O') => {
                self.insert_row_above();
            }
            KeyCode::Char('x') => {
                self.delete_row();
            }
            KeyCode::Char('Y') => {
                self.copy_row();
            }
            KeyCode::Char('P') => {
                self.paste_row();
            }
            _ => {}
        }
    }

    fn handle_edit_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.edit_mode = false;
                self.edit_buffer.clear();
            }
            KeyCode::Enter => {
                self.commit_edit();
            }
            KeyCode::Backspace => {
                self.edit_buffer.pop();
            }
            KeyCode::Char(c) => {
                self.edit_buffer.push(c);
            }
            _ => {}
        }
    }

    fn handle_command_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.command_mode = false;
                self.command_buffer.clear();
            }
            KeyCode::Enter => {
                self.execute_command();
                self.command_mode = false;
                self.command_buffer.clear();
            }
            KeyCode::Backspace => {
                if self.command_buffer.is_empty() {
                    self.command_mode = false;
                } else {
                    self.command_buffer.pop();
                }
            }
            KeyCode::Char(c) => {
                self.command_buffer.push(c);
            }
            _ => {}
        }
    }

    fn execute_command(&mut self) {
        let cmd = self.command_buffer.trim();
        if cmd == "q!" {
            self.should_quit = true;
            self.force_quit = true;
            return;
        }
        if cmd == "q" {
            self.should_quit = true;
            return;
        }
        match cmd {
            "w" => {
                self.save();
            }
            "wq" => {
                self.save();
                self.should_quit = true;
            }
            "f" => {
                self.toggle_column_filter();
            }
            _ if cmd.starts_with("export ") => {
                let path = cmd.strip_prefix("export ").unwrap().trim().to_string();
                if path.is_empty() {
                    self.message = Some("Usage: export <filepath>".to_string());
                    return;
                }
                self.export_data(&path);
            }
            _ if cmd.starts_with("replace ") => {
                let query = cmd.strip_prefix("replace ").unwrap_or("").trim().to_string();
                if query.is_empty() {
                    self.message = Some("Usage: :replace <query>".to_string());
                    return;
                }
                self.replace_query = query.clone();
                self.search_query = query;
                self.replace_with.clear();
                self.replace_cursor = 1;
                self.replace_all_mode = false;
                self.replace_active = true;
                self.perform_search();
                let count = self.search_results.len();
                self.message = Some(format!("Found {} matches for '{}'. Press n/N to navigate, Enter to replace.", count, self.replace_query));
                return;
            }
            _ if cmd.starts_with("replaceall ") => {
                let query = cmd.strip_prefix("replaceall ").unwrap_or("").trim().to_string();
                if query.is_empty() {
                    self.message = Some("Usage: :replaceall <query>".to_string());
                    return;
                }
                self.replace_query = query.clone();
                self.search_query = query;
                self.replace_with.clear();
                self.replace_cursor = 1;
                self.replace_all_mode = true;
                self.replace_active = true;
                self.perform_search();
                let count = self.search_results.len();
                self.message = Some(format!("Found {} matches for '{}'. Enter to replace all.", count, self.replace_query));
                return;
            }
            _ => {
                self.message = Some(format!("Unknown command: {}", cmd));
            }
        }
    }

    fn export_data(&mut self, path: &str) {
        use std::fs::File;
        
        let path_lower = path.to_lowercase();
        let is_excel = path_lower.ends_with(".xlsx") || path_lower.ends_with(".xls");
        
        if is_excel {
            // Export as Excel using rust_xlsxwriter
            let mut workbook = rust_xlsxwriter::Workbook::new();
            let worksheet = workbook.add_worksheet();
            
            // Write headers
            for (col_idx, header) in self.data.headers().iter().enumerate() {
                let col_u16: u16 = col_idx.try_into().unwrap_or(0);
                let header_row: u32 = 0;
                if let Err(e) = worksheet.write_string(header_row, col_u16, header) {
                    self.message = Some(format!("Error writing header: {}", e));
                    return;
                }
            }
            
            // Write data rows
            for (row_idx, &data_row_idx) in self.row_order.iter().enumerate() {
                let excel_row = (row_idx + 1) as u32; // +1 for header row
                if let Some(row) = self.data.get_row(data_row_idx) {
                    for (col_idx, cell) in row.iter().enumerate() {
                        let col_u16: u16 = col_idx.try_into().unwrap_or(0);
                        if let Err(e) = worksheet.write_string(excel_row as u32, col_u16, cell) {
                            self.message = Some(format!("Error writing cell: {}", e));
                            return;
                        }
                    }
                }
            }
            
            // Save workbook
            if let Err(e) = workbook.save(path) {
                self.message = Some(format!("Error saving Excel file: {}", e));
                return;
            }
        } else {
            // Export as CSV (default behavior)
            let file = match File::create(path) {
                Ok(f) => f,
                Err(e) => {
                    self.message = Some(format!("Error creating file: {}", e));
                    return;
                }
            };
            
            let mut wtr = csv::Writer::from_writer(file);
            
            if let Err(e) = wtr.write_record(self.data.headers()) {
                self.message = Some(format!("Error writing headers: {}", e));
                return;
            }
            
            for &row_idx in &self.row_order {
                if let Some(row) = self.data.get_row(row_idx) {
                    if let Err(e) = wtr.write_record(&row) {
                        self.message = Some(format!("Error writing row: {}", e));
                        return;
                    }
                }
            }
            
            if let Err(e) = wtr.flush() {
                self.message = Some(format!("Error flushing: {}", e));
                return;
            }
        }
        
        self.message = Some(format!("Exported {} rows to {}", self.row_order.len(), path));
    }

    fn handle_command_palette_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.command_palette_mode = false;
                self.command_palette_query.clear();
            }
            KeyCode::Enter => {
                self.execute_palette_command();
                self.command_palette_mode = false;
                self.command_palette_query.clear();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.command_palette_cursor > 0 {
                    self.command_palette_cursor -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let cmds = self.get_palette_commands();
                if self.command_palette_cursor < cmds.len() - 1 {
                    self.command_palette_cursor += 1;
                }
            }
            KeyCode::Backspace => {
                if self.command_palette_query.is_empty() {
                    self.command_palette_mode = false;
                } else {
                    self.command_palette_query.pop();
                    self.command_palette_cursor = 0;
                }
            }
            KeyCode::Char(c) => {
                self.command_palette_query.push(c);
                self.command_palette_cursor = 0;
            }
            _ => {}
        }
    }

    fn get_palette_commands(&self) -> Vec<String> {
        let all_commands = vec![
            "save (Ctrl+S)".to_string(),
            "quit (q/Esc)".to_string(),
            "export (:export <file>)".to_string(),
            "filter (:f)".to_string(),
            "clear filter".to_string(),
            "sort (s)".to_string(),
            "clear sort (R)".to_string(),
            "sheet next (t)".to_string(),
            "sheet prev (T)".to_string(),
            "duplicates (U)".to_string(),
            "help (?)".to_string(),
            "replace (:replace)".to_string(),
            "replace all (:replaceall)".to_string(),
        ];
        
        if self.command_palette_query.is_empty() {
            return all_commands;
        }
        
        // For filtering, we need to check against the base command names
        let base_names = vec![
            "save", "quit", "export", "filter", "clear filter", "sort", 
            "clear sort", "sheet next", "sheet prev", "duplicates", "help",
            "replace", "replace all"
        ];
        
        all_commands.into_iter()
            .enumerate()
            .filter(|(idx, _)| {
                base_names[*idx].contains(&self.command_palette_query.to_lowercase())
            })
            .map(|(_, cmd)| cmd)
            .collect()
    }

    fn execute_palette_command(&mut self) {
        let cmds = self.get_palette_commands();
        if let Some(cmd_str) = cmds.get(self.command_palette_cursor) {
            // Extract base command (everything before the first '(')
            let base_cmd = cmd_str.split('(').next().unwrap_or(cmd_str).trim();
            match base_cmd {
                "save" => self.save(),
                "quit" => self.should_quit = true,
                "export" => self.message = Some("Usage: :export <filepath>".to_string()),
                "filter" => self.toggle_column_filter(),
                "clear filter" => {
                    self.filter_active = false;
                    self.filter_col = None;
                    self.filter_values.clear();
                    self.filter_selected.clear();
                    self.filter_mode = false;
                    let row_count = self.data.row_count();
                    self.row_order = (0..row_count).collect();
                    self.message = Some("Filter cleared".to_string());
                }
                "sort" => self.toggle_sort(),
                "clear sort" => self.clear_sort(),
                "sheet next" => self.next_sheet(),
                "sheet prev" => self.prev_sheet(),
                "duplicates" => self.toggle_duplicate_highlight(),
                "help" => self.show_help = true,
                _ => self.message = Some(format!("Executing: {}", cmd_str)),
            }
        }
    }

    fn start_edit(&mut self) {
        if let Some(cell) = self.data.get_cell(
            self.row_order[self.selected_row],
            self.selected_col
        ) {
            self.edit_mode = true;
            self.edit_buffer = cell;
        }
    }

    fn commit_edit(&mut self) {
        let row = self.row_order[self.selected_row];
        let col = self.selected_col;
        
        let old_value = self.data.get_cell(row, col).unwrap_or_default();
        
        if old_value != self.edit_buffer {
            self.undo_stack.push(EditAction::CellChange {
                row,
                col,
                old_value,
                new_value: self.edit_buffer.clone(),
            });
            self.redo_stack.clear();
            
            self.data.set_cell(row, col, &self.edit_buffer);
            self.message = Some(format!("Edited: {}", self.edit_buffer));
        }
        self.edit_mode = false;
        self.edit_buffer.clear();
    }

    fn copy_selection(&mut self) {
        if let Some(cell) = self.data.get_cell(self.row_order[self.selected_row], self.selected_col) {
            self.clipboard = Some(cell.clone());
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(&cell);
            }
            self.message = Some(format!("Copied: {}", cell));
        } else {
            self.message = Some("Empty cell".to_string());
        }
    }

    fn cut_selection(&mut self) {
        if let Some(cell) = self.data.get_cell(self.row_order[self.selected_row], self.selected_col) {
            let row = self.row_order[self.selected_row];
            let col = self.selected_col;
            
            self.undo_stack.push(EditAction::CellChange {
                row,
                col,
                old_value: cell.clone(),
                new_value: String::new(),
            });
            self.redo_stack.clear();
            
            self.data.set_cell(row, col, "");
            
            self.clipboard = Some(cell.clone());
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(&cell);
            }
            self.message = Some(format!("Cut: {}", cell));
        } else {
            self.message = Some("Empty cell".to_string());
        }
    }

    fn paste(&mut self) {
        if let Some(text) = &self.clipboard {
            let row = self.row_order[self.selected_row];
            let col = self.selected_col;
            let old_value = self.data.get_cell(row, col).unwrap_or_default();
            
            if old_value != *text {
                self.undo_stack.push(EditAction::CellChange {
                    row,
                    col,
                    old_value,
                    new_value: text.clone(),
                });
                self.redo_stack.clear();
                self.data.set_cell(row, col, text);
                self.message = Some(format!("Pasted: {}", text));
            } else {
                self.message = Some("No change to paste".to_string());
            }
        } else {
            self.message = Some("Clipboard empty".to_string());
        }
    }

    fn insert_row(&mut self) {
        // Insert row BELOW current row
        let actual_row = (self.selected_row + 1).min(self.row_order.len());
        self.undo_stack.push(EditAction::RowInsert { row: actual_row, data: vec![] });
        self.redo_stack.clear();
        self.data.insert_row(actual_row);
        
        let row_count = self.data.row_count();
        self.row_order = (0..row_count).collect();
        
        self.message = Some(format!("Inserted row below {}", actual_row + 1));
    }

    fn insert_row_above(&mut self) {
        // Insert row ABOVE current row
        let actual_row = self.row_order[self.selected_row];
        self.undo_stack.push(EditAction::RowInsert { row: actual_row, data: vec![] });
        self.redo_stack.clear();
        self.data.insert_row(actual_row);
        
        let row_count = self.data.row_count();
        self.row_order = (0..row_count).collect();
        
        self.message = Some(format!("Inserted row above {}", actual_row + 1));
    }

    fn delete_row(&mut self) {
        let actual_row = self.row_order[self.selected_row];
        let row_data = self.data.get_row(actual_row).unwrap_or_default();
        self.undo_stack.push(EditAction::RowInsert { row: actual_row, data: row_data });
        self.redo_stack.clear();
        self.data.delete_row(actual_row);
        
        let row_count = self.data.row_count();
        self.row_order = (0..row_count).collect();
        self.selected_row = self.selected_row.min(row_count.saturating_sub(1));
        
        self.message = Some(format!("Deleted row {}", actual_row + 1));
    }

    fn copy_row(&mut self) {
        let actual_row = self.row_order[self.selected_row];
        if let Some(row) = self.data.get_row(actual_row) {
            let row_text = row.join("\t");
            self.clipboard = Some(row_text.clone());
            self.clipboard_row = Some(row.clone());
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(&row_text);
            }
            self.message = Some(format!("Copied row {}: {}", actual_row + 1, row.join(", ")));
        }
    }

    fn paste_row(&mut self) {
        if let Some(ref row_data) = self.clipboard_row {
            let actual_row = self.row_order[self.selected_row];
            
            // For now, paste as a new row below
            self.undo_stack.push(EditAction::RowInsert { row: actual_row + 1, data: row_data.clone() });
            self.redo_stack.clear();
            self.data.insert_row(actual_row + 1);
            
            // Update the row with clipboard data
            for (col_idx, value) in row_data.iter().enumerate() {
                if col_idx < self.data.column_count() {
                    self.data.set_cell(actual_row + 1, col_idx, value);
                }
            }
            
            let row_count = self.data.row_count();
            self.row_order = (0..row_count).collect();
            
            self.message = Some(format!("Pasted row: {}", row_data.join(", ")));
        } else {
            self.message = Some("No row in clipboard".to_string());
        }
    }

    fn undo(&mut self) {
        if let Some(action) = self.undo_stack.pop() {
            match action {
                EditAction::CellChange { row, col, old_value, new_value } => {
                    self.redo_stack.push(EditAction::CellChange {
                        row,
                        col,
                        old_value: old_value.clone(),
                        new_value: new_value.clone(),
                    });
                    self.data.set_cell(row, col, &old_value);
                    self.navigate_to_cell(row, col);
                    self.message = Some(format!("Undo: cell changed to '{}'", old_value));
                }
                EditAction::RowInsert { row, data } => {
                    if data.is_empty() {
                        self.redo_stack.push(EditAction::RowInsert { row, data: vec![] });
                        self.data.delete_row(row);
                        let row_count = self.data.row_count();
                        self.row_order = (0..row_count).collect();
                        self.selected_row = self.selected_row.min(row_count.saturating_sub(1));
                        self.message = Some(format!("Undo: deleted inserted row {}", row + 1));
                    } else {
                        self.redo_stack.push(EditAction::RowInsert { row, data: data.clone() });
                        self.data.insert_row(row);
                        for (col_idx, value) in data.iter().enumerate() {
                            if col_idx < self.data.column_count() {
                                self.data.set_cell(row, col_idx, value);
                            }
                        }
                        let row_count = self.data.row_count();
                        self.row_order = (0..row_count).collect();
                        self.selected_row = row.min(row_count.saturating_sub(1));
                        self.ensure_selection_visible();
                        self.message = Some(format!("Undo: restored row {}", row + 1));
                    }
                }
            }
        } else {
            self.message = Some("Nothing to undo".to_string());
        }
    }

    fn redo(&mut self) {
        if let Some(action) = self.redo_stack.pop() {
            match action {
                EditAction::CellChange { row, col, old_value, new_value } => {
                    // Push what we just redid so we can undo it
                    self.undo_stack.push(EditAction::CellChange {
                        row,
                        col,
                        old_value: old_value.clone(),
                        new_value: new_value.clone(),
                    });
                    // Apply new_value
                    self.data.set_cell(row, col, &new_value);
                    // Navigate to the cell
                    self.navigate_to_cell(row, col);
                    self.message = Some(format!("Redo: cell changed to '{}'", new_value));
                }
                EditAction::RowInsert { row, data } => {
                    self.undo_stack.push(EditAction::RowInsert { row, data: data.clone() });
                    self.data.delete_row(row);
                    let row_count = self.data.row_count();
                    self.row_order = (0..row_count).collect();
                    self.selected_row = self.selected_row.min(row_count.saturating_sub(1));
                    self.message = Some(format!("Redo: deleted row {}", row + 1));
                }
            }
        } else {
            self.message = Some("Nothing to redo".to_string());
        }
    }

    fn navigate_to_cell(&mut self, row: usize, col: usize) {
        // Find the display index for the given actual row
        if let Some(display_idx) = self.row_order.iter().position(|&r| r == row) {
            self.selected_row = display_idx;
            self.selected_col = col;
            self.ensure_selection_visible();
        }
    }

    fn handle_search_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.search_active = false;
                self.search_query.clear();
                self.regex_mode = false;
                self.search_column = None;
            }
            KeyCode::Enter => {
                self.perform_search();
                self.search_active = false;
            }
            KeyCode::Backspace => {
                if self.search_query.is_empty() {
                    self.search_active = false;
                    self.regex_mode = false;
                    self.search_column = None;
                } else {
                    self.search_query.pop();
                    // Check if we removed R from end
                    if self.search_query.ends_with('R') && !self.regex_mode {
                        // Was in regex mode, toggle off
                    } else if self.search_query.ends_with('R') {
                        self.regex_mode = false;
                        self.search_query.pop();
                    }
                }
            }
            KeyCode::Char('R') => {
                // Toggle regex mode
                self.regex_mode = !self.regex_mode;
                if self.regex_mode {
                    self.message = Some("Regex mode: ON".to_string());
                } else {
                    self.message = Some("Regex mode: OFF".to_string());
                }
            }
            KeyCode::Char('f') => {
                // Toggle column filter - search in current column only
                if self.search_column.is_some() {
                    self.search_column = None;
                    self.message = Some("Searching all columns".to_string());
                } else {
                    self.search_column = Some(self.selected_col);
                    self.message = Some(format!("Searching column {} only", self.selected_col + 1));
                }
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
            }
            _ => {}
        }
    }

    fn handle_replace_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.replace_active = false;
                self.replace_query.clear();
                self.replace_with.clear();
                self.replace_cursor = 0;
                self.replace_all_mode = false;
            }
            KeyCode::Enter => {
                if self.replace_with.is_empty() {
                    self.message = Some("Enter replacement text first".to_string());
                    return;
                }
                if self.replace_all_mode {
                    self.replace_all();
                } else {
                    self.replace_current();
                }
                self.replace_active = false;
                self.replace_cursor = 0;
            }
            KeyCode::Tab => {
                // Toggle between find and replace fields
                self.replace_cursor = if self.replace_cursor == 0 { 1 } else { 0 };
            }
            KeyCode::Char('n') => {
                if self.replace_cursor == 0 {
                    self.jump_to_next_search_result();
                } else {
                    self.replace_with.push('n');
                }
            }
            KeyCode::Char('N') => {
                if self.replace_cursor == 0 {
                    self.jump_to_prev_search_result();
                } else {
                    self.replace_with.push('N');
                }
            }
            KeyCode::Backspace => {
                if self.replace_cursor == 0 {
                    if !self.replace_query.is_empty() {
                        self.replace_query.pop();
                        self.search_query = self.replace_query.clone();
                        self.perform_search();
                    }
                } else {
                    if !self.replace_with.is_empty() {
                        self.replace_with.pop();
                    }
                }
            }
            KeyCode::Char(c) => {
                // Input to the active field based on replace_cursor
                if self.replace_cursor == 0 {
                    self.replace_query.push(c);
                    self.search_query = self.replace_query.clone();
                    self.perform_search();
                    let count = self.search_results.len();
                    self.message = Some(format!("Found {} matches", count));
                } else {
                    self.replace_with.push(c);
                }
            }
            _ => {}
        }
    }

    fn perform_search(&mut self) {
        self.search_results.clear();
        if self.search_query.is_empty() {
            return;
        }

        let mut actual_to_display: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
        for (display_idx, &actual_idx) in self.row_order.iter().enumerate() {
            actual_to_display.insert(actual_idx, display_idx);
        }

        // Determine which columns to search
        let columns_to_search: Vec<usize> = if let Some(col) = self.search_column {
            vec![col]
        } else {
            (0..self.data.column_count()).collect()
        };

        // Build regex if in regex mode
        let search_regex = if self.regex_mode {
            let pattern = if self.case_sensitive {
                self.search_query.clone()
            } else {
                format!("(?i){}", self.search_query)
            };
            regex::Regex::new(&pattern).ok()
        } else {
            None
        };

        for &actual_row_idx in &self.row_order {
            for col_idx in &columns_to_search {
                if let Some(cell) = self.data.get_cell(actual_row_idx, *col_idx) {
                    let matches = if self.regex_mode {
                        if let Some(ref re) = search_regex {
                            re.is_match(&cell)
                        } else {
                            false
                        }
                    } else {
                        let cell_text = if self.case_sensitive {
                            cell.clone()
                        } else {
                            cell.to_lowercase()
                        };
                        let query = if self.case_sensitive {
                            self.search_query.clone()
                        } else {
                            self.search_query.to_lowercase()
                        };
                        cell_text.contains(&query)
                    };

                    if matches {
                        let display_idx = actual_to_display[&actual_row_idx];
                        if !self.search_results.iter().any(|(r, _)| *r == display_idx) {
                            self.search_results.push((display_idx, *col_idx));
                        }
                    }
                }
            }
        }

        if !self.search_results.is_empty() {
            // Find the closest match starting from current position
            let current_pos = self.selected_row;
            let mut start_idx = 0;
            for (i, &(row, _)) in self.search_results.iter().enumerate() {
                if row >= current_pos {
                    start_idx = i;
                    break;
                }
                if i == self.search_results.len() - 1 {
                    start_idx = 0;
                }
            }
            
            self.current_search_index = start_idx;
            let (row, col) = self.search_results[start_idx];
            self.selected_row = row;
            self.selected_col = col;
            self.ensure_selection_visible();
        }
        
        let count = self.search_results.len();
        if count > 0 {
            self.message = Some(format!("[{}/{}] Found {} matches", 
                self.current_search_index + 1, 
                count, 
                count));
        } else {
            self.message = Some("No matches found".to_string());
        }
    }

    fn replace_current(&mut self) {
        if self.search_results.is_empty() || self.replace_query.is_empty() {
            return;
        }

        let (row, col) = self.search_results[self.current_search_index];
        let actual_row = self.row_order[row];
        
        if let Some(cell) = self.data.get_cell(actual_row, col) {
            // Check if cell matches our search criteria
            let matches = if self.regex_mode {
                let pattern = if self.case_sensitive {
                    self.replace_query.clone()
                } else {
                    format!("(?i){}", self.replace_query)
                };
                regex::Regex::new(&pattern)
                    .map(|re| re.is_match(&cell))
                    .unwrap_or(false)
            } else {
                let cell_text = if self.case_sensitive {
                    cell.clone()
                } else {
                    cell.to_lowercase()
                };
                let query = if self.case_sensitive {
                    self.replace_query.clone()
                } else {
                    self.replace_query.to_lowercase()
                };
                cell_text.contains(&query)
            };

            if matches {
                // Apply the replacement
                let new_value = if self.regex_mode {
                    let pattern = if self.case_sensitive {
                        self.replace_query.clone()
                    } else {
                        format!("(?i){}", self.replace_query)
                    };
                    regex::Regex::new(&pattern)
                        .map(|re| re.replace_all(&cell, &self.replace_with).to_string())
                        .unwrap_or_else(|_| self.replace_with.clone())
                } else {
                    if self.case_sensitive {
                        cell.replace(&self.replace_query, &self.replace_with)
                    } else {
                        // Case-insensitive replacement
                        let lower_cell = cell.to_lowercase();
                        let lower_query = self.replace_query.to_lowercase();
                        let mut result = String::with_capacity(cell.len());
                        let mut start = 0;
                        while let Some(pos) = lower_cell[start..].find(&lower_query) {
                            let pos = start + pos;
                            result.push_str(&cell[start..pos]);
                            result.push_str(&self.replace_with);
                            start = pos + self.replace_query.len();
                        }
                        result.push_str(&cell[start..]);
                        result
                    }
                };

                // Record the change for undo/redo
                self.undo_stack.push(EditAction::CellChange {
                    row: actual_row,
                    col,
                    old_value: cell.clone(),
                    new_value: new_value.clone(),
                });
                self.redo_stack.clear();
                
                // Actually apply the change
                self.data.set_cell(actual_row, col, &new_value);
                self.message = Some(format!("Replaced: '{}' with '{}'", cell, new_value));
            }
        }
    }

    fn replace_all(&mut self) {
        if self.search_results.is_empty() || self.replace_query.is_empty() {
            return;
        }

        let mut replace_count = 0;
        
        // Determine which columns to search
        let columns_to_search: Vec<usize> = if let Some(col) = self.search_column {
            vec![col]
        } else {
            (0..self.data.column_count()).collect()
        };

        for &actual_row_idx in &self.row_order {
            for col_idx in &columns_to_search {
                if let Some(cell) = self.data.get_cell(actual_row_idx, *col_idx) {
                    // Check if cell matches our search criteria
                    let matches = if self.regex_mode {
                        let pattern = if self.case_sensitive {
                            self.replace_query.clone()
                        } else {
                            format!("(?i){}", self.replace_query)
                        };
                        regex::Regex::new(&pattern)
                            .map(|re| re.is_match(&cell))
                            .unwrap_or(false)
                    } else {
                        let cell_text = if self.case_sensitive {
                            cell.clone()
                        } else {
                            cell.to_lowercase()
                        };
                        let query = if self.case_sensitive {
                            self.replace_query.clone()
                        } else {
                            self.replace_query.to_lowercase()
                        };
                        cell_text.contains(&query)
                    };

                    if matches {
                        // Apply the replacement
                        let new_value = if self.regex_mode {
                            let pattern = if self.case_sensitive {
                                self.replace_query.clone()
                            } else {
                                format!("(?i){}", self.replace_query)
                            };
                            regex::Regex::new(&pattern)
                                .map(|re| re.replace_all(&cell, &self.replace_with).to_string())
                                .unwrap_or_else(|_| self.replace_with.clone())
                        } else {
                            if self.case_sensitive {
                                cell.replace(&self.replace_query, &self.replace_with)
                            } else {
                                // Case-insensitive replacement
                                let lower_cell = cell.to_lowercase();
                                let lower_query = self.replace_query.to_lowercase();
                                let mut result = String::with_capacity(cell.len());
                                let mut start = 0;
                                while let Some(pos) = lower_cell[start..].find(&lower_query) {
                                    let pos = start + pos;
                                    result.push_str(&cell[start..pos]);
                                    result.push_str(&self.replace_with);
                                    start = pos + self.replace_query.len();
                                }
                                result.push_str(&cell[start..]);
                                result
                            }
                        };

                        // Record the change for undo/redo
                        self.undo_stack.push(EditAction::CellChange {
                            row: actual_row_idx,
                            col: *col_idx,
                            old_value: cell.clone(),
                            new_value: new_value.clone(),
                        });
                        
                        // Actually apply the change
                        self.data.set_cell(actual_row_idx, *col_idx, &new_value);
                        replace_count += 1;
                    }
                }
            }
        }
        
        if replace_count > 0 {
            self.redo_stack.clear();
            self.message = Some(format!("Replaced {} occurrences", replace_count));
        } else {
            self.message = Some("No matches found for replace".to_string());
        }
    }

    fn jump_to_next_search_result(&mut self) {
        if self.search_results.is_empty() {
            return;
        }
        self.current_search_index = (self.current_search_index + 1) % self.search_results.len();
        let (row, col) = self.search_results[self.current_search_index];
        self.selected_row = row;
        self.selected_col = col;
        self.ensure_selection_visible();
        let total = self.search_results.len();
        self.message = Some(format!("[{}/{}]", self.current_search_index + 1, total));
    }

    fn jump_to_prev_search_result(&mut self) {
        if self.search_results.is_empty() {
            return;
        }
        if self.current_search_index == 0 {
            self.current_search_index = self.search_results.len() - 1;
        } else {
            self.current_search_index -= 1;
        }
        let (row, col) = self.search_results[self.current_search_index];
        self.selected_row = row;
        self.selected_col = col;
        self.ensure_selection_visible();
        let total = self.search_results.len();
        self.message = Some(format!("[{}/{}]", self.current_search_index + 1, total));
    }

    fn toggle_sort(&mut self) {
        let col = self.selected_col;
        
        // Check if this column is already in the sort
        if let Some(pos) = self.sort_columns.iter().position(|(c, _)| *c == col) {
            // Toggle ascending/descending
            let (_, asc) = self.sort_columns[pos];
            self.sort_columns[pos] = (col, !asc);
        } else {
            // Add to sort columns
            self.sort_columns.push((col, true));
        }
        
        // Perform the sort
        let sort_cols = self.sort_columns.clone();
        self.row_order.sort_by(|&a, &b| {
            for &(col, ascending) in &sort_cols {
                let cell_a = self.data.get_cell(a, col).unwrap_or_default();
                let cell_b = self.data.get_cell(b, col).unwrap_or_default();
                
                let cmp = cell_a.cmp(&cell_b);
                if cmp != std::cmp::Ordering::Equal {
                    return if ascending { cmp } else { cmp.reverse() };
                }
            }
            std::cmp::Ordering::Equal
        });
        
        self.selected_row = 0;
        self.scroll_offset = 0;
        
        let sort_desc: Vec<String> = self.sort_columns.iter()
            .map(|(c, asc)| format!("{}:{}", c + 1, if *asc { "asc" } else { "desc" }))
            .collect();
        self.message = Some(format!("Sorted by [{}]", sort_desc.join(", ")));
    }

    fn clear_sort(&mut self) {
        self.sort_columns.clear();
        let row_count = self.data.row_count();
        self.row_order = (0..row_count).collect();
        self.selected_row = 0;
        self.scroll_offset = 0;
        self.message = Some("Sort cleared".to_string());
    }

    fn toggle_column_filter(&mut self) {
        if self.filter_mode {
            self.filter_mode = false;
            self.message = Some("Filter popup closed".to_string());
            return;
        }

        if self.filter_active {
            self.filter_mode = true;
            self.filter_cursor = 0;
            self.message = Some("Filter mode: SPACE=toggle, Enter=apply, Esc=cancel".to_string());
            return;
        }

        let col = self.selected_col;
        let mut unique_values: std::collections::HashSet<String> = std::collections::HashSet::new();
        
        for &row_idx in &self.row_order {
            if let Some(row) = self.data.get_row(row_idx) {
                if let Some(value) = row.get(col) {
                    if !value.is_empty() {
                        unique_values.insert(value.clone());
                    }
                }
            }
        }
        
        let mut values: Vec<String> = unique_values.into_iter().collect();
        values.sort();
        
        if values.is_empty() {
            self.message = Some("No filter values in column".to_string());
            return;
        }

        self.filter_mode = true;
        self.filter_col = Some(col);
        self.filter_values = values;
        self.filter_selected = vec![true; self.filter_values.len()];
        self.filter_cursor = 0;
        self.message = Some("Filter mode: SPACE=toggle, Enter=apply, Esc=cancel".to_string());
    }

    fn handle_filter_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.filter_mode = false;
                self.message = Some("Filter cancelled".to_string());
            }
            KeyCode::Enter => {
                self.apply_filter();
                self.filter_mode = false;
                let count = self.row_order.len();
                self.message = Some(format!("Filter applied: {} rows", count));
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.filter_cursor > 0 {
                    self.filter_cursor -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.filter_cursor < self.filter_values.len() - 1 {
                    self.filter_cursor += 1;
                }
            }
            KeyCode::Char(' ') => {
                if self.filter_cursor < self.filter_selected.len() {
                    self.filter_selected[self.filter_cursor] = !self.filter_selected[self.filter_cursor];
                }
            }
            KeyCode::Char('a') => {
                self.filter_selected = vec![true; self.filter_values.len()];
                self.message = Some("All values selected".to_string());
            }
            KeyCode::Char('n') => {
                self.filter_selected = vec![false; self.filter_values.len()];
                self.message = Some("All values deselected".to_string());
            }
            KeyCode::Char('c') => {
                self.filter_active = false;
                self.filter_col = None;
                self.filter_values.clear();
                self.filter_selected.clear();
                self.filter_mode = false;
                self.message = Some("Column filter cleared".to_string());
                let row_count = self.data.row_count();
                self.row_order = (0..row_count).collect();
                self.selected_row = 0;
                self.scroll_offset = 0;
            }
            _ => {}
        }
    }

    fn apply_filter(&mut self) {
        let col = self.filter_col.unwrap_or(0);
        
        let selected_values: Vec<&String> = self.filter_values.iter()
            .enumerate()
            .filter(|(i, _)| self.filter_selected[*i])
            .map(|(_, v)| v)
            .collect();
        
        let filter_set: std::collections::HashSet<&String> = selected_values.iter().cloned().collect();
        
        self.row_order.retain(|&row_idx| {
            if let Some(row) = self.data.get_row(row_idx) {
                if let Some(value) = row.get(col) {
                    return filter_set.contains(value);
                }
            }
            false
        });
        
        self.filter_active = self.row_order.len() < self.data.row_count();
        self.selected_row = 0;
        self.scroll_offset = 0;
    }

    fn toggle_duplicate_highlight(&mut self) {
        self.highlight_duplicates = !self.highlight_duplicates;
        if self.highlight_duplicates {
            self.duplicate_rows = self.find_duplicate_rows();
            let count: usize = self.duplicate_rows.iter().map(|(_, rows)| rows.len() - 1).sum();
            self.message = Some(format!("Found {} duplicate rows", count));
        } else {
            self.duplicate_rows.clear();
            self.message = Some("Duplicate highlight cleared".to_string());
        }
    }

    fn next_sheet(&mut self) {
        let sheet_count = self.data.sheet_count();
        if sheet_count > 1 {
            let next = (self.data.current_sheet() + 1) % sheet_count;
            if let Err(e) = self.data.switch_sheet(next) {
                self.message = Some(format!("Error: {}", e));
            } else {
                let row_count = self.data.row_count();
                self.row_order = (0..row_count).collect();
                self.selected_row = 0;
                self.scroll_offset = 0;
            }
        }
    }

    fn prev_sheet(&mut self) {
        let sheet_count = self.data.sheet_count();
        if sheet_count > 1 {
            let prev = if self.data.current_sheet() == 0 {
                sheet_count - 1
            } else {
                self.data.current_sheet() - 1
            };
            if let Err(e) = self.data.switch_sheet(prev) {
                self.message = Some(format!("Error: {}", e));
            } else {
                let row_count = self.data.row_count();
                self.row_order = (0..row_count).collect();
                self.selected_row = 0;
                self.scroll_offset = 0;
            }
        }
    }

    fn effective_row_count(&self) -> usize {
        self.row_order.len()
    }

    fn page_up(&mut self) {
        let rows_visible = 20;
        if self.selected_row > rows_visible {
            self.selected_row -= rows_visible;
        } else {
            self.selected_row = 0;
        }
        self.ensure_selection_visible();
    }

    fn page_down(&mut self) {
        let rows_visible = 20;
        let max_row = self.effective_row_count().saturating_sub(1);
        if self.selected_row + rows_visible < max_row {
            self.selected_row += rows_visible;
        } else {
            self.selected_row = max_row;
        }
        self.ensure_selection_visible();
    }

    fn ensure_selection_visible(&mut self) {
        let visible_rows = 20usize;
        
        if self.selected_row < self.scroll_offset {
            self.scroll_offset = self.selected_row;
        } else if self.selected_row >= self.scroll_offset + visible_rows {
            self.scroll_offset = self.selected_row.saturating_sub(visible_rows - 1);
        }
    }

    fn save(&mut self) {
        match self.data.save() {
            Ok(()) => {
                self.undo_stack.clear();
                self.redo_stack.clear();
                self.message = Some("File saved successfully".to_string());
            }
            Err(e) => {
                self.message = Some(format!("Error saving: {}", e));
            }
        }
    }

    pub fn get_column_stats(&self, col: usize) -> ColumnStats {
        let mut stats = ColumnStats::new();
        for &row_idx in &self.row_order {
            if let Some(row) = self.data.get_row(row_idx) {
                if let Some(value) = row.get(col) {
                    stats.add(value);
                }
            }
        }
        stats
    }

    pub fn find_duplicate_rows(&self) -> Vec<(usize, Vec<usize>)> {
        use std::collections::HashMap;
        let mut seen: HashMap<String, Vec<usize>> = HashMap::new();
        for (display_idx, &actual_idx) in self.row_order.iter().enumerate() {
            if let Some(row) = self.data.get_row(actual_idx) {
                let key = row.join("\t");
                seen.entry(key).or_insert_with(Vec::new).push(display_idx);
            }
        }
        seen.into_iter()
            .filter(|(_, indices)| indices.len() > 1)
            .map(|(_, indices)| (indices[0], indices))
            .collect()
    }

    pub fn data(&self) -> &dyn DataSource {
        &self.data
    }

    pub fn row_count(&self) -> usize {
        self.row_order.len()
    }

    pub fn selected_row(&self) -> usize {
        self.selected_row
    }

    pub fn selected_col(&self) -> usize {
        self.selected_col
    }

    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    pub fn show_help(&self) -> bool {
        self.show_help
    }

    pub fn highlight_duplicates(&self) -> bool {
        self.highlight_duplicates
    }

    pub fn filter_info(&self) -> (bool, bool, Option<usize>, &Vec<String>, &Vec<bool>, usize) {
        (self.filter_active, self.filter_mode, self.filter_col, &self.filter_values, &self.filter_selected, self.filter_cursor)
    }

    pub fn duplicate_rows(&self) -> &[(usize, Vec<usize>)] {
        &self.duplicate_rows
    }

    pub fn sheet_info(&self) -> (usize, usize, Vec<String>) {
        let current = self.data.current_sheet();
        let count = self.data.sheet_count();
        let names = self.data.sheet_names();
        (current, count, names)
    }

    pub fn message(&self) -> Option<&String> {
        self.message.as_ref()
    }

    pub fn search_active(&self) -> bool {
        self.search_active
    }

    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    pub fn edit_mode(&self) -> bool {
        self.edit_mode
    }

    pub fn edit_buffer(&self) -> &str {
        &self.edit_buffer
    }

    pub fn command_mode(&self) -> bool {
        self.command_mode
    }

    pub fn command_buffer(&self) -> &str {
        &self.command_buffer
    }

    pub fn replace_active(&self) -> bool {
        self.replace_active
    }

    pub fn replace_query(&self) -> &str {
        &self.replace_query
    }

    pub fn replace_with(&self) -> &str {
        &self.replace_with
    }

    pub fn replace_all_mode(&self) -> bool {
        self.replace_all_mode
    }

    pub fn command_palette_info(&self) -> (bool, &str, usize, Vec<String>) {
        let cmds = self.get_palette_commands();
        (self.command_palette_mode, &self.command_palette_query, self.command_palette_cursor, cmds)
    }

    pub fn get_display_row(&self, display_idx: usize) -> Option<Vec<String>> {
        let actual_idx = *self.row_order.get(display_idx)?;
        self.data.get_row(actual_idx)
    }
    
    pub fn get_display_cell(&self, display_row: usize, col: usize) -> Option<String> {
        let actual_row = *self.row_order.get(display_row)?;
        self.data.get_cell(actual_row, col).map(|c| c.to_string())
    }
}
