use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use csv::ReaderBuilder;

use super::DataSource;

pub struct CsvSource {
    file_path: PathBuf,
    file_name: String,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    modified: bool,
}

impl CsvSource {
    pub fn open(path: &Path, delimiter: Option<char>) -> Result<Self, Box<dyn std::error::Error>> {
        let file_path = path.to_path_buf();
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let delimiter = delimiter.unwrap_or(',');
        let delimiter = delimiter as u8;

        let file = File::open(&file_path)?;

        let mut reader = ReaderBuilder::new()
            .delimiter(delimiter)
            .has_headers(true)
            .flexible(true)
            .from_reader(BufReader::new(&file));

        let headers: Vec<String> = reader
            .headers()?
            .iter()
            .map(|s| s.to_string())
            .collect();

        let mut rows: Vec<Vec<String>> = Vec::new();

        for result in reader.records() {
            let record = result?;
            let row: Vec<String> = record.iter().map(|s| s.to_string()).collect();
            rows.push(row);
        }

        Ok(Self {
            file_path,
            file_name,
            headers,
            rows,
            modified: false,
        })
    }
}

impl DataSource for CsvSource {
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
        if !self.modified {
            return Ok(());
        }

        let mut wtr = csv::Writer::from_path(&self.file_path)?;

        wtr.write_record(&self.headers)?;

        for row in &self.rows {
            wtr.write_record(row)?;
        }

        wtr.flush()?;
        self.modified = false;
        Ok(())
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
        vec!["Sheet1".to_string()]
    }

    fn sheet_count(&self) -> usize {
        1
    }

    fn current_sheet(&self) -> usize {
        0
    }

    fn switch_sheet(&mut self, _index: usize) -> Result<(), Box<dyn std::error::Error>> {
        if _index == 0 { Ok(()) } else { Err("CSV has only one sheet".into()) }
    }
}