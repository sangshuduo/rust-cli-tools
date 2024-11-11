use calamine::{open_workbook_auto, DataType, Reader};
use std::env;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Get the path to the xlsx file from command-line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <file.xlsx>", args[0]);
        std::process::exit(1);
    }
    let path = &args[1];

    // Open the workbook (auto-detects the format)
    let mut workbook = open_workbook_auto(path)?;

    // Iterate over the worksheets
    let sheet_names = workbook.sheet_names().to_owned();
    for sheet_name in sheet_names {
        if let Some(Ok(range)) = workbook.worksheet_range(&sheet_name) {
            println!("Sheet: {}", sheet_name);
            for row in range.rows() {
                for cell in row {
                    match cell {
                        DataType::Empty => print!("(empty)\t"),
                        DataType::String(s) => print!("{}\t", s),
                        DataType::Float(f) => print!("{}\t", f),
                        DataType::Int(i) => print!("{}\t", i),
                        DataType::Bool(b) => print!("{}\t", b),
                        DataType::Error(e) => print!("Error({:?})\t", e),
                        DataType::DateTime(dt) => print!("DateTime({})\t", dt),
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
