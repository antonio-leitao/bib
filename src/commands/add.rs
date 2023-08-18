use sha2::{Digest, Sha256};
use std::fs;
use std::process::Command;

pub fn add() {
    let working_dir = std::env::current_dir().expect("Failed to get current directory");
    let temp_file_path = working_dir.join(".loom").join("temp.txt");

    // Ensure the directory exists before opening Vim
    if let Some(parent_dir) = temp_file_path.parent() {
        fs::create_dir_all(parent_dir).expect("Failed to create directory");
    }

    // Open Vim for user input
    Command::new("vim")
        .arg(temp_file_path.clone())
        .status()
        .expect("Failed to open Vim");

    // Read the content of the file
    let message = fs::read_to_string(&temp_file_path).expect("Failed to read file");

    // Hash the content of the file
    let hash = calculate_hash(&message);

    // Create the new filename using the hash
    let new_filename = format!("{}.txt", hash);

    // Construct the new file path
    let new_file_path = temp_file_path.parent().unwrap().join(new_filename);

    // Rename the file using the hash as the filename
    fs::rename(&temp_file_path, &new_file_path).expect("Failed to rename file");

    println!("File saved as: {}", new_file_path.to_string_lossy());
}

fn calculate_hash(data: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}
