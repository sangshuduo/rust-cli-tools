use std::error::Error;
use std::fs::File;
use std::io::{self, BufRead};

use clap::Parser;
use csv::Writer;
use rust_xlsxwriter::Workbook;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Benchmark name
    #[arg(long, value_name = "BENCHMARK")]
    benchmark: String,

    /// Module name
    #[arg(long, value_name = "MODULE")]
    module: String,

    /// Input file path
    #[arg(long, value_name = "INPUT_FILE")]
    input: String,

    /// Output file path
    #[arg(long, value_name = "OUTPUT_FILE")]
    output: String,
}

struct DataEntry {
    benchmark: String,
    module: String,
    dataset: String,
    result: String,
    values: Vec<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    // Parse command-line arguments
    let args = Args::parse();

    // Parse the input file
    let data_entries = parse_input_file(&args)?;

    // Determine output format based on file extension
    if args.output.ends_with(".xlsx") {
        write_excel(&data_entries, &args.output)?;
    } else if args.output.ends_with(".csv") {
        write_csv(&data_entries, &args.output)?;
    } else {
        eprintln!("Unsupported output file format. Please use .xlsx or .csv extension.");
        std::process::exit(1);
    }

    Ok(())
}

fn parse_input_file(args: &Args) -> Result<Vec<DataEntry>, Box<dyn Error>> {
    let mut data_entries = Vec::new();
    let mut current_dataset = String::new();

    let file = File::open(&args.input)?;
    let reader = io::BufReader::new(file);

    for line_result in reader.lines() {
        let line = line_result?;
        let line = line.trim();
        if line.ends_with("dataset") {
            // Dataset line
            let dataset_name = line.trim_end_matches("dataset").trim();
            current_dataset = dataset_name.to_string();
        } else if line.contains(':') {
            // Data line
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            let result_name = parts[0].trim();
            let values_str = parts[1].trim();

            // Extract values inside square brackets
            if values_str.starts_with('[') && values_str.ends_with(']') {
                let values_content = &values_str[1..values_str.len() - 1];
                // Split values by comma
                let values: Vec<String> = values_content
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();

                // Create a DataEntry and add to the vector
                data_entries.push(DataEntry {
                    benchmark: args.benchmark.clone(),
                    module: args.module.clone(),
                    dataset: current_dataset.clone(),
                    result: result_name.to_string(),
                    values,
                });
            } else {
                eprintln!("Invalid values format: {}", line);
            }
        } else {
            // Ignore empty or unrecognized lines
        }
    }

    Ok(data_entries)
}

fn write_excel(data_entries: &[DataEntry], output_file: &str) -> Result<(), Box<dyn Error>> {
    // Create a new workbook
    let mut workbook = Workbook::new();

    // Add a worksheet
    let worksheet = workbook.add_worksheet();

    // Write the header row
    let headers = vec![
        "benchmark",
        "module",
        "dataset",
        "result",
        "3",
        "5",
        "10",
        "20",
        "30",
        "40",
        "50",
        "60",
        "70",
        "80",
        "90",
        "100",
    ];

    // Write the headers
    for (col_num, header) in headers.iter().enumerate() {
        worksheet.write(0, col_num as u16, *header)?;
    }

    // Write the data entries
    for (row_num, entry) in data_entries.iter().enumerate() {
        let row = (row_num + 1) as u32;

        // Column indices:
        // Column 0: benchmark
        // Column 1: module
        // Column 2: dataset
        // Column 3: result
        // Columns 4 onward: values

        worksheet.write_string(row, 0, &entry.benchmark)?;
        worksheet.write_string(row, 1, &entry.module)?;
        worksheet.write_string(row, 2, &entry.dataset)?;
        worksheet.write_string(row, 3, &entry.result)?;

        // Write the values
        for (i, value) in entry.values.iter().enumerate() {
            let col = (i + 4) as u16;
            if let Ok(num) = value.parse::<f64>() {
                worksheet.write_number(row, col, num)?;
            } else {
                worksheet.write_string(row, col, value)?;
            }
        }
    }

    // Save the workbook
    workbook.save(output_file)?;

    Ok(())
}

fn write_csv(data_entries: &[DataEntry], output_file: &str) -> Result<(), Box<dyn Error>> {
    let mut wtr = Writer::from_path(output_file)?;

    // Write the header row
    let headers = vec![
        "benchmark",
        "module",
        "dataset",
        "result",
        "3",
        "5",
        "10",
        "20",
        "30",
        "40",
        "50",
        "60",
        "70",
        "80",
        "90",
        "100",
    ];

    wtr.write_record(&headers)?;

    // Write the data entries
    for entry in data_entries {
        let mut row = vec![
            entry.benchmark.clone(),
            entry.module.clone(),
            entry.dataset.clone(),
            entry.result.clone(),
        ];

        // Append the values
        row.extend(entry.values.clone());

        wtr.write_record(&row)?;
    }

    wtr.flush()?;
    Ok(())
}
