//! `hog` - Calvin Structured Data Recorder
//!
//! A utility for recording structured data produced by applications into
//! Calvin storage files. Similar to the hobbes `hog` program, it reads
//! data from producers and writes it to structured files.

use clap::Parser;
use std::path::PathBuf;

/// Calvin Structured Data Recorder - record structured data to files.
#[derive(Parser, Debug)]
#[command(name = "hog", version, about = "Calvin Structured Data Recorder")]
struct Args {
    /// The directory to store data files.
    #[arg(short, long, default_value = ".")]
    dir: PathBuf,

    /// The storage group name.
    #[arg(short, long)]
    group: Option<String>,

    /// Port to listen for data producers.
    #[arg(short, long, default_value = "8473")]
    port: u16,

    /// Run in batch mode (process and exit).
    #[arg(short, long)]
    batch: bool,

    /// Show statistics about stored data.
    #[arg(short, long)]
    stat: bool,

    /// List all storage groups in the directory.
    #[arg(short, long)]
    list: bool,

    /// Compact/optimize storage files.
    #[arg(short, long)]
    compact: bool,
}

fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    if args.list {
        list_groups(&args.dir);
        return;
    }

    if args.stat {
        if let Some(ref group) = args.group {
            show_stats(&args.dir, group);
        } else {
            eprintln!("Error: --group is required with --stat");
            std::process::exit(1);
        }
        return;
    }

    // Create or open the storage group
    let group_name = args.group.unwrap_or_else(|| "default".to_string());

    match calvin_rs::storage::StorageGroup::new(&group_name, &args.dir) {
        Ok(group) => {
            println!(
                "Storage group '{}' ready at {}",
                group.name,
                group.directory.display()
            );

            if args.batch {
                println!("Running in batch mode...");
                // In batch mode, process available data and exit
                if let Err(e) = group.save_metadata() {
                    eprintln!("Error saving metadata: {}", e);
                }
                println!("Batch processing complete.");
            } else {
                println!("Listening for data producers on port {}...", args.port);
                println!("Press Ctrl+C to stop.");

                // Start listening for data producers
                match start_listener(&group, args.port) {
                    Ok(()) => {}
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Error creating storage group: {}", e);
            std::process::exit(1);
        }
    }
}

fn list_groups(dir: &PathBuf) {
    println!("Storage groups in {}:", dir.display());
    match std::fs::read_dir(dir) {
        Ok(entries) => {
            let mut found = false;
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "json")
                    && path
                        .file_name()
                        .map_or(false, |n| n.to_string_lossy().ends_with(".meta.json"))
                {
                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .strip_suffix(".meta")
                        .unwrap_or("unknown");
                    println!("  {}", name);
                    found = true;
                }
            }
            if !found {
                println!("  (no storage groups found)");
            }
        }
        Err(e) => {
            eprintln!("Error reading directory: {}", e);
        }
    }
}

fn show_stats(dir: &PathBuf, group_name: &str) {
    match calvin_rs::storage::StorageGroup::load_metadata(group_name, dir) {
        Ok(group) => {
            println!("Storage group: {}", group.name);
            println!("  Created: {}", group.metadata.created_at);
            println!("  Series:");
            let series = group.list_series();
            if series.is_empty() {
                println!("    (no series)");
            } else {
                for s in series {
                    println!(
                        "    {} - {} elements ({} bytes)",
                        s.name, s.count, s.length
                    );
                }
            }
        }
        Err(e) => {
            eprintln!("Error loading storage group: {}", e);
        }
    }
}

fn start_listener(
    _group: &calvin_rs::storage::StorageGroup,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::net::TcpListener;

    let listener = TcpListener::bind(format!("0.0.0.0:{}", port))?;
    tracing::info!("Hog listener started on port {}", port);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let peer = stream
                    .peer_addr()
                    .map(|a| a.to_string())
                    .unwrap_or_else(|_| "unknown".to_string());
                tracing::info!("Data producer connected: {}", peer);
                // Handle the connection in a separate thread
                std::thread::spawn(move || {
                    tracing::info!("Handling data from {}", peer);
                    // Data handling would go here
                });
            }
            Err(e) => {
                tracing::error!("Accept error: {}", e);
            }
        }
    }

    Ok(())
}
