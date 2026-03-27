mod csv_source;
mod excel_source;

pub use csv_source::CsvSource;
pub use excel_source::ExcelSource;

use std::error::Error;

pub trait DataSource: Send {
    fn headers(&self) -> &[String];
    fn row_count(&self) -> usize;
    fn column_count(&self) -> usize;
    fn get_row(&self, index: usize) -> Option<Vec<String>>;
    fn get_cell(&self, row: usize, col: usize) -> Option<String>;
    fn set_cell(&mut self, row: usize, col: usize, value: &str);
    fn insert_row(&mut self, at: usize);
    fn delete_row(&mut self, at: usize);
    fn file_name(&self) -> String;
    fn save(&mut self) -> Result<(), Box<dyn Error>>;
    fn sheet_names(&self) -> Vec<String>;
    fn sheet_count(&self) -> usize;
    fn current_sheet(&self) -> usize;
    fn switch_sheet(&mut self, index: usize) -> Result<(), Box<dyn Error>>;
}

pub enum DataSourceType {
    Csv(CsvSource),
    Excel(ExcelSource),
}

impl DataSource for DataSourceType {
    fn headers(&self) -> &[String] {
        match self {
            DataSourceType::Csv(ds) => ds.headers(),
            DataSourceType::Excel(ds) => ds.headers(),
        }
    }

    fn row_count(&self) -> usize {
        match self {
            DataSourceType::Csv(ds) => ds.row_count(),
            DataSourceType::Excel(ds) => ds.row_count(),
        }
    }

    fn column_count(&self) -> usize {
        match self {
            DataSourceType::Csv(ds) => ds.column_count(),
            DataSourceType::Excel(ds) => ds.column_count(),
        }
    }

    fn get_row(&self, index: usize) -> Option<Vec<String>> {
        match self {
            DataSourceType::Csv(ds) => ds.get_row(index),
            DataSourceType::Excel(ds) => ds.get_row(index),
        }
    }

    fn get_cell(&self, row: usize, col: usize) -> Option<String> {
        match self {
            DataSourceType::Csv(ds) => ds.get_cell(row, col),
            DataSourceType::Excel(ds) => ds.get_cell(row, col),
        }
    }

    fn set_cell(&mut self, row: usize, col: usize, value: &str) {
        match self {
            DataSourceType::Csv(ds) => ds.set_cell(row, col, value),
            DataSourceType::Excel(ds) => ds.set_cell(row, col, value),
        }
    }

    fn insert_row(&mut self, at: usize) {
        match self {
            DataSourceType::Csv(ds) => ds.insert_row(at),
            DataSourceType::Excel(ds) => ds.insert_row(at),
        }
    }

    fn delete_row(&mut self, at: usize) {
        match self {
            DataSourceType::Csv(ds) => ds.delete_row(at),
            DataSourceType::Excel(ds) => ds.delete_row(at),
        }
    }

    fn file_name(&self) -> String {
        match self {
            DataSourceType::Csv(ds) => ds.file_name(),
            DataSourceType::Excel(ds) => ds.file_name(),
        }
    }

    fn save(&mut self) -> Result<(), Box<dyn Error>> {
        match self {
            DataSourceType::Csv(ds) => ds.save(),
            DataSourceType::Excel(_) => {
                Err("Excel saving not supported yet".into())
            }
        }
    }

    fn sheet_names(&self) -> Vec<String> {
        match self {
            DataSourceType::Csv(_) => vec!["Sheet1".to_string()],
            DataSourceType::Excel(ds) => ds.sheet_names().to_vec(),
        }
    }

    fn sheet_count(&self) -> usize {
        match self {
            DataSourceType::Csv(_) => 1,
            DataSourceType::Excel(ds) => ds.sheet_names().len(),
        }
    }

    fn current_sheet(&self) -> usize {
        match self {
            DataSourceType::Csv(_) => 0,
            DataSourceType::Excel(ds) => ds.current_sheet(),
        }
    }

    fn switch_sheet(&mut self, index: usize) -> Result<(), Box<dyn Error>> {
        match self {
            DataSourceType::Csv(_) => {
                if index == 0 { Ok(()) } else { Err("CSV has only one sheet".into()) }
            }
            DataSourceType::Excel(ds) => ds.switch_sheet(index),
        }
    }
}

pub fn open_file(path: &std::path::Path, delimiter: Option<char>) -> Result<DataSourceType, Box<dyn Error>> {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    match extension.as_str() {
        "csv" | "tsv" | "txt" => {
            let source = CsvSource::open(path, delimiter)?;
            Ok(DataSourceType::Csv(source))
        }
        "xlsx" | "xls" | "xlsm" => {
            let source = ExcelSource::open(path)?;
            Ok(DataSourceType::Excel(source))
        }
        _ => {
            let source = CsvSource::open(path, delimiter)?;
            Ok(DataSourceType::Csv(source))
        }
    }
}
