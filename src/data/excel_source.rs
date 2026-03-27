use std::path::{Path, PathBuf};

use calamine::{open_workbook_auto, Data, Reader};

use super::DataSource;

pub struct ExcelSource {
    file_path: PathBuf,
    file_name: String,
    sheet_names: Vec<String>,
    current_sheet: usize,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    modified: bool,
}

impl ExcelSource {
    pub fn open(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let file_path = path.to_path_buf();
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let mut workbook = open_workbook_auto(&file_path)?;

        let sheet_names = workbook.sheet_names().to_vec();
        let sheet_name = sheet_names.first().ok_or("No sheets found")?;

        let range = workbook.worksheet_range(sheet_name)?;

        let mut rows: Vec<Vec<String>> = Vec::new();
        let mut headers: Vec<String> = Vec::new();

        for (i, row) in range.rows().enumerate() {
            let row_data: Vec<String> = row
                .iter()
                .map(|cell| match cell {
                    Data::Int(n) => n.to_string(),
                    Data::Float(f) => f.to_string(),
                    Data::String(s) => s.clone(),
                    Data::Bool(b) => b.to_string(),
                    Data::DateTime(dt) => dt.to_string(),
                    Data::Empty => String::new(),
                    Data::Error(e) => format!("{:?}", e),
                    Data::DateTimeIso(s) => s.clone(),
                    Data::DurationIso(s) => s.clone(),
                })
                .collect();

            if i == 0 {
                headers = row_data;
            } else {
                rows.push(row_data);
            }
        }

        Ok(Self {
            file_path,
            file_name,
            sheet_names,
            current_sheet: 0,
            headers,
            rows,
            modified: false,
        })
    }

    fn read_current_sheet(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let sheet_name = &self.sheet_names[self.current_sheet];
        let mut workbook = open_workbook_auto(&self.file_path)?;
        let range = workbook.worksheet_range(sheet_name)?;

        let mut rows: Vec<Vec<String>> = Vec::new();
        let mut headers: Vec<String> = Vec::new();

        for (i, row) in range.rows().enumerate() {
            let row_data: Vec<String> = row
                .iter()
                .map(|cell| match cell {
                    Data::Int(n) => n.to_string(),
                    Data::Float(f) => f.to_string(),
                    Data::String(s) => s.clone(),
                    Data::Bool(b) => b.to_string(),
                    Data::DateTime(dt) => dt.to_string(),
                    Data::Empty => String::new(),
                    Data::Error(e) => format!("{:?}", e),
                    Data::DateTimeIso(s) => s.clone(),
                    Data::DurationIso(s) => s.clone(),
                })
                .collect();

            if i == 0 {
                headers = row_data;
            } else {
                rows.push(row_data);
            }
        }

        self.headers = headers;
        self.rows = rows;
        Ok(())
    }

    pub fn sheet_names(&self) -> &[String] {
        &self.sheet_names
    }

    pub fn current_sheet(&self) -> usize {
        self.current_sheet
    }

    pub fn switch_sheet(&mut self, index: usize) -> Result<(), Box<dyn std::error::Error>> {
        if index >= self.sheet_names.len() {
            return Err("Sheet index out of bounds".into());
        }

        self.current_sheet = index;
        self.read_current_sheet()?;
        self.modified = false;
        Ok(())
    }
}

impl DataSource for ExcelSource {
    fn headers(&self) -> &[String] {
        &self.headers
    }

    fn row_count(&self) -> usize {
        self.rows.len()
    }

    fn column_count(&self) -> usize {
        self.headers.len()
    }

    fn get_row(&self, index: usize) -> Option<Vec<String>> {
        self.rows.get(index).cloned()
    }

    fn get_cell(&self, row: usize, col: usize) -> Option<String> {
        self.rows.get(row)?.get(col).cloned()
    }

    fn file_name(&self) -> String {
        self.file_name.clone()
    }

    fn save(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Err("Excel saving not supported yet".into())
    }

    fn set_cell(&mut self, row: usize, col: usize, value: &str) {
        if let Some(row_data) = self.rows.get_mut(row) {
            if col < row_data.len() {
                row_data[col] = value.to_string();
                self.modified = true;
            } else if col == row_data.len() {
                row_data.push(value.to_string());
                self.modified = true;
            }
        }
    }

    fn insert_row(&mut self, at: usize) {
        let col_count = self.headers.len();
        let empty_row: Vec<String> = (0..col_count).map(|_| String::new()).collect();
        let at = at.min(self.rows.len());
        self.rows.insert(at, empty_row);
        self.modified = true;
    }

    fn delete_row(&mut self, at: usize) {
        if at < self.rows.len() {
            self.rows.remove(at);
            self.modified = true;
        }
    }

    fn sheet_names(&self) -> Vec<String> {
        self.sheet_names.clone()
    }

    fn sheet_count(&self) -> usize {
        self.sheet_names.len()
    }

    fn current_sheet(&self) -> usize {
        self.current_sheet
    }

    fn switch_sheet(&mut self, index: usize) -> Result<(), Box<dyn std::error::Error>> {
        ExcelSource::switch_sheet(self, index)
    }
}