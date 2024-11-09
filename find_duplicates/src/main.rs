use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Get the directory path from command-line arguments
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <directory>", args[0]);
        std::process::exit(1);
    }

    let dir_path = &args[1];

    // Collect filenames in the directory
    let filenames = match get_filenames(dir_path) {
        Ok(names) => names,
        Err(e) => {
            eprintln!("Error reading directory '{}': {}", dir_path, e);
            std::process::exit(1);
        }
    };

    // Map base names to lists of files (with and without extension)
    let mut base_name_map: HashMap<String, Vec<String>> = HashMap::new();

    for filename in filenames {
        let path = Path::new(&filename);
        let base_name = match path.file_stem().and_then(|s| s.to_str()) {
            Some(name) => name.to_string(),
            None => continue, // Skip if unable to get base name
        };

        base_name_map
            .entry(base_name)
            .or_default()
            .push(filename.clone());
    }

    // Find base names that have both files with and without extension
    let mut duplicates = Vec::new();

    for (base_name, files) in &base_name_map {
        let has_extension = files.iter().any(|f| Path::new(f).extension().is_some());
        let has_no_extension = files.iter().any(|f| Path::new(f).extension().is_none());

        if has_extension && has_no_extension {
            duplicates.push(base_name.clone());
        }
    }

    // Display the result
    if duplicates.is_empty() {
        println!("No files found with both extension and without extension.");
    } else {
        println!("Files with and without extension:");
        for base_name in duplicates {
            if let Some(files) = base_name_map.get(&base_name) {
                println!("Base name: {}", base_name);
                for file in files {
                    println!("  {}", file);
                }
                println!();
            }
        }
    }
}

fn get_filenames(dir: &str) -> Result<Vec<String>, std::io::Error> {
    let mut filenames = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        // Check if the entry is a file
        if path.is_file() {
            if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                filenames.push(filename.to_string());
            }
        }
    }

    Ok(filenames)
}
