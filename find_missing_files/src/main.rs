use std::collections::HashSet;
use std::env;
use std::fs;

fn main() {
    // Get command-line arguments for directory paths, postfix, and expected file count
    let args: Vec<String> = env::args().collect();

    if args.len() != 5 {
        eprintln!(
            "Usage: {} <dir1> <dir2> <postfix> <expected_count>",
            args[0]
        );
        std::process::exit(1);
    }

    let dir1 = &args[1];
    let dir2 = &args[2];
    let postfix = &args[3];
    let expected_count: usize = match args[4].parse() {
        Ok(n) if n > 0 => n,
        _ => {
            eprintln!("Error: Expected count must be a positive integer.");
            std::process::exit(1);
        }
    };

    // Collect base filenames from dir1
    let dir1_basenames = match get_basenames(dir1) {
        Ok(names) => names,
        Err(e) => {
            eprintln!("Error reading directory '{}': {}", dir1, e);
            std::process::exit(1);
        }
    };

    // Collect filenames from dir2
    let dir2_filenames = match get_filenames(dir2) {
        Ok(names) => names,
        Err(e) => {
            eprintln!("Error reading directory '{}': {}", dir2, e);
            std::process::exit(1);
        }
    };

    // Create a HashSet for quick lookup
    let dir2_filenames_set: HashSet<String> = dir2_filenames.into_iter().collect();

    // Check for each basename if all expected files exist in dir2
    let mut files_with_missing = Vec::new();

    for basename in dir1_basenames {
        let mut missing_files = Vec::new();
        for i in 0..expected_count {
            let filename = format!("{}{}{}.jpg", basename, postfix, i);
            if !dir2_filenames_set.contains(&filename) {
                missing_files.push(filename);
            }
        }
        if !missing_files.is_empty() {
            files_with_missing.push((basename, missing_files));
        }
    }

    // Display the result
    if files_with_missing.is_empty() {
        println!(
            "All files in '{}' have all {} corresponding files in '{}'.",
            dir1, expected_count, dir2
        );
    } else {
        println!(
            "Files in '{}' without all {} corresponding files in '{}':",
            dir1, expected_count, dir2
        );
        for (basename, missing_files) in files_with_missing {
            println!("Base name: {}", basename);
            println!("Missing files:");
            for file in missing_files {
                println!("  {}", file);
            }
            println!();
        }
    }
}

fn get_basenames(dir: &str) -> Result<Vec<String>, std::io::Error> {
    let mut basenames = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        // Check if the entry is a file with .jpg extension
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ext.eq_ignore_ascii_case("jpg") {
                    if let Some(filename) = path.file_stem().and_then(|f| f.to_str()) {
                        basenames.push(filename.to_string());
                    }
                }
            }
        }
    }

    Ok(basenames)
}

fn get_filenames(dir: &str) -> Result<Vec<String>, std::io::Error> {
    let mut filenames = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        // Check if the entry is a file with .jpg extension
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ext.eq_ignore_ascii_case("jpg") {
                    if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                        filenames.push(filename.to_string());
                    }
                }
            }
        }
    }

    Ok(filenames)
}
