use calamine::{open_workbook_auto, Data, Reader};
use clap::Parser;
use std::error::Error;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the xlsx file
    xlsx_file: PathBuf,
}

/// Reads and displays the contents of an Excel (.xlsx) file.
/// Iterates through all worksheets and prints their contents in a tab-separated format.
/// Each worksheet is clearly delimited and labeled.
fn main() -> Result<(), Box<dyn Error>> {
    // Get the path to the xlsx file from command-line arguments
    let args = Args::parse();

    let path = args.xlsx_file;
    // Check if the file exists
    if !path.exists() {
    // Validate file extension
    if !path.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("xlsx")) {
        eprintln!("Error: File must have .xlsx extension");
        std::process::exit(1);
    }

    // Open the workbook (auto-detects the format)
    let mut workbook = open_workbook_auto(path)?;

    // Iterate over the worksheets
    let sheet_names = workbook.sheet_names().to_owned();
    for sheet_name in sheet_names {
        if let Ok(range) = workbook.worksheet_range(&sheet_name) {
            println!("Sheet: {}", sheet_name);
            for row in range.rows() {
                for cell in row {
                    match cell {
                        Data::Empty => print!("(empty)\t"),
                        Data::String(s) => print!("{}\t", s),
                        Data::Float(f) => print!("{}\t", f),
                        Data::Int(i) => print!("{}\t", i),
                        Data::Bool(b) => print!("{}\t", b),
                        Data::Error(e) => print!("Error({:?})\t", e),
                        Data::DateTime(dt) => print!("DateTime({})\t", dt),
                        _ => print!("(unknown)\t"),
                    }
                }
                println!();
            }
            println!("-----------------------------------");
        }
    }

    Ok(())
}
