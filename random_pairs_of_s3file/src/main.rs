use clap::Parser;
use rand::seq::SliceRandom;
use serde::Serialize;
use std::collections::HashSet;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};

// AWS SDK for Rust (1.x)
use aws_config::{load_defaults, BehaviorVersion};
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::types::Object;
use aws_sdk_s3::Client;

/// Command-line arguments (all required, no defaults)
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Number of pairs to generate
    #[arg(long, required = true)]
    num_pairs: usize,

    /// Name of the S3 bucket
    #[arg(long, required = true)]
    bucket: String,

    /// Directory (prefix) in the bucket (e.g. "image/")
    #[arg(long, required = true)]
    directory: String,

    /// URL prefix to form the final URL (e.g. "https://api.example.com/s3/api/v1/resource?url=s3://")
    #[arg(long, required = true)]
    url_prefix: String,

    /// File containing keys to exclude
    #[arg(long, required = false)]
    exclude_file: Option<String>,
}

#[derive(Serialize)]
struct PairsOutput {
    pairs: Vec<Pair>,
}

#[derive(Serialize)]
struct Pair {
    source: String,
    candidate: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let num_pairs = args.num_pairs;
    let bucket_name = &args.bucket;
    let directory_prefix = &args.directory;
    let url_prefix = &args.url_prefix;

    // Read excluded keys from file if provided
    let excluded_keys: HashSet<String> = if let Some(exclude_file_path) = args.exclude_file {
        let file = File::open(&exclude_file_path)?;
        BufReader::new(file).lines().map_while(Result::ok).collect()
    } else {
        HashSet::new()
    };

    let shared_config = load_defaults(BehaviorVersion::latest()).await;
    let s3_client = Client::new(&shared_config);

    let resp = s3_client
        .list_objects_v2()
        .bucket(bucket_name)
        .prefix(directory_prefix)
        .send()
        .await;

    let output = match resp {
        Ok(o) => o,
        Err(SdkError::ServiceError(e)) => {
            eprintln!("Service error: {:#?}", e);
            return Ok(());
        }
        Err(e) => {
            eprintln!("Other error listing objects: {:?}", e);
            return Ok(());
        }
    };

    // Extract all object keys
    let objects: &[Object] = output.contents();
    let all_keys: Vec<String> = objects
        .iter()
        .filter_map(|obj| obj.key().map(str::to_string))
        .filter(|key| !excluded_keys.contains(key))
        .collect();

    if all_keys.len() < 2 {
        eprintln!(
            "Not enough objects to generate pairs. Found only {} object(s).",
            all_keys.len()
        );
        return Ok(());
    }

    // Generate all unique pairs (source, candidate) where source != candidate
    let mut all_pairs = Vec::new();
    for (i, source) in all_keys.iter().enumerate() {
        // check if source is empty
        if source.is_empty() || source.ends_with('/') {
            continue;
        }
        for (j, candidate) in all_keys.iter().enumerate() {
            // check if candidate is is_empty
            if candidate.is_empty() || candidate.ends_with('/') {
                continue;
            }
            if i != j {
                all_pairs.push(Pair {
                    source: format!("{}{}/{}", url_prefix, bucket_name, source),
                    candidate: format!("{}{}/{}", url_prefix, bucket_name, candidate),
                });
            }
        }
    }

    let max_pairs_possible = all_pairs.len();
    if num_pairs > max_pairs_possible {
        eprintln!(
            "Requested {} pairs, but only {} unique pairs can be generated with {} objects.",
            num_pairs,
            max_pairs_possible,
            all_keys.len()
        );
    }

    // Shuffle and take the requested number of pairs
    let mut rng = rand::thread_rng();
    all_pairs.shuffle(&mut rng);

    let selected_pairs: Vec<Pair> = all_pairs.into_iter().take(num_pairs).collect();

    if selected_pairs.len() < num_pairs {
        eprintln!(
            "Requested {} pairs, but only {} unique pairs could be generated with {} objects.",
            num_pairs,
            selected_pairs.len(),
            all_keys.len()
        );
    }

    // Print JSON output
    let output_json = PairsOutput {
        pairs: selected_pairs,
    };
    println!("{}", serde_json::to_string_pretty(&output_json)?);

    Ok(())
}
