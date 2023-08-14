use std::fs;
use std::process::Command;

pub fn add() {
    let temp_file_path = "temp.txt";

    // Open Vim for user input
    Command::new("vim")
        .arg(temp_file_path)
        .status()
        .expect("Failed to open Vim");

    // Read the content of the file
    let message = fs::read_to_string(temp_file_path).expect("Failed to read file");

    // Print the user's message
    println!("User's Message:\n{}", message);

    // Clean up temporary file
    fs::remove_file(temp_file_path).expect("Failed to remove file");
}
