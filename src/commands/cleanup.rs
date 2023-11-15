use crate::base::{self, MetaData, Paper, Pdf};
use crate::settings;
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

fn remove_from_metadata(key: &str) -> Result<Option<MetaData>> {
    let mut all_metadata = base::read_metadata()?;
    let metadata = all_metadata.remove(key);
    base::save(&all_metadata, "metadata.bin")?;
    Ok(metadata)
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

fn remove_notes(notes: Option<String>) -> Result<()> {
    match notes {
        Some(name) => {
            let notes_dir = settings::notes_dir()?;
            remove_file(&notes_dir, &name)
        }
        None => Ok(()),
    }
}
fn remove_pdf(pdf: Option<Pdf>) -> Result<()> {
    match pdf {
        Some(Pdf::Path(filename)) => {
            let pdf_dir = settings::pdf_dir()?;
            remove_file(&pdf_dir, &filename)
        }
        Some(Pdf::Url(_)) => Ok(()),
        None => Ok(()),
    }
}

pub fn delete_paper(paper: Paper) -> Result<()> {
    println!("Deleting paper and notes from: {}", paper.title);
    remove_from_bibliography(&paper.id)?;
    let metadata = remove_from_metadata(&paper.id)?;
    match metadata {
        Some(data) => {
            remove_pdf(data.pdf)?;
            remove_notes(data.notes)?;
            Ok(())
        }
        None => Ok(()),
    }
}
