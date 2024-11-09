use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Get command-line arguments
    let args: Vec<String> = env::args().collect();

    if args.len() != 4 {
        eprintln!("Usage: {} <directory> <postfix> <expected_count>", args[0]);
        std::process::exit(1);
    }

    let dir = &args[1];
    let postfix = &args[2];
    let expected_count: usize = match args[3].parse() {
        Ok(n) if n > 0 => n,
        _ => {
            eprintln!("Error: Expected count must be a positive integer.");
            std::process::exit(1);
        }
    };

    // Collect filenames from the directory
    let filenames = match get_filenames(dir) {
        Ok(names) => names,
        Err(e) => {
            eprintln!("Error reading directory '{}': {}", dir, e);
            std::process::exit(1);
        }
    };

    // Map base names to available indices
    let mut base_name_map: HashMap<String, HashSet<usize>> = HashMap::new();

    for filename in filenames {
        let path = Path::new(&filename);
        if let Some((base_name, index)) = extract_base_name_and_index(path, postfix) {
            base_name_map.entry(base_name).or_default().insert(index);
        }
    }

    // Check for missing indices for each base name
    let mut bases_with_missing_files = Vec::new();

    for (base_name, indices) in &base_name_map {
        let mut missing_indices = Vec::new();
        for i in 0..expected_count {
            if !indices.contains(&i) {
                missing_indices.push(i);
            }
        }
        if !missing_indices.is_empty() {
            bases_with_missing_files.push((base_name.clone(), missing_indices));
        }
    }

    // Display the result
    if bases_with_missing_files.is_empty() {
        println!(
            "All base names have all {} files with postfix '{}' in '{}'.",
            expected_count, postfix, dir
        );
    } else {
        println!("Base names missing files in directory '{}':", dir);
        for (base_name, missing_indices) in bases_with_missing_files {
            println!("Base name: {}", base_name);
            println!("Missing files:");
            for index in missing_indices {
                let missing_file = format!("{}{}{}.jpg", base_name, postfix, index);
                println!("  {}", missing_file);
            }
            println!();
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

fn extract_base_name_and_index(path: &Path, postfix: &str) -> Option<(String, usize)> {
    let filename = path.file_stem()?.to_str()?;
    if let Some(pos) = filename.rfind(postfix) {
        let base_name = &filename[..pos];
        let index_str = &filename[pos + postfix.len()..];
        if let Ok(index) = index_str.parse::<usize>() {
            return Some((base_name.to_string(), index));
        }
    }
    None
}
