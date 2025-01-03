use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Result};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Build a map of file stems (lowercased) to the full path of the *first* encountered file.
/// If multiple files share the same stem, only the first is stored.
/// Adapt if you need to handle duplicates differently (e.g., store Vec<PathBuf>).
fn build_stem_map(root_dir: &str) -> HashMap<String, PathBuf> {
    // 1. Collect all entries first, so we know how many there are.
    let entries: Vec<_> = WalkDir::new(root_dir)
        .into_iter()
        .filter_map(|e| e.ok()) // ignore errors
        .filter(|e| e.file_type().is_file()) // only files
        .collect();

    // 2. Create a progress bar for building the stem map
    let pb = ProgressBar::new(entries.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.yellow} Building map [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) - {msg}")
            .unwrap()
            .progress_chars("##-"),
    );

    let mut map = HashMap::new();

    // 3. Iterate over each file entry, update the progress bar
    for entry in entries {
        let path = entry.path();

        // Obtain the stem (filename without extension) in lowercase
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            let stem_lower = stem.to_lowercase();

            // Insert into the map if not already present
            map.entry(stem_lower).or_insert_with(|| path.to_path_buf());
        }

        pb.inc(1);
    }

    pb.finish_with_message("Stem map built.");
    map
}

fn main() -> Result<()> {
    // Usage:
    //   cargo run -- <list_file> <output_directory> [optional_prefix]
    //
    // If [optional_prefix] is present, only lines in <list_file> that
    // start with that prefix are processed. Otherwise, all lines.

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

    // If a third argument is given, treat it as a prefix filter
    let optional_prefix = args.get(3);

    // 1. Read lines from list_file
    let file = File::open(list_file)?;
    let reader = BufReader::new(file);
    let mut lines: Vec<String> = reader
        .lines()
        .map_while(|l| l.ok()) // discard IO errors
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    // 2. Optionally filter lines by prefix
    if let Some(prefix) = optional_prefix {
        lines.retain(|line| line.starts_with(prefix));
    }

    // 3. Build a map of (stem -> path) from the current directory tree (or any dir you want).
    //    This step now has its own progress bar.
    let stem_map = build_stem_map(".");

    // Ensure the output directory exists
    fs::create_dir_all(output_dir)?;

    // 4. Create and configure a progress bar for the copy phase
    let pb = ProgressBar::new(lines.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} Copying files [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) - {msg}")
            .unwrap()
            .progress_chars("##-"),
    );

    // 5. For each line, parse out the stem ignoring any extension the user might have included
    for line in &lines {
        pb.set_message(format!("Searching: {line}"));

        // Extract the "stem" from the line itself. This handles if
        // the list file says "myfile.txt" or "MyDocument" etc.
        let line_path = Path::new(&line);
        let line_stem_raw = match line_path.file_stem() {
            Some(s) => s.to_string_lossy().to_string(),
            None => line.clone(), // fallback if file_stem() is None
        };

        let line_stem_lower = line_stem_raw.to_lowercase();

        // Lookup in the map
        if let Some(found_path) = stem_map.get(&line_stem_lower) {
            // found_path is the actual file path, with extension
            let file_name = found_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            let mut dest_path = PathBuf::from(output_dir);
            dest_path.push(&file_name);

            // Optionally skip if file already exists in dest
            if dest_path.exists() {
                eprintln!(
                    "Skipping, file already exists in destination: {:?}",
                    dest_path
                );
            } else {
                pb.set_message(format!("Copying:  {}", file_name));
                if let Err(e) = fs::copy(found_path, &dest_path) {
                    eprintln!("Failed to copy {:?} => {:?}: {}", found_path, dest_path, e);
                }
            }
        } else {
            eprintln!(
                "No matching file for '{}' (stem '{}') in directory.",
                line, line_stem_lower
            );
        }

        pb.inc(1);
    }

    pb.finish_with_message("All done copying!");
    Ok(())
}
