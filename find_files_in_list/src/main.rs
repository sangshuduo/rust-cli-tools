use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Result};
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, Error as WalkDirError, WalkDir};

/// Builds a map of file stems (lowercased) -> full path of the *first* encountered file.
/// Also collects any WalkDir errors into a separate Vec so we can report them.
fn build_stem_map(root_dir: &str) -> (HashMap<String, PathBuf>, Vec<WalkDirError>) {
    let mut entries = Vec::new();
    let mut errors = Vec::new();

    // Gather all entries (ok and err)
    for entry_result in WalkDir::new(root_dir) {
        match entry_result {
            Ok(entry) => entries.push(entry),
            Err(err) => errors.push(err),
        }
    }

    // We only keep files, not directories
    let entries: Vec<DirEntry> = entries
        .into_iter()
        .filter(|e| e.file_type().is_file())
        .collect();

    // Create a progress bar for building the map
    let pb = ProgressBar::new(entries.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.yellow} Building map [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) - {msg}")
            .unwrap()
            .progress_chars("##-"),
    );

    let mut map = HashMap::new();

    // Process each file entry, extracting the stem and storing in the map
    for entry in entries {
        let path = entry.path();
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            let stem_lower = stem.to_lowercase();
            // Insert if not already present
            map.entry(stem_lower).or_insert_with(|| path.to_path_buf());
        }
        pb.inc(1);
    }

    pb.finish_with_message("Stem map built.");

    (map, errors)
}

fn main() -> Result<()> {
    // Command-line usage:
    //   cargo run -- <list_file> <output_directory> [optional_prefix]
    //
    // If [optional_prefix] is present, only lines in <list_file> that start with that prefix
    // are processed. Otherwise, all lines.

    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!(
            "Usage: {} <list_file> <output_directory> [optional_prefix]",
            args[0]
        );
        std::process::exit(1);
    }

    let list_file = &args[1];
    let output_dir = &args[2];
    let optional_prefix = args.get(3).map(|s| s.as_str());

    // 1. Read lines from list_file, collecting line-read errors
    let file = File::open(list_file)?;
    let reader = BufReader::new(file);

    let mut lines = Vec::new();
    let mut line_read_errors = Vec::new();

    for line_result in reader.lines() {
        match line_result {
            Ok(line) => {
                let trimmed = line.trim().to_string();
                if !trimmed.is_empty() {
                    lines.push(trimmed);
                }
            }
            Err(e) => {
                // Collect the error
                line_read_errors.push(e);
            }
        }
    }

    // Report line-read errors, if any
    if !line_read_errors.is_empty() {
        eprintln!("Errors occurred while reading lines from '{list_file}':");
        for (i, err) in line_read_errors.iter().enumerate() {
            eprintln!("  {}. {}", i + 1, err);
        }
        // Decide if you want to stop or continue. Here we continue.
    }

    // 2. Optionally filter lines by prefix
    if let Some(prefix) = optional_prefix {
        lines.retain(|line| line.starts_with(prefix));
    }

    // 3. Build the stem map of the current directory (.) and collect any WalkDir errors
    let (stem_map, walkdir_errors) = build_stem_map(".");

    // Report WalkDir errors, if any
    if !walkdir_errors.is_empty() {
        eprintln!("Errors occurred while scanning the directory for files:");
        for (i, err) in walkdir_errors.iter().enumerate() {
            eprintln!("  {}. {}", i + 1, err);
        }
        // Again, decide if you want to stop here or continue. We'll continue.
    }

    // Ensure the output directory exists
    fs::create_dir_all(output_dir)?;

    // 4. Prepare a progress bar for the copy phase
    let pb = ProgressBar::new(lines.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} Copying files [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) - {msg}")
            .unwrap()
            .progress_chars("##-"),
    );

    // 5. Copy files according to the list
    for line in &lines {
        // Show which file is being processed
        pb.set_message(format!("Searching: {line}"));

        // Extract the stem from the list line itself (in case user wrote "myfile.txt")
        let line_path = Path::new(line);
        let line_stem_raw = match line_path.file_stem() {
            Some(s) => s.to_string_lossy().to_string(),
            None => line.clone(), // fallback if no stem
        };
        let line_stem_lower = line_stem_raw.to_lowercase();

        // Lookup in the map
        if let Some(found_path) = stem_map.get(&line_stem_lower) {
            // found_path is the actual file on disk
            let file_name = found_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            let mut dest_path = PathBuf::from(output_dir);
            dest_path.push(&file_name);

            // Optional: skip if the file already exists in the destination
            if dest_path.exists() {
                eprintln!(
                    "Skipping, file already exists in destination: {:?}",
                    dest_path
                );
            } else {
                // Copy the file
                pb.set_message(format!("Copying: {file_name}"));
                if let Err(e) = fs::copy(found_path, &dest_path) {
                    eprintln!("Failed to copy '{found_path:?}' to '{dest_path:?}': {e}");
                }
            }
        } else {
            // If not found, report it
            eprintln!(
                "No matching file for '{}' (stem '{}') found in the directory.",
                line, line_stem_lower
            );
        }

        pb.inc(1);
    }

    pb.finish_with_message("All done copying!");

    Ok(())
}
