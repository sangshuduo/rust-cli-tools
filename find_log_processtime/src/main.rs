use anyhow::{Context, Result};
use chrono::NaiveDateTime;
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use std::fs::File;
use std::io::{BufRead, BufReader};

/// Find log processing times from a log file.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Display top files with the longest processing times.
    Top {
        /// Number of top files to display.
        num_files: usize,
        /// Path to the log file.
        log_file: String,
    },
    /// Calculate and display the average processing time across all files.
    Avg {
        /// Path to the log file.
        log_file: String,
    },
}

/// Remove ANSI escape codes from a string.
fn remove_ansi_codes(s: &str) -> String {
    // Regex to match ANSI escape sequences.
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

/// Extract the filename from a line using a regex.
/// Captures the filename following "The format of" and before "is <format>".
fn extract_filename(line: &str) -> Option<String> {
    let re = Regex::new(r"The format of\s+(\S+)\s+is\s+\S+").unwrap();
    re.captures(line)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
}

/// Compute processing time differences from the log file.
/// Returns a vector of tuples (processing time in seconds, filename).
fn compute_diffs(log_file: &str) -> Result<Vec<(f64, String)>> {
    // Open the log file.
    let file =
        File::open(log_file).with_context(|| format!("Error opening log file: {}", log_file))?;
    let metadata = file.metadata().context("Failed to get file metadata")?;
    let total_size = metadata.len();
    let reader = BufReader::new(file);

    // Create a progress bar based on the total file size.
    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("##-"),
    );

    // Store (processing time in seconds, filename) pairs.
    let mut diffs: Vec<(f64, String)> = Vec::new();

    // Variables to hold the previous log entry's timestamp and file name.
    let mut prev_dt: Option<NaiveDateTime> = None;
    let mut prev_file: Option<String> = None;

    // Timestamp format: milliseconds (3 digits).
    const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S.%3f";

    for line in reader.lines() {
        let line = line.context("Error reading a line")?;
        pb.inc((line.len() + 1) as u64);

        // Remove ANSI escape sequences.
        let clean_line = remove_ansi_codes(&line);

        // Extract and parse the timestamp.
        let ts_str = match extract_timestamp(&clean_line) {
            Some(ts) => ts,
            None => continue,
        };

        let naive_dt = match NaiveDateTime::parse_from_str(&ts_str, TIMESTAMP_FORMAT) {
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

        // If we have a previous timestamp, compute the processing duration.
        if let (Some(prev), Some(prev_filename)) = (prev_dt, &prev_file) {
            let duration = naive_dt.signed_duration_since(prev);
            let diff_seconds = duration.num_microseconds().unwrap_or(0) as f64 / 1_000_000.0;
            diffs.push((diff_seconds, prev_filename.clone()));
        }

        prev_dt = Some(naive_dt);
        prev_file = Some(filename);
    }

    pb.finish_with_message("Processing complete");
    Ok(diffs)
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Top {
            num_files,
            log_file,
        } => {
            let mut diffs = compute_diffs(&log_file)?;
            // Sort by processing time in descending order.
            diffs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

            println!("Top {} files with longest processing times:", num_files);
            for (i, (duration, file)) in diffs.iter().take(num_files).enumerate() {
                println!("{}. {} took {:.6} seconds", i + 1, file, duration);
            }
        }
        Command::Avg { log_file } => {
            let diffs = compute_diffs(&log_file)?;
            if diffs.is_empty() {
                println!("No processing times found in the log file.");
            } else {
                let total: f64 = diffs.iter().map(|(duration, _)| duration).sum();
                let avg = total / (diffs.len() as f64);
                println!("Average processing time: {:.6} seconds", avg);
            }
        }
    }

    Ok(())
}
