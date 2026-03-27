mod table;

use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;

pub fn render(frame: &mut Frame, app: &mut App) {
    let search_active = app.search_active();
    let help_active = app.show_help();
    let edit_active = app.edit_mode();
    let command_active = app.command_mode();
    let replace_active = app.replace_active();
    let (_, filter_mode, _, _, _, _) = app.filter_info();
    let palette_query: String;
    let palette_cursor: usize;
    let palette_commands: Vec<String>;
    let palette_active: bool;
    {
        let palette_info = app.command_palette_info();
        palette_active = palette_info.0;
        palette_query = palette_info.1.to_string();
        palette_cursor = palette_info.2;
        palette_commands = palette_info.3;
    }

    let status_height = 3u16;
    let input_height = 3u16;
    let help_height = if help_active { 30u16 } else { 0 };
    let filter_height = if filter_mode { 15u16 } else { 0 };

    let terminal_width = frame.area().width;
    let terminal_height = frame.area().height;

    let mut areas = Vec::new();

    if help_active {
        let help_area = Rect {
            x: 0,
            y: 0,
            width: terminal_width,
            height: help_height,
        };
        areas.push(("help", help_area));
    } else {
        let total_input = (search_active as u16) * input_height
            + (edit_active as u16) * input_height
            + (command_active as u16) * input_height
            + (replace_active as u16) * input_height
            + (filter_mode as u16) * filter_height;
        let table_height = terminal_height.saturating_sub(status_height + total_input);

        let table_area = Rect {
            x: 0,
            y: 0,
            width: terminal_width,
            height: table_height,
        };
        areas.push(("table", table_area));

        let mut y_offset = table_height;

        if edit_active {
            let edit_area = Rect {
                x: 0,
                y: y_offset,
                width: terminal_width,
                height: input_height,
            };
            areas.push(("edit", edit_area));
            y_offset += input_height;
        }

        if command_active {
            let command_area = Rect {
                x: 0,
                y: y_offset,
                width: terminal_width,
                height: input_height,
            };
            areas.push(("command", command_area));
            y_offset += input_height;
        }

        if search_active {
            let search_area = Rect {
                x: 0,
                y: y_offset,
                width: terminal_width,
                height: input_height,
            };
            areas.push(("search", search_area));
            y_offset += input_height;
        }

        if replace_active {
            let replace_area = Rect {
                x: 0,
                y: y_offset,
                width: terminal_width,
                height: input_height,
            };
            areas.push(("replace", replace_area));
            y_offset += input_height;
        }

        if filter_mode {
            let filter_area = Rect {
                x: 0,
                y: y_offset,
                width: terminal_width,
                height: filter_height,
            };
            areas.push(("filter", filter_area));
        }

        let status_area = Rect {
            x: 0,
            y: terminal_height - status_height,
            width: terminal_width,
            height: status_height,
        };
        areas.push(("status", status_area));
    }

    for (name, area) in areas {
        match name {
            "help" => render_help_overlay(frame, app),
            "table" => render_table(frame, app, area),
            "command" => render_command_bar(frame, app, area),
            "search" => render_search_bar(frame, app, area),
            "edit" => render_edit_bar(frame, app, area),
            "replace" => render_replace_bar(frame, app, area),
            "filter" => render_filter_popup(frame, app, area),
            "status" => render_status_bar(frame, app, area),
            _ => {}
        }
    }

    if palette_active {
        let width = 50u16;
        let height = (palette_commands.len() as u16 + 4).min(15);

        let palette_area = Rect {
            x: (frame.area().width - width) / 2,
            y: (frame.area().height - height) / 2,
            width,
            height,
        };

        render_palette(
            frame,
            palette_area,
            palette_query,
            palette_cursor,
            palette_commands,
        );
    }
}

fn render_table(frame: &mut Frame, app: &mut App, area: Rect) {
    let data = app.data();
    let headers = data.headers();
    let column_count = data.column_count();
    let row_count = app.row_count();
    let scroll_offset = app.scroll_offset();
    let selected_row = app.selected_row();
    let selected_col = app.selected_col();
    let highlight_duplicates = app.highlight_duplicates();
    let duplicate_rows = app.duplicate_rows();

    let header_rows = 1u16;
    let borders = 2u16;
    let available_height = area.height.saturating_sub(header_rows + borders) as usize;
    let visible_rows = available_height
        .min(row_count.saturating_sub(scroll_offset))
        .max(1);

    let row_index_width = 6u16;

    let col_widths: Vec<Constraint> = std::iter::once(Constraint::Length(row_index_width))
        .chain((0..column_count).map(|i| {
            let max_width = calculate_column_width(app, i, scroll_offset, visible_rows);
            Constraint::Length(
                max_width
                    .max(headers.get(i).map(|h| h.len()).unwrap_or(0))
                    .min(30) as u16,
            )
        }))
        .collect();

    let rows: Vec<ratatui::widgets::Row> = (scroll_offset..scroll_offset + visible_rows)
        .filter_map(|row_idx| {
            let row_data = app.get_display_row(row_idx)?;

            let is_duplicate = if highlight_duplicates {
                duplicate_rows
                    .iter()
                    .any(|(_, rows)| rows.contains(&row_idx))
            } else {
                false
            };

            let index_cell = ratatui::widgets::Cell::from(format!("{:>5}", row_idx + 1))
                .style(Style::new().fg(Color::DarkGray));

            let cells: Vec<ratatui::widgets::Cell> = std::iter::once(index_cell)
                .chain(row_data.iter().enumerate().map(|(col_idx, cell)| {
                    let is_selected = col_idx == selected_col && row_idx == selected_row;
                    let style = if is_selected {
                        Style::new().bg(Color::Blue).fg(Color::White)
                    } else if col_idx == selected_col {
                        Style::new().bg(Color::DarkGray)
                    } else if is_duplicate {
                        Style::new().bg(Color::Yellow).fg(Color::Black)
                    } else if cell.is_empty() {
                        Style::new().fg(Color::DarkGray)
                    } else {
                        Style::new()
                    };
                    ratatui::widgets::Cell::from(cell.clone()).style(style)
                }))
                .collect();

            Some(ratatui::widgets::Row::new(cells))
        })
        .collect();

    let header_cells: Vec<ratatui::widgets::Cell> =
        std::iter::once(ratatui::widgets::Cell::from("  #  ").style(Style::new().bold()))
            .chain(headers.iter().enumerate().map(|(i, h)| {
                let style = if i == selected_col {
                    Style::new().bg(Color::DarkGray).bold()
                } else {
                    Style::new().bold()
                };
                ratatui::widgets::Cell::from(h.clone()).style(style)
            }))
            .collect();

    let header = ratatui::widgets::Row::new(header_cells).height(1);

    let table = ratatui::widgets::Table::new(rows, col_widths)
        .header(header)
        .block(Block::bordered().title(data.file_name()))
        .column_spacing(1);

    frame.render_widget(table, area);
}

fn calculate_column_width(app: &App, col_idx: usize, start_row: usize, num_rows: usize) -> usize {
    let mut max_width = 0;

    for row_idx in start_row..start_row + num_rows {
        if let Some(cell) = app.get_display_cell(row_idx, col_idx) {
            max_width = max_width.max(cell.len());
        }
    }

    max_width + 2
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let data = app.data();
    let row_count = app.row_count();
    let col_count = data.column_count();
    let selected_row = app.selected_row();
    let selected_col = app.selected_col();

    let stats = app.get_column_stats(selected_col);
    let header = data
        .headers()
        .get(selected_col)
        .map(|h| h.as_str())
        .unwrap_or("Column");
    let (current_sheet, sheet_count, _sheet_names) = app.sheet_info();

    let sheet_str = if sheet_count > 1 {
        format!("Sheet: {}/{} | ", current_sheet + 1, sheet_count)
    } else {
        String::new()
    };

    let stats_str = if stats.numeric_count > 0 {
        format!(
            "{}: count={} null={} min={} max={} avg={} sum={}",
            header,
            stats.count,
            stats.null_count,
            format!("{:.1}", stats.min.unwrap_or(0.0)),
            format!("{:.1}", stats.max.unwrap_or(0.0)),
            format!("{:.1}", stats.avg().unwrap_or(0.0)),
            format!("{:.1}", stats.sum)
        )
    } else {
        format!(
            "{}: count={} null={}",
            header, stats.count, stats.null_count
        )
    };

    let position = format!(
        "{}Row: {}/{} | Col: {}/{} | {}",
        sheet_str,
        selected_row + 1,
        row_count,
        selected_col + 1,
        col_count,
        stats_str
    );

    let status_text = if let Some(msg) = app.message() {
        format!("{} | {}", position, msg)
    } else {
        position
    };

    let status = Paragraph::new(status_text)
        .style(Style::new().bg(Color::DarkGray).fg(Color::White))
        .block(Block::new().borders(Borders::ALL));

    frame.render_widget(status, area);
}

fn render_search_bar(frame: &mut Frame, app: &App, area: Rect) {
    let query = app.search_query();
    let search_text = if query.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", query)
    };

    let search_block = Paragraph::new(search_text)
        .style(Style::new().bg(Color::Black).fg(Color::Green))
        .block(Block::new().borders(Borders::ALL).title("Search"));

    frame.render_widget(search_block, area);
}

fn render_replace_bar(frame: &mut Frame, app: &App, area: Rect) {
    let query = app.replace_query();
    let replace_with = app.replace_with();
    let all_mode = app.replace_all_mode();
    let replace_text = if replace_with.is_empty() {
        format!("with: (replacing '{}')", query)
    } else {
        format!("with: {}", replace_with)
    };

    let (bg_color, title) = if all_mode {
        (Color::Magenta, "Replace All")
    } else {
        (Color::Yellow, "Replace")
    };

    let replace_block = Paragraph::new(replace_text)
        .style(Style::new().bg(Color::Black).fg(bg_color))
        .block(Block::new().borders(Borders::ALL).title(title));

    frame.render_widget(replace_block, area);
}

fn render_edit_bar(frame: &mut Frame, app: &App, area: Rect) {
    let buffer = app.edit_buffer();
    let col = app.selected_col();
    let row = app.selected_row() + 1;
    let edit_text = format!("Editing cell {}:{} - {}", row, col + 1, buffer);

    let edit_block = Paragraph::new(edit_text)
        .style(Style::new().bg(Color::Blue).fg(Color::White))
        .block(Block::new().borders(Borders::ALL).title("Edit"));

    frame.render_widget(edit_block, area);
}

fn render_command_bar(frame: &mut Frame, app: &App, area: Rect) {
    let buffer = app.command_buffer();
    let command_text = format!(":{}", buffer);

    let command_block = Paragraph::new(command_text)
        .style(Style::new().bg(Color::Magenta).fg(Color::White))
        .block(Block::new().borders(Borders::ALL).title("Command"));

    frame.render_widget(command_block, area);
}

fn render_help_overlay(frame: &mut Frame, _app: &App) {
    let area = frame.area();
    let help_text = vec![
        Line::from("┌─ SheetView Help ───────────────────────────────────────────┐"),
        Line::from("│ Navigation                                                 │"),
        Line::from("│   ↑/k  ↓/j  ←/h  →/l/;   Move                              │"),
        Line::from("│   Ctrl+B     Page Up                                       │"),
        Line::from("│   Ctrl+F     Page Down                                     │"),
        Line::from("│   gg          First row                                    │"),
        Line::from("│   G           Last row                                     │"),
        Line::from("│   0           First column                                 │"),
        Line::from("│   $           Last column                                  │"),
        Line::from("│   Num + j/k/h/l  Move by number (e.g., 10j)                │"),
        Line::from("│                                                            │"),
        Line::from("│ Search                                                     │"),
        Line::from("│   /           Start search                                 │"),
        Line::from("│   n           Next match                                   │"),
        Line::from("│   N           Previous match                               │"),
        Line::from("│   R           Toggle regex mode                            │"),
        Line::from("│   f           Filter by column                             │"),
        Line::from("│                                                            │"),
        Line::from("│ Sort                                                       │"),
        Line::from("│   s           Sort by selected column                      │"),
        Line::from("│   R           Clear sort                                   │"),
        Line::from("│                                                            │"),
        Line::from("│ Analysis                                                   │"),
        Line::from("│   U           Toggle duplicate highlight                   │"),
        Line::from("│                                                            │"),
        Line::from("│ Sheets (Excel)                                             │"),
        Line::from("│   t           Next sheet                                   │"),
        Line::from("│   T           Previous sheet                               │"),
        Line::from("│                                                            │"),
        Line::from("│ Row Operations                                             │"),
        Line::from("│   o           Insert row below                             │"),
        Line::from("│   O           Insert row above                             │"),
        Line::from("│   x           Delete row                                   │"),
        Line::from("│                                                            │"),
        Line::from("│ Editing                                                    │"),
        Line::from("│   Enter/i    Edit cell                                     │"),
        Line::from("│   y           Copy cell                                    │"),
        Line::from("│   Y           Copy row                                     │"),
        Line::from("│   d           Cut cell                                     │"),
        Line::from("│   p           Paste cell                                   │"),
        Line::from("│   P           Paste row                                    │"),
        Line::from("│   u           Undo                                         │"),
        Line::from("│   r           Redo                                         │"),
        Line::from("│                                                            │"),
        Line::from("│ Actions                                                    │"),
        Line::from("│   ?           Toggle help                                  │"),
        Line::from("│   Ctrl+P      Command palette                              │"),
        Line::from("│   ┌───────────── Command Palette Help ───────────────────┐ │"),
        Line::from("│   │ Inside palette:                                      │ │"),
        Line::from("│   │   ↑/k     Move cursor up                             │ │"),
        Line::from("│   │   ↓/j     Move cursor down                           │ │"),
        Line::from("│   │   Enter   Execute selected command                   │ │"),
        Line::from("│   │   Esc     Close palette                              │ │"),
        Line::from("│   │   Type    Filter commands                            │ │"),
        Line::from("│   └──────────────────────────────────────────────────────┘ │"),
        Line::from("│   Ctrl+S      Save file                                    │"),
        Line::from("│   :w          Save                                         │"),
        Line::from("│   :export <f> Export filtered data to CSV                  │"),
        Line::from("│   q/Esc       Quit                                         │"),
        Line::from("│   :q!         Force quit                                   │"),
        Line::from("└────────────────────────────────────────────────────────────┘"),
    ];

    let help_width = 64u16;
    let help_height = help_text.len() as u16 + 2;
    let help_area = Rect {
        x: (area.width - help_width) / 2,
        y: (area.height - help_height) / 2,
        width: help_width,
        height: help_height,
    };

    let help = Paragraph::new(help_text)
        .style(Style::new().bg(Color::Black).fg(Color::White))
        .block(Block::bordered().border_type(ratatui::widgets::BorderType::Rounded))
        .alignment(ratatui::layout::Alignment::Left);

    frame.render_widget(help, help_area);
}

fn render_filter_popup(frame: &mut Frame, app: &App, area: Rect) {
    let (_filter_active, filter_mode, filter_col, filter_values, filter_selected, filter_cursor) =
        app.filter_info();

    if !filter_mode {
        return;
    }

    let col_name = if let Some(col) = filter_col {
        app.data()
            .headers()
            .get(col)
            .map(|h| h.as_str())
            .unwrap_or("Column")
    } else {
        "Column"
    };

    let selected_count = filter_selected.iter().filter(|&&v| v).count();
    let total_count = filter_values.len();

    let mut lines = Vec::new();
    lines.push(format!(" Filter: {} ", col_name));
    lines.push("─".repeat(area.width as usize - 2));

    let max_visible = (area.height as usize).saturating_sub(4);
    let start = filter_cursor.saturating_sub(max_visible / 2);
    let end = (start + max_visible).min(filter_values.len());

    for (i, value) in filter_values
        .iter()
        .enumerate()
        .skip(start)
        .take(max_visible)
    {
        if i >= end {
            break;
        }
        let is_selected = filter_selected[i];
        let is_cursor = i == filter_cursor;
        let marker = if is_selected { "[✓]" } else { "[ ]" };
        let prefix = if is_cursor { ">" } else { " " };
        lines.push(format!("{} {} {}", prefix, marker, value));
    }

    lines.push("─".repeat(area.width as usize - 2));
    lines.push(format!(
        " Selected: {}/{} | SPACE=toggle | Enter=apply | Esc=cancel",
        selected_count, total_count
    ));

    let filter_block = Paragraph::new(lines.join("\n"))
        .style(Style::new().bg(Color::DarkGray).fg(Color::White))
        .block(Block::bordered().border_type(ratatui::widgets::BorderType::Rounded));

    frame.render_widget(filter_block, area);
}

fn render_palette(
    frame: &mut Frame,
    area: Rect,
    query: String,
    cursor: usize,
    commands: Vec<String>,
) {
    let width = area.width as usize;
    let height = area.height as usize;

    let mut lines = Vec::new();
    lines.push(" Command Palette ".to_string());
    lines.push("─".repeat(width));

    let max_visible = height.saturating_sub(4);

    for (i, cmd) in commands.iter().enumerate().take(max_visible) {
        let is_cursor = i == cursor;
        let prefix = if is_cursor { ">" } else { " " };
        // Truncate command if too long to fit in the palette area (we have 2 chars for prefix and space)
        let max_cmd_len = width.saturating_sub(2);
        let display_cmd = if cmd.len() > max_cmd_len {
            format!("{}...", &cmd[..max_cmd_len.saturating_sub(3)])
        } else {
            cmd.clone()
        };
        let line_content = format!("{} {}", prefix, display_cmd);
        // Pad to full width
        let padding = width.saturating_sub(line_content.len());
        let line = format!("{}{}", line_content, " ".repeat(padding));
        lines.push(line);
    }

    lines.push("─".repeat(width));

    let help_text = format!(
        " ↑/k ↓/j: Navigate | Enter: Execute | Esc: Close | Type: Filter {}",
        if query.is_empty() {
            "type to filter"
        } else {
            &query
        }
    );
    let display_help = if help_text.len() > width {
        format!("{}...", &help_text[..width.saturating_sub(3)])
    } else {
        help_text
    };
    let help_padding = width.saturating_sub(display_help.len());
    let help_line = format!("{}{}", display_help, " ".repeat(help_padding));
    lines.push(help_line);

    let palette_block = Paragraph::new(lines.join("\n"))
        .style(Style::new().bg(Color::Black).fg(Color::White))
        .block(Block::bordered().border_type(ratatui::widgets::BorderType::Rounded));

    frame.render_widget(palette_block, area);
}

#[allow(dead_code)]
fn render_palette_overlay(frame: &mut Frame, query: String, cursor: usize, commands: Vec<String>) {
    let area = frame.area();
    let width = 50u16;
    let height = (commands.len() as u16 + 4).min(15);
    let palette_area = Rect {
        x: (area.width - width) / 2,
        y: (area.height - height) / 2,
        width,
        height,
    };

    render_palette(frame, palette_area, query, cursor, commands);
}
