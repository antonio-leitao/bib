use crate::base::Paper;
use crate::utils::bibfile;
use anyhow::Result;
use biblatex::Bibliography;
use std::fs;
use std::path::Path;

fn remove_from_bibliography(key: &str) -> Result<()> {
    //add to main bibliography
    let mut bibliography: Bibliography;
    bibliography = bibfile::read_bibliography()?;
    bibliography.remove(&key);
    bibfile::save_bibliography(bibliography)
}

fn remove_file(directory_path: &str, file_name: &str) -> Result<()> {
    // Construct the full path to the file
    let file_path = Path::new(directory_path).join(file_name);
    // Check if the file exists before attempting to remove it
    if file_path.exists() {
        // Remove the file
        fs::remove_file(file_path)?;
        println!("File '{}' removed successfully.", file_name);
    } else {
        println!("File '{}' does not exist in the directory.", file_name);
    }
    Ok(())
}

pub fn delete_paper(paper: Paper) -> Result<()> {
    println!("Deleting paper and notes from: {}", paper.title);
    remove_from_bibliography(&paper.id)
}
