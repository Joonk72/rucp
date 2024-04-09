# Rust Multi-Threaded Folder Copier

This is a Rust console program that copies a folder using multi-threads and displays copying progress, including copied files, estimated time, and elapsed time.

## Usage
1. Clone the repository.
2. Compile and run the program.
3. Provide the source directory, target directory, and number of threads as command-line arguments.

cargo run [source_folder] [target_folder] [thread_number]

## Example
cargo run ./src ./copied_folder 8

## Dependencies
- `indicatif` for progress bar functionality.
- `walkdir` for directory traversal.
- `structopt` for command-line argument parsing.

## Structure
- `main.rs`: Main program file.
- `README.md`: Instructions and information about the program.

## How to Run
1. Compile the program using `cargo build`.
2. Run the program with the source directory, target directory, and number of threads as arguments.
## Example
./target/release/rucp ./src ./copied_folder 8


Feel free to contribute or report issues!