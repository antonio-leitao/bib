use crate::base::{self, MetaData, Paper};
use crate::settings;
use crate::utils::bibfile;
use anyhow::{anyhow, Result};
use biblatex::{Bibliography, Entry};
use reqwest;
use std::fs;
use std::path::Path;

fn download_pdf_from_url(url: &str, key: &str) -> Result<String> {
    // Create the directory if it doesn't exist
    let directory = settings::pdf_dir()?;
    let filename = format!("{}.pdf", key); // Use format! to create the filename
    let file_path = Path::new(&directory).join(&filename);
    // Check if the file already exists
    if file_path.exists() {
        return Ok(directory.to_string() + &filename); // Convert directory to String
    }
    // Send an HTTP GET request to download the PDF
    let mut response = reqwest::blocking::get(url)?;
    // Check if the request was successful
    if !response.status().is_success() {
        return Err(anyhow!("Failed to download PDF: {}", response.status()));
    }
    // Create and write the PDF file
    let mut pdf_file = fs::File::create(&file_path)?;
    response.copy_to(&mut pdf_file)?;
    Ok(directory.to_string() + &filename) // Convert directory to String
}

fn insert_entry_to_bibliography(entry: Entry) -> Result<()> {
    //add to main bibliography
    let mut bibliography: Bibliography;
    bibliography = bibfile::read_bibliography()?;
    bibliography.insert(entry);
    bibfile::save_bibliography(bibliography, false)
}

fn insert_entry_to_local_bibliography(entry: Entry) -> Result<()> {
    //add to main bibliography
    let mut bibliography = match bibfile::read_local_bibliography() {
        Ok(local_bib) => local_bib,
        Err(_) => Bibliography::new(),
    };
    bibliography.insert(entry);
    bibfile::save_bibliography(bibliography, true)
}

fn fetch_pdf(key: &str, meta: MetaData) -> MetaData {
    if let Some(pdf) = &meta.pdf {
        match download_pdf_from_url(&pdf, key) {
            Ok(path) => MetaData {
                semantic_id: meta.semantic_id,
                pdf: Some(path),
                notes: meta.notes,
            },
            Err(_) => meta,
        }
    } else {
        meta
    }
}

fn insert_metadata(key: String, data: MetaData) -> Result<()> {
    let mut metadata = base::read_metadata()?;
    metadata.insert(key, data);
    base::save(&metadata, "metadata.bin")
}

pub fn add_paper_to_library(paper: Paper, local: bool) -> Result<()> {
    let key = paper.entry.key.clone();
    //insert into main bibliography
    insert_entry_to_bibliography(paper.entry.clone())?;
    //insert in local bibliography
    if local {
        insert_entry_to_local_bibliography(paper.entry)?;
    }
    //Attempt to add metadata
    if let Some(meta) = paper.meta {
        let data = fetch_pdf(&key, meta);
        insert_metadata(key, data)?;
    }
    Ok(())
}
