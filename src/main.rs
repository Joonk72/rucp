use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use indicatif::{ProgressBar, ProgressStyle};
use std::thread;
use std::sync::mpsc;
use std::time::Instant;
use walkdir::WalkDir;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Args {
    source: String,
    target: String,
    threads: u32,
}

fn main() {
    let args = Args::from_args();

    let source = &args.source;
    let target = &args.target;
    let thread_count = &args.threads;

    // Convert source and target paths to PathBuf for easier manipulation
    let source_path = Path::new(source).to_path_buf();
    let target_path = Path::new(target).to_path_buf();

    // Validate source path
    if !Path::new(source).is_dir() {
        eprintln!("Source must be a directory.");
        return;
    }

    // Create target directory if it doesn't exist
    if !Path::new(target).exists() {
        fs::create_dir_all(target).expect("Failed to create target directory.");
    }

    // Start timer for performance measurement
    let start = Instant::now();

    // Count total files and directories in the source
    println!("Gathering folder structure...");
    let (total_files, _total_size) = count_files_in_dir(source);
    let folders = get_directories(source);
    let files: Vec<PathBuf> = get_files(source);

    // Print total files and directories
    println!("Total files / folders: {:?} / {:?}", total_files, folders.len());

    // Create directories in the target path
    for folder in folders.clone() {
        let relative_path = folder.strip_prefix(source).expect("Failed to strip prefix");
        let dat_folder = target_path.join(relative_path);
        fs::create_dir_all(&dat_folder).expect("Failed to create directory");
    }
    println!("Created all folders in destination.");

    // Set up progress bar for user feedback
    let (tx, _rx): (mpsc::Sender<u64>, mpsc::Receiver<u64>) = mpsc::channel();
    let pb = ProgressBar::new(total_files as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:80.cyan/blue}] {pos}/{len} ({eta}) {percent}% {msg}")
        .expect("Fail to set ProgressStyle template")
        .progress_chars("#>-"));
        
    let pb = Arc::new(Mutex::new(pb));
    let pb_clone = Arc::clone(&pb);

    // Spawn a thread to listen for progress updates and update the progress bar
    let _pb_thread = thread::spawn(move || {
        let mut _received_count = 0 as u64;

        while _received_count < total_files as u64 {
            let mut _received = 0;
            match _rx.recv() {
                Ok(message) => {
                    _received = message;
                    _received_count += 1;   
                    let pb = pb_clone.lock().unwrap();     
                    pb.inc(1);
                }
                Err(e) => {
                    if e.to_string() != "receiving on an empty channel" {
                        eprintln!("RX receive error: {}", e)
                    }
                }
            }
        }

        let msg = format!("\n{}/{} files copied\n", _received_count, total_files);
        let pb = pb.lock().unwrap();
        pb.finish_with_message(msg);
    });

    // Calculate chunk size for parallel processing
    let chunk_size_fp64 = (total_files / *thread_count as u64) as f64;
    let chunk_size = chunk_size_fp64.round() as usize;
    let chunks: Vec<_> = files.chunks(chunk_size).collect();
    let _thread_num = chunks.len();

    // Configure and start the global thread pool for parallel file copying
    rayon::ThreadPoolBuilder::new().num_threads(*thread_count as usize).build_global().unwrap();

    // Spawn threads to copy files in parallel
    rayon::scope(|s| {
        for chunk in chunks {
            let source_path = source_path.clone();
            let target_path = target_path.clone();
            let tx = tx.clone();

            s.spawn(move |_| {
                copy_folder(&source_path, &target_path, tx, &chunk);
            });
        }
    });

    // Wait for all threads to finish and the progress bar to complete
    let _= _pb_thread.join().unwrap();

    // Calculate and print the total elapsed time
    let elapsed = start.elapsed();
    let hours = elapsed.as_secs() / 3600;
    let minutes = (elapsed.as_secs() % 3600) / 60;
    let seconds = elapsed.as_secs() % 60;
    let milliseconds = elapsed.subsec_millis();
    println!("\nElapsed time: {:02}:{:02}:{:02}:{:03}", hours, minutes, seconds, milliseconds);
}

// Function to copy a folder's contents from source to target
fn copy_folder(source: &Path, target: &Path, tx: mpsc::Sender<u64>, files: &[PathBuf]) {

    for file in files {
        // Ensure the source path is a subdirectory of the target path
        let is_valid_src_path = file.starts_with(source);
        if !is_valid_src_path {
            eprintln!("Source path is not a subdirectory of the target path");
            continue;
        }

        // Calculate relative path (clone current_source to avoid move)
        let relative_path = file.strip_prefix(source).expect("Failed to strip prefix");
        let dst_path = target.join(relative_path);

        // check if the destination file is existed.
        let mut _existed_dst_file: bool = false;
        match fs::metadata(dst_path.clone()) {
            Ok(_metadata) => {
                _existed_dst_file = true;
            }
            Err(_e) => {
                _existed_dst_file = false;
            }
        };

        // Skip if destination file is existed already.
        if _existed_dst_file == false {
            match fs::copy(&file, &dst_path) {
                Ok(_) => {tx.send(1).expect("Failed to send message through the channel");}
                Err(e) => {eprintln!("Failed to copy {} => {:?}: {}", file.display(), dst_path, e);},
            }            
        }
        else {
            tx.send(1).expect("Failed to send message through the channel");
        }
    }
}

// Function to count the total number of files in a directory
fn count_files_in_dir(path: &str) -> (u64, u64) {
    let walkdir = WalkDir::new(path);
    let mut files_count = 0;
    let mut total_size = 0;

    for entry in walkdir.into_iter() {
        if entry.is_ok() {
            let dir_entry = entry.unwrap();
            if dir_entry.file_type().is_file() {
                files_count += 1;
                total_size += dir_entry.metadata().unwrap().len();
            }
        }
    }

    (files_count, total_size)
}

// Function to get a list of all directories in a given path
fn get_directories(path: &str) -> Vec<PathBuf> {
    let mut directories = Vec::new();
    for entry in WalkDir::new(path) {
        let entry = entry.unwrap();
        if entry.file_type().is_dir() {
            directories.push(entry.path().to_path_buf());
        }
    }
    directories
}

// Function to get a list of all files in a given path
fn get_files(path: &str) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
            files.push(entry.path().to_path_buf());
    }
    files
}
