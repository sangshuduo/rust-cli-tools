use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use rand::seq::SliceRandom;
use std::fs;
use std::path::PathBuf;

/// Copies a random number of files from one directory to another.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Source directory path
    source_directory: PathBuf,

    /// Destination directory path
    destination_directory: PathBuf,

    /// Number of files to copy
    number_of_files: usize,
}

fn main() {
    // Parse command-line arguments
    let args = Args::parse();

    // Validate number_of_files is positive
    if args.number_of_files == 0 {
        eprintln!("Error: Number of files must be a positive integer.");
        print_usage_and_exit();
    }

    // Check if source directory exists and is a directory
    if !args.source_directory.exists() || !args.source_directory.is_dir() {
        eprintln!(
            "Error: Source directory '{}' does not exist or is not a directory.",
            args.source_directory.display()
        );
        std::process::exit(1);
    }

    // Create destination directory if it doesn't exist
    if let Err(e) = fs::create_dir_all(&args.destination_directory) {
        eprintln!(
            "Error: Failed to create destination directory '{}': {}",
            args.destination_directory.display(),
            e
        );
        std::process::exit(1);
    }

    // Read the list of files in the source directory
    let files = match fs::read_dir(&args.source_directory) {
        Ok(entries) => entries
            .filter_map(|entry| {
                entry.ok().and_then(|e| {
                    let path = e.path();
                    if path.is_file() {
                        Some(path)
                    } else {
                        None
                    }
                })
            })
            .collect::<Vec<PathBuf>>(),
        Err(e) => {
            eprintln!(
                "Error: Failed to read source directory '{}': {}",
                args.source_directory.display(),
                e
            );
            std::process::exit(1);
        }
    };

    // Check if there are enough files to copy
    if files.len() < args.number_of_files {
        eprintln!(
            "Error: Not enough files to copy. Available: {}, Requested: {}.",
            files.len(),
            args.number_of_files
        );
        std::process::exit(1);
    }

    // Shuffle the list and select the specified number of random files
    let mut rng = rand::thread_rng();
    let selected_files = files
        .choose_multiple(&mut rng, args.number_of_files)
        .cloned()
        .collect::<Vec<PathBuf>>();

    // Initialize the progress bar
    let progress_bar = ProgressBar::new(args.number_of_files as u64);
    progress_bar.set_style(
        ProgressStyle::with_template(
            "[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
        )
        .unwrap()
        .progress_chars("#>-"),
    );
    progress_bar.set_message("Copying files");

    // Copy the selected files to the destination directory
    for file in selected_files {
        let file_name = match file.file_name() {
            Some(name) => name,
            None => {
                eprintln!(
                    "Warning: Skipping file with invalid name '{}'.",
                    file.display()
                );
                progress_bar.inc(1);
                continue;
            }
        };
        let dest_path = args.destination_directory.join(file_name);
        if let Err(e) = fs::copy(&file, &dest_path) {
            eprintln!(
                "Error: Failed to copy '{}' to '{}': {}",
                file.display(),
                dest_path.display(),
                e
            );
            progress_bar.finish_with_message("Failed");
            std::process::exit(1);
        }
        progress_bar.inc(1);
    }

    progress_bar.finish_with_message("Done");

    println!(
        "Successfully copied {} files from '{}' to '{}'.",
        args.number_of_files,
        args.source_directory.display(),
        args.destination_directory.display()
    );
}

fn print_usage_and_exit() {
    eprintln!(
        "Usage: copy_random_files <source_directory> <destination_directory> <number_of_files>"
    );
    std::process::exit(1);
}
