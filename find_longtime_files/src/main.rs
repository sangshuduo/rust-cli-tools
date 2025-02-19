use chrono::NaiveDateTime;
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};

/// Remove ANSI escape codes from a string.
fn remove_ansi_codes(s: &str) -> String {
    // This regex matches ANSI escape sequences.
    let ansi_re = Regex::new(r"\x1B\[[0-9;]*[a-zA-Z]").unwrap();
    ansi_re.replace_all(s, "").to_string()
}

/// Extract the timestamp from a line (first two whitespace-separated tokens).
fn extract_timestamp(line: &str) -> Option<String> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 2 {
        None
    } else {
        Some(format!("{} {}", parts[0], parts[1]))
    }
}

/// Extract the filename from the line using a regex.
/// The regex captures the filename following "The format of" and before "is <format>".
fn extract_filename(line: &str) -> Option<String> {
    let re = Regex::new(r"The format of\s+(\S+)\s+is\s+\S+").unwrap();
    re.captures(line)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
}

fn main() {
    // Check command-line arguments.
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <number-of-files> <log-file>", args[0]);
        std::process::exit(1);
    }
    let num_files: usize = args[1].parse().expect("Invalid number-of-files");
    let log_file = &args[2];

    // Open the log file.
    let file = File::open(log_file).unwrap_or_else(|err| {
        eprintln!("Error opening {}: {}", log_file, err);
        std::process::exit(1);
    });
    // Get file metadata to determine total file size.
    let metadata = file.metadata().expect("Failed to get metadata");
    let total_size = metadata.len();

    let reader = BufReader::new(file);

    // Create a progress bar based on total file size.
    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("##-"),
    );

    // We'll store (duration_in_seconds, filename) pairs.
    let mut diffs: Vec<(f64, String)> = Vec::new();

    // Variables to hold the previous log entry's timestamp and file name.
    let mut prev_dt: Option<NaiveDateTime> = None;
    let mut prev_file: Option<String> = None;

    // Timestamp format for milliseconds (3 digits).
    let timestamp_format = "%Y-%m-%d %H:%M:%S.%3f";

    for line in reader.lines() {
        // Update progress bar with the length of the line plus newline.
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Error reading line: {}", e);
                continue;
            }
        };
        pb.inc((line.len() + 1) as u64);

        // Remove ANSI escape sequences.
        let clean_line = remove_ansi_codes(&line);

        // Extract the timestamp string.
        let ts_str = match extract_timestamp(&clean_line) {
            Some(ts) => ts,
            None => continue,
        };

        // Parse the timestamp as a NaiveDateTime (without timezone).
        let naive_dt = match NaiveDateTime::parse_from_str(&ts_str, timestamp_format) {
            Ok(dt) => dt,
            Err(e) => {
                eprintln!("Error parsing date '{}': {}", ts_str, e);
                continue;
            }
        };

        // Extract the filename.
        let filename = match extract_filename(&clean_line) {
            Some(f) => f,
            None => continue,
        };

        // If we have a previous timestamp, compute the duration.
        if let (Some(prev), Some(prev_filename)) = (prev_dt, &prev_file) {
            let duration = naive_dt.signed_duration_since(prev);
            let diff_seconds = duration.num_microseconds().unwrap_or(0) as f64 / 1_000_000.0;
            diffs.push((diff_seconds, prev_filename.clone()));
        }

        // Update the previous values.
        prev_dt = Some(naive_dt);
        prev_file = Some(filename);
    }

    pb.finish_with_message("Processing complete");

    // Sort by processing time in descending order.
    diffs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

    println!("Top {} files with longest processing times:", num_files);
    for (i, (duration, file)) in diffs.iter().take(num_files).enumerate() {
        println!("{}. {} took {:.6} seconds", i + 1, file, duration);
    }
}
