use regex::Regex;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, BufRead, Write};

#[derive(Default, Debug)]
struct VideoMetrics {
    scene_detect_fps: Option<f64>,
    scene_detect_time: Option<f64>,
    ocr_fps: Option<f64>,
    ocr_time: Option<f64>,
    logo_detect_images: Option<i32>,
    logo_detect_fps: Option<f64>,
    logo_detect_time: Option<f64>,
    object_detect_images: Option<i32>,
    object_detect_fps: Option<f64>,
    object_detect_time: Option<f64>,
    transcribe_time: Option<f64>,
    scene_description_time: Option<f64>,
    process_video_time: Option<f64>,
}

fn fmt_float(val: Option<f64>, suffix: &str) -> String {
    match val {
        Some(v) => format!("{:.1}{}", v, suffix),
        None => "-".to_string(),
    }
}

fn fmt_int(val: Option<i32>) -> String {
    match val {
        Some(v) => v.to_string(),
        None => "-".to_string(),
    }
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: {} <input_log_file> <output_file>", args[0]);
        eprintln!(
            "Example: {} 20260113-all-perf.log 20260113-all-perf-sorted.log",
            args[0]
        );
        std::process::exit(1);
    }

    let input_file = &args[1];
    let output_file = &args[2];

    // Compile regex patterns
    let summary_re = Regex::new(r"Performance Summary of (input_files/\S+):").unwrap();
    let scene_detect_re =
        Regex::new(r"func=scene_detect\s+file=\S+\s+fps=([\d.]+)\s+time=([\d.]+)s").unwrap();
    let ocr_re = Regex::new(r"func=ocr\s+file=\S+\s+fps=([\d.]+)\s+time=([\d.]+)s").unwrap();
    let logo_detect_re =
        Regex::new(r"func=logo_detect\s+images=(\d+)\s+fps=([\d.]+)\s+time=([\d.]+)s").unwrap();
    let object_detect_re =
        Regex::new(r"func=object_detect\s+images=(\d+)\s+fps=([\d.]+)\s+time=([\d.]+)s").unwrap();
    let transcribe_re = Regex::new(r"func=transcribe\s+file=\S+\s+time=([\d.]+)s").unwrap();
    let scene_desc_re = Regex::new(r"func=scene_description\s+file=\S+\s+time=([\d.]+)s").unwrap();
    let process_video_re = Regex::new(r"func=process_video\s+file=\S+\s+time=([\d.]+)s").unwrap();

    // Parse the log file
    let file = fs::File::open(input_file)?;
    let reader = io::BufReader::new(file);

    let mut videos: HashMap<String, VideoMetrics> = HashMap::new();
    let mut current_video: Option<String> = None;

    for line in reader.lines() {
        let line = line?;

        // Check for Performance Summary line
        if let Some(caps) = summary_re.captures(&line) {
            let full_path = caps.get(1).unwrap().as_str();
            let video_id = full_path.replace("input_files/", "");
            current_video = Some(video_id.clone());
            videos.entry(video_id).or_default();
            continue;
        }

        // Parse performance metrics
        if let Some(ref video_id) = current_video {
            if !line.contains("[PERF]") {
                continue;
            }

            let metrics = videos.get_mut(video_id).unwrap();

            if let Some(caps) = scene_detect_re.captures(&line) {
                metrics.scene_detect_fps = caps.get(1).and_then(|m| m.as_str().parse().ok());
                metrics.scene_detect_time = caps.get(2).and_then(|m| m.as_str().parse().ok());
                continue;
            }

            if let Some(caps) = ocr_re.captures(&line) {
                metrics.ocr_fps = caps.get(1).and_then(|m| m.as_str().parse().ok());
                metrics.ocr_time = caps.get(2).and_then(|m| m.as_str().parse().ok());
                continue;
            }

            if let Some(caps) = logo_detect_re.captures(&line) {
                metrics.logo_detect_images = caps.get(1).and_then(|m| m.as_str().parse().ok());
                metrics.logo_detect_fps = caps.get(2).and_then(|m| m.as_str().parse().ok());
                metrics.logo_detect_time = caps.get(3).and_then(|m| m.as_str().parse().ok());
                continue;
            }

            if let Some(caps) = object_detect_re.captures(&line) {
                metrics.object_detect_images = caps.get(1).and_then(|m| m.as_str().parse().ok());
                metrics.object_detect_fps = caps.get(2).and_then(|m| m.as_str().parse().ok());
                metrics.object_detect_time = caps.get(3).and_then(|m| m.as_str().parse().ok());
                continue;
            }

            if let Some(caps) = transcribe_re.captures(&line) {
                metrics.transcribe_time = caps.get(1).and_then(|m| m.as_str().parse().ok());
                continue;
            }

            if let Some(caps) = scene_desc_re.captures(&line) {
                metrics.scene_description_time = caps.get(1).and_then(|m| m.as_str().parse().ok());
                continue;
            }

            if let Some(caps) = process_video_re.captures(&line) {
                metrics.process_video_time = caps.get(1).and_then(|m| m.as_str().parse().ok());
                continue;
            }
        }
    }

    // Sort videos by process_video_time (descending)
    let mut sorted_videos: Vec<(&String, &VideoMetrics)> = videos
        .iter()
        .filter(|(_, m)| m.process_video_time.is_some())
        .collect();

    sorted_videos.sort_by(|a, b| {
        b.1.process_video_time
            .partial_cmp(&a.1.process_video_time)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Generate output
    let mut output = fs::File::create(output_file)?;

    writeln!(
        output,
        "# Video Processing Performance Report (Sorted by Total Time)"
    )?;
    writeln!(output)?;
    writeln!(output, "Total videos processed: {}", sorted_videos.len())?;
    writeln!(output)?;
    writeln!(output, "| # | Video ID | scene_detect (fps/time) | ocr (fps/time) | logo_detect (imgs/fps/time) | object_detect (imgs/fps/time) | transcribe | scene_desc | total |")?;
    writeln!(output, "|---|----------|-------------------------|----------------|------------------------------|-------------------------------|------------|------------|-------|")?;

    for (i, (vid, data)) in sorted_videos.iter().enumerate() {
        let scene_d = format!(
            "{}/{}",
            fmt_float(data.scene_detect_fps, ""),
            fmt_float(data.scene_detect_time, "s")
        );
        let ocr = format!(
            "{}/{}",
            fmt_float(data.ocr_fps, ""),
            fmt_float(data.ocr_time, "s")
        );
        let logo = format!(
            "{}/{}/{}",
            fmt_int(data.logo_detect_images),
            fmt_float(data.logo_detect_fps, ""),
            fmt_float(data.logo_detect_time, "s")
        );
        let obj = format!(
            "{}/{}/{}",
            fmt_int(data.object_detect_images),
            fmt_float(data.object_detect_fps, ""),
            fmt_float(data.object_detect_time, "s")
        );
        let trans = fmt_float(data.transcribe_time, "s");
        let scene_desc = fmt_float(data.scene_description_time, "s");
        let total = fmt_float(data.process_video_time, "s");

        writeln!(
            output,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            i + 1,
            vid,
            scene_d,
            ocr,
            logo,
            obj,
            trans,
            scene_desc,
            total
        )?;
    }

    println!("Output written to {}", output_file);
    println!("Total videos: {}", sorted_videos.len());
    if let Some((first_vid, first_data)) = sorted_videos.first() {
        println!(
            "Longest: {} at {:.1}s",
            first_vid,
            first_data.process_video_time.unwrap_or(0.0)
        );
    }
    if let Some((last_vid, last_data)) = sorted_videos.last() {
        println!(
            "Shortest: {} at {:.1}s",
            last_vid,
            last_data.process_video_time.unwrap_or(0.0)
        );
    }

    Ok(())
}
