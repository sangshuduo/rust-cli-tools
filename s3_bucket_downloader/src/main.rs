use aws_config::meta::region::RegionProviderChain;
use aws_config::retry::RetryConfig;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::fs::{self, File};
use std::io::Result;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;
use tokio::runtime::Runtime;

const BINARY: bool = true;

fn format_size(size: u64, binary: bool) -> String {
    let units = if binary {
        ["B", "KiB", "MiB", "GiB", "TiB"]
    } else {
        ["B", "KB", "MB", "GB", "TB"]
    };
    let base = if binary { 1024.0 } else { 1000.0 };
    let mut size = size as f64;
    let mut unit_index = 0;

    while size >= base && unit_index < units.len() - 1 {
        size /= base;
        unit_index += 1;
    }

    format!("{:.2} {}", size, units[unit_index])
}

/// S3 Downloader: Download all files from an S3 bucket with multiple threads.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// S3 bucket name
    #[arg(short, long)]
    bucket: String,

    /// Local directory to download to
    #[arg(short, long)]
    output: String,

    /// Number of worker threads
    #[arg(short, long, default_value_t = 4)]
    workers: usize,

    /// Maximum number of retries for failed downloads
    #[arg(short, long, default_value_t = 3)]
    retries: u32,

    /// File containing list of files to download (one per line)
    #[arg(short, long)]
    file_list: Option<String>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let bucket_name = args.bucket;
    let local_dir = PathBuf::from(args.output);
    let num_workers = args.workers;
    let max_retries = args.retries;

    if !local_dir.exists() {
        fs::create_dir_all(&local_dir).expect("Failed to create output directory");
    }

    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
    let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(region_provider)
        .retry_config(RetryConfig::standard().with_max_attempts(max_retries))
        .load()
        .await;
    let client = Arc::new(Client::new(&config));

    // Default file list name based on bucket
    let default_file_list = format!("{}.files.txt", bucket_name);

    // Get list of files to download
    let keys = if let Some(file_list) = args.file_list {
        println!("Reading file list from: {}", file_list);
        let file = File::open(&file_list).expect("Failed to open file list");
        let reader = BufReader::new(file);
        reader.lines().map_while(Result::ok).collect()
    } else if PathBuf::from(&default_file_list).exists() {
        println!("Reading cached file list from: {}", default_file_list);
        let file = File::open(&default_file_list).expect("Failed to open cached file list");
        let reader = BufReader::new(file);
        reader.lines().map_while(Result::ok).collect()
    } else {
        println!("Listing objects in bucket: {}", bucket_name);
        let keys = list_objects(&client, &bucket_name).await;

        // Always save the file list
        println!("Saving file list to: {}", default_file_list);
        let mut file = File::create(&default_file_list).expect("Failed to create file list");
        for key in &keys {
            writeln!(file, "{}", key).expect("Failed to write to file list");
        }
        println!("File list saved successfully");

        keys
    };

    println!(
        "Found {} files. Starting downloads with {} threads...",
        keys.len(),
        num_workers
    );
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_workers)
        .build_global()
        .unwrap();

    let m = Arc::new(MultiProgress::new());
    let downloaded = Arc::new(AtomicUsize::new(0));
    let failed = Arc::new(AtomicUsize::new(0));
    let downloaded_size = Arc::new(AtomicUsize::new(0));

    // Create overall progress bar
    let total_pb = m.add(ProgressBar::new(keys.len() as u64));
    total_pb.set_style(
        ProgressStyle::with_template(
            "{spinner} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg}",
        )
        .unwrap(),
    );

    // Calculate files per thread
    let files_per_thread = keys.len().div_ceil(num_workers);

    // Create fixed progress bars for each thread
    let thread_pbs: Vec<_> = (0..num_workers)
        .map(|i| {
            let pb = m.add(ProgressBar::new(files_per_thread as u64));
            pb.set_style(
                ProgressStyle::with_template("[{thread}] {spinner} [{elapsed_precise}] [{bar:40.yellow/blue}] {pos}/{len} ({percent}%) {msg}")
                    .unwrap()
            );
            pb.set_message(format!("Thread {}: Starting", i + 1));
            pb
        })
        .collect();

    keys.par_iter().enumerate().for_each(|(i, key)| {
        let client = Arc::clone(&client);
        let bucket = bucket_name.clone();
        let dir = local_dir.clone();
        let key = key.clone();
        let downloaded = Arc::clone(&downloaded);
        let failed = Arc::clone(&failed);
        let downloaded_size = Arc::clone(&downloaded_size);
        let total_pb = total_pb.clone();
        let thread_num = i % num_workers;
        let thread_pb = thread_pbs[thread_num].clone();

        let rt = Runtime::new().unwrap();
        rt.block_on(async move {
            let local_path = dir.join(&key);
            if let Some(parent) = local_path.parent() {
                fs::create_dir_all(parent).expect("Failed to create parent directory");
            }

            match download_object_with_retry(&client, &bucket, &key, max_retries).await {
                Ok(bytes) => {
                    let mut file = File::create(&local_path).expect("Failed to create file");
                    file.write_all(&bytes).expect("Failed to write file");
                    downloaded.fetch_add(1, Ordering::SeqCst);
                    downloaded_size.fetch_add(bytes.len(), Ordering::SeqCst);
                    total_pb.inc(1);
                    thread_pb.inc(1);
                    thread_pb.set_message(format!(
                        "Thread {}: Downloaded {}/{} files",
                        thread_num + 1,
                        downloaded.load(Ordering::SeqCst),
                        files_per_thread
                    ));
                }
                Err(_e) => {
                    failed.fetch_add(1, Ordering::SeqCst);
                    total_pb.inc(1);
                    thread_pb.inc(1);
                    thread_pb.set_message(format!(
                        "Thread {}: Failed {}/{} files",
                        thread_num + 1,
                        failed.load(Ordering::SeqCst),
                        files_per_thread
                    ));
                }
            }
        });
    });

    // Clean up all progress bars
    for pb in thread_pbs {
        pb.finish_and_clear();
    }
    total_pb.finish_with_message("Download complete");
    println!(
        "✅ Total files downloaded: {}",
        downloaded.load(Ordering::SeqCst)
    );
    println!("❌ Total files failed: {}", failed.load(Ordering::SeqCst));
    println!(
        "📦 Total data downloaded: {}",
        format_size(downloaded_size.load(Ordering::SeqCst) as u64, BINARY)
    );
}

async fn list_objects(client: &Client, bucket: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let mut continuation_token = None;

    loop {
        let mut req = client.list_objects_v2().bucket(bucket.to_string());
        if let Some(token) = continuation_token {
            req = req.continuation_token(token);
        }

        match req.send().await {
            Ok(resp) => {
                if let Some(contents) = resp.contents {
                    for obj in contents {
                        if let Some(key) = obj.key {
                            keys.push(key);
                        }
                    }
                }

                if resp.is_truncated.unwrap_or(false) {
                    continuation_token = resp.next_continuation_token;
                } else {
                    break;
                }
            }
            Err(_e) => {
                eprintln!("Failed to list objects: {:?}", _e);
                break;
            }
        }
    }

    keys
}

async fn download_object_with_retry(
    client: &Client,
    bucket: &str,
    key: &str,
    max_retries: u32,
) -> Result<Vec<u8>> {
    let mut retry_count = 0;
    let mut last_error = None;

    while retry_count <= max_retries {
        match download_object(client, bucket, key).await {
            Ok(bytes) => return Ok(bytes),
            Err(e) => {
                last_error = Some(e);
                retry_count += 1;
                if retry_count <= max_retries {
                    tokio::time::sleep(Duration::from_secs(2u64.pow(retry_count))).await;
                }
            }
        }
    }

    Err(last_error
        .unwrap_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "Unknown error")))
}

async fn download_object(client: &Client, bucket: &str, key: &str) -> Result<Vec<u8>> {
    let resp = client
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let data: ByteStream = resp.body;
    let bytes = data
        .collect()
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
        .into_bytes()
        .to_vec();
    Ok(bytes)
}
