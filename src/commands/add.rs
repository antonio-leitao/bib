use crate::base::{self, MetaData, Paper};
use crate::semantic::query::query_arxiv_paper;
use crate::settings;
use crate::utils::bibfile::{self, parse_entry, read_bibtex};
use anyhow::{anyhow, Result};
use biblatex::{Bibliography, Entry};
use reqwest;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::Command;
use tempfile::NamedTempFile;

fn prompt_message() -> Result<String> {
    // Create a temporary file
    let temp_file = NamedTempFile::new()?;
    let temp_file_path = temp_file.path().to_owned();

    // Open Vim for user input (you might need to adjust the vim command)
    Command::new(settings::EDITOR)
        .arg(temp_file.path())
        .status()?;

    // Read the content of the file
    let mut message = String::new();
    let mut file = fs::File::open(&temp_file_path)?;
    file.read_to_string(&mut message)?;

    // Delete the temporary file
    temp_file.close()?;

    // Return the message
    Ok(message)
}

fn copy_pdf_from_path(path: &str, key: &str) -> Result<String> {
    // Create the directory if it doesn't exist
    let directory = settings::pdf_dir()?;
    let filename = format!("{}.pdf", key); // Use format! to create the filename
    let file_path = Path::new(&directory).join(&filename);
    // Check if the file already exists
    if file_path.exists() {
        return Ok(directory.to_string() + &filename); // Convert directory to String
    }
    // Copy the PDF file to the destination directory
    fs::copy(path, &file_path)?;
    //copy pdf into pdf folder
    Ok(directory.to_string() + &filename) // Convert directory to String
}

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
    // Ok(directory.to_string() + &filename) // Convert directory to String
    Ok(filename) // Convert directory to String
}

fn insert_entry_to_bibliography(entry: Entry) -> Result<()> {
    //add to main bibliography
    let mut bibliography: Bibliography;
    bibliography = bibfile::read_bibliography()?;
    bibliography.insert(entry);
    bibfile::save_bibliography(bibliography)
}

fn insert_metadata(key: String, data: MetaData) -> Result<()> {
    let mut metadata = base::read_metadata()?;
    metadata.insert(key, data);
    base::save(&metadata, "metadata.bin")
}

fn add_paper_to_stack(mut paper: Paper) -> Result<()> {
    paper.update_last_accessed();
    let key = paper.entry.key.clone();
    //insert into main bibliography
    insert_entry_to_bibliography(paper.entry.clone())?;
    //Attempt to add metadata
    if let Some(meta) = paper.meta {
        insert_metadata(key, meta)?;
    }
    Ok(())
}

fn attempt_pdf_download(paper: &mut Paper) {
    //if there is pdf download it
    if let Some(data) = paper.meta.as_mut() {
        if let Some(url) = &data.pdf {
            match download_pdf_from_url(&url, &paper.id) {
                Ok(filename) => data.pdf = Some(filename),
                Err(err) => {
                    println!("Didn't manage to download pdf.\n{}", err);
                    data.pdf = None
                }
            }
        }
    }
}

fn add_from_arxiv(arxiv: &str) -> Result<()> {
    let paper = query_arxiv_paper(arxiv)?;
    add_online_paper(paper)
}

fn add_from_url(url: &str) -> Result<()> {
    let content = prompt_message()?;
    let bib = read_bibtex(&content)?;
    //get only the first entry
    if let Some(entry) = bib.into_iter().next() {
        let meta = MetaData {
            pdf: Some(url.to_string()),
            notes: None,
            last_accessed: Some(u64::MAX),
        };
        let paper = parse_entry(entry, Some(meta))
            .map_err(|err| anyhow!("Failed to parse bibliography\n{}", err))?;
        add_online_paper(paper)
    } else {
        Err(anyhow!("Empty bibtex"))
    }
}

fn add_from_path(path: &str) -> Result<()> {
    let content = prompt_message()?;
    let bib = read_bibtex(&content)?;
    //get only the first entry
    if let Some(entry) = bib.into_iter().next() {
        let meta = match copy_pdf_from_path(path, &entry.key) {
            Ok(filename) => Some(MetaData {
                pdf: Some(filename),
                notes: None,
                last_accessed: Some(u64::MAX),
            }),
            Err(err) => {
                println!("Didn't manage to copy pdf.\n{}", err);
                None
            }
        };
        let paper = parse_entry(entry, meta)
            .map_err(|err| anyhow!("Failed to parse bibliography\n{}", err))?;
        add_paper_to_stack(paper)
    } else {
        Err(anyhow!("Empty bibtex"))
    }
}

fn add_bibtex() -> Result<()> {
    let content = prompt_message()?;
    let bib = read_bibtex(&content)?;
    for entry in bib.into_iter() {
        let mut paper = parse_entry(entry, None)
            .map_err(|err| anyhow!("Failed to parse bibliography\n{}", err))?;
        add_paper_to_stack(paper)?;
    }
    Ok(())
}

pub fn add_online_paper(mut paper: Paper) -> Result<()> {
    attempt_pdf_download(&mut paper);
    add_paper_to_stack(paper)
}

pub fn add(arxiv: Option<String>, url: Option<String>, path: Option<String>) -> Result<()> {
    match (arxiv, url, path) {
        (Some(arxiv), None, None) => add_from_arxiv(&arxiv),
        (None, Some(url), None) => add_from_url(&url),
        (None, None, Some(path)) => add_from_path(&path),
        (None, None, None) => add_bibtex(),
        _ => Err(anyhow!("Wrong usae of command add")),
    }
}
