mod app;
mod data;
mod ui;

use std::path::PathBuf;
use std::process;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "sheetview")]
#[command(about = "A fast CSV/Excel TUI viewer", long_about = None)]
struct Args {
    #[arg(help = "Path to CSV or Excel file")]
    file: PathBuf,

    #[arg(short, long, help = "Delimiter character for CSV files")]
    delimiter: Option<char>,

    #[arg(short, long, help = "Number of rows to preview (0 = all)")]
    limit: Option<usize>,
}

fn main() {
    let args = Args::parse();

    let file_path = args.file.clone();
    if !file_path.exists() {
        eprintln!("Error: File not found: {}", file_path.display());
        process::exit(1);
    }

    let mut app = match app::App::new(&file_path, args.delimiter) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };

    if let Err(e) = app.run() {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
