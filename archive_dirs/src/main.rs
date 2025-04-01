use chrono::Local;
use humansize::{format_size, BINARY};
use indicatif::{ProgressBar, ProgressStyle};
use log::{error, info, warn};
use std::env;
use std::fs;
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;

fn setup_logging(product_id: &str) -> io::Result<()> {
    // Create logs directory if it doesn't exist
    let logs_dir = PathBuf::from("logs");
    if !logs_dir.exists() {
        fs::create_dir_all(&logs_dir)?;
    }

    // Create log file with timestamp
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let log_file = logs_dir.join(format!("archive_{}_{}.log", product_id, timestamp));

    // Initialize file logger
    let file = File::create(&log_file)?;
    env_logger::Builder::new()
        .target(env_logger::Target::Pipe(Box::new(file)))
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .init();

    info!("Logging initialized. Log file: {}", log_file.display());
    Ok(())
}

fn print_usage(program: &str) {
    let usage = format!(
        "Usage: {} [product_id] [custom_archive_dir]\n\
         {} --help\n\n\
         Arguments:\n\
           product_id         The product ID to archive (required)\n\
           custom_archive_dir Optional custom archive directory name\n\n\
         Example:\n\
           {} wish\n\
           {} wish custom-archive",
        program, program, program, program
    );
    eprintln!("{}", usage);
    error!("{}", usage);
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    // Check for --help flag
    if args.len() > 1 && (args[1] == "--help" || args[1] == "-h") {
        print_usage(&args[0]);
        process::exit(0);
    }

    // Check if product_id parameter is provided
    if args.len() < 2 {
        eprintln!("Error: Product ID parameter is required.");
        print_usage(&args[0]);
        process::exit(1);
    }

    let product_id = &args[1];

    // Setup logging
    setup_logging(product_id)?;
    info!("Starting archive operation for product: {}", product_id);

    let pattern = format!("product_images-{}-202*", product_id);
    let default_archive = format!("product_images-{}-archive", product_id);

    // Set archive directory (use custom if provided, otherwise use default)
    let archive_dir = if args.len() >= 3 {
        PathBuf::from(&args[2])
    } else {
        PathBuf::from(&default_archive)
    };

    // Create archive directory if it doesn't exist
    if !archive_dir.exists() {
        println!("Creating archive directory: {}", archive_dir.display());
        info!("Creating archive directory: {}", archive_dir.display());
        fs::create_dir_all(&archive_dir)?;
    }

    // Find all matching product image directories
    println!("Finding product image directories for '{}'...", product_id);
    info!("Finding product image directories for '{}'...", product_id);
    let current_dir = env::current_dir()?;
    let mut dirs: Vec<PathBuf> = vec![];

    for entry in fs::read_dir(&current_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            if let Some(name) = path.file_name() {
                let name_str = name.to_string_lossy();
                if glob_matches(&name_str, &pattern) {
                    dirs.push(path);
                }
            }
        }
    }

    // Sort directories by name (which contains the date)
    dirs.sort();

    // Check if any directories were found
    if dirs.is_empty() {
        let error_msg = format!(
            "No product image directories found matching pattern '{}'!",
            pattern
        );
        eprintln!("{}", error_msg);
        error!("{}", error_msg);
        process::exit(1);
    }

    // Print summary before moving
    println!(
        "Files will be moved from the following directories in this order (oldest to newest):"
    );
    info!("Files will be moved from the following directories in this order (oldest to newest):");
    for dir in &dirs {
        let dir_name = dir.file_name().unwrap().to_string_lossy();
        println!("  {}", dir_name);
        info!("  {}", dir_name);
    }

    println!("\nDisk usage of directories:");
    info!("Disk usage of directories:");
    let mut total_size = 0u64;
    let mut total_files = 0;
    for dir in &dirs {
        let dir_size = calculate_dir_size(dir)?;
        let file_count = count_files(dir)?;
        total_size += dir_size;
        total_files += file_count;
        let msg = format!(
            "  {}: {} ({} files)",
            dir.file_name().unwrap().to_string_lossy(),
            format_size(dir_size, BINARY),
            file_count
        );
        println!("{}", msg);
        info!("{}", msg);
    }

    let total_msg = format!("Total disk usage: {}", format_size(total_size, BINARY));
    println!("{}", total_msg);
    info!("{}", total_msg);

    let files_msg = format!("Total files to be moved: {}", total_files);
    println!("{}", files_msg);
    info!("{}", files_msg);

    let note_msg = "Note: Files with duplicate names will be overwritten by newer versions.";
    println!("{}", note_msg);
    info!("{}", note_msg);

    // Ask for confirmation
    print!("Do you want to proceed? (y/n): ");
    io::stdout().flush()?;
    let mut response = String::new();
    io::stdin().read_line(&mut response)?;
    if !response.trim().eq_ignore_ascii_case("y") {
        println!("Operation cancelled.");
        info!("Operation cancelled by user.");
        process::exit(0);
    }

    // Move files from each directory into the flat archive directory
    println!("Moving files to {}...", archive_dir.display());
    info!("Moving files to {}...", archive_dir.display());
    let mut moved_count = 0;
    let mut overwrite_count = 0;

    for dir in &dirs {
        let dir_name = dir.file_name().unwrap().to_string_lossy();
        println!("Moving files from {}...", dir_name);
        info!("Moving files from {}...", dir_name);

        let mut dir_moved = 0;
        let mut dir_overwrite = 0;

        // Count total files in directory first
        let total_files = count_files(dir)?;

        // Create progress bar
        let pb = ProgressBar::new(total_files as u64);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%)")
            .unwrap()
            .progress_chars("#>-"));

        // Move only files to the flat archive directory
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let filename = path.file_name().unwrap();
                let dest_path = archive_dir.join(filename);

                // Check if file already exists in destination
                if dest_path.exists() {
                    dir_overwrite += 1;
                }

                // Move the file and verify it was moved successfully
                match fs::rename(&path, &dest_path) {
                    Ok(_) => {
                        // Verify the file exists in destination and not in source
                        if dest_path.exists() && !path.exists() {
                            dir_moved += 1;
                            pb.inc(1);
                        } else {
                            warn!("File {} was not properly moved", path.display());
                            // Try to remove the source file if it still exists
                            if path.exists() {
                                if let Err(e) = fs::remove_file(&path) {
                                    warn!("Failed to remove source file {}: {}", path.display(), e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to move file {}: {}", path.display(), e);
                    }
                }
            }
        }

        pb.finish_with_message("completed");
        moved_count += dir_moved;
        overwrite_count += dir_overwrite;
        let move_msg = format!(
            "  Moved {} files from {} (overwrote {} files)",
            dir_moved, dir_name, dir_overwrite
        );
        println!("{}", move_msg);
        info!("{}", move_msg);
    }

    // Remove empty directories
    println!("\nRemoving empty directories...");
    info!("Removing empty directories...");
    for dir in &dirs {
        // Double check if directory is empty before removing
        if let Ok(mut entries) = fs::read_dir(dir) {
            if entries.next().is_none() {
                if let Err(e) = fs::remove_dir(dir) {
                    let warn_msg = format!(
                        "Warning: Failed to remove directory {}: {}",
                        dir.display(),
                        e
                    );
                    eprintln!("{}", warn_msg);
                    warn!("{}", warn_msg);
                } else {
                    let remove_msg =
                        format!("  Removed: {}", dir.file_name().unwrap().to_string_lossy());
                    println!("{}", remove_msg);
                    info!("{}", remove_msg);
                }
            } else {
                let warn_msg = format!(
                    "Warning: Directory {} is not empty, skipping removal",
                    dir.display()
                );
                eprintln!("{}", warn_msg);
                warn!("{}", warn_msg);
            }
        }
    }

    println!("\nOperation completed.");
    info!("Operation completed.");

    let success_msg = format!(
        "Successfully moved {} files to {}",
        moved_count,
        archive_dir.display()
    );
    println!("{}", success_msg);
    info!("{}", success_msg);

    // Print summary of overwritten files
    println!("\nSummary:");
    info!("Summary:");
    let summary_msg = format!("  - Total files moved: {}", moved_count);
    println!("{}", summary_msg);
    info!("{}", summary_msg);

    let overwrite_msg = format!("  - Files overwritten: {}", overwrite_count);
    println!("{}", overwrite_msg);
    info!("{}", overwrite_msg);

    let unique_msg = format!(
        "  - Unique files in archive: {}",
        moved_count - overwrite_count
    );
    println!("{}", unique_msg);
    info!("{}", unique_msg);

    // Print disk usage comparison
    println!("\nDisk Usage:");
    info!("Disk Usage:");
    let initial_msg = format!(
        "  - Initial total size: {}",
        format_size(total_size, BINARY)
    );
    println!("{}", initial_msg);
    info!("{}", initial_msg);

    // Print final archive directory size
    let final_size = calculate_dir_size(&archive_dir)?;
    let final_size_msg = format!(
        "  - Final archive size: {}",
        format_size(final_size, BINARY)
    );
    println!("{}", final_size_msg);
    info!("{}", final_size_msg);

    Ok(())
}

// Count files in a directory (non-recursive)
fn count_files(dir: &Path) -> io::Result<usize> {
    let mut count = 0;
    for entry in fs::read_dir(dir)? {
        if entry?.path().is_file() {
            count += 1;
        }
    }
    Ok(count)
}

// Simple glob pattern matching for our specific case
fn glob_matches(value: &str, pattern: &str) -> bool {
    let pattern_parts: Vec<&str> = pattern.split('*').collect();

    if pattern_parts.is_empty() {
        return false;
    }

    // Check if string starts with the first part of the pattern
    if !value.starts_with(pattern_parts[0]) {
        return false;
    }

    // If there's only a prefix pattern with *, we're done
    if pattern_parts.len() == 1 {
        return true;
    }

    // Check ending (for patterns like "prefix*suffix")
    if pattern_parts.len() == 2 && !pattern_parts[1].is_empty() {
        return value.ends_with(pattern_parts[1]);
    }

    // For more complex patterns, this is a simplification
    // In a real implementation, we would use a proper glob crate
    true
}

// Calculate total size of a directory
fn calculate_dir_size(dir: &Path) -> io::Result<u64> {
    let mut total_size = 0u64;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            total_size += path.metadata()?.len();
        }
    }
    Ok(total_size)
}
