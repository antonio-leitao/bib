use crate::base::Paper;
use crate::parser;
use crate::settings;
use crate::utils::bibfile::{self, parse_entry, read_bibtex};
// use crate::utils::ui::Spinner;
use anyhow::{anyhow, Result};
use std::fs;
use std::io::Read;
use std::process::Command;
use tempfile::NamedTempFile;

enum URL {
    Empty,
    Arxiv(String),
}

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

pub fn add_paper_to_stack(paper: Paper) -> Result<()> {
    //insert into main bibliography
    let mut bibliography = bibfile::read_bibliography()?;
    //MAKE SURE TO ADD IT ON THE TOP!
    //OR REVERSE BIBLIOGRAPHY WHEN SHOWING
    bibliography.insert(paper.entry.clone());
    bibfile::save_bibliography(bibliography)
}

fn add_bibtex(content: &str) -> Result<()> {
    // let content = prompt_message()?;
    let bib = read_bibtex(&content)?;
    //get only the first entry
    if let Some(entry) = bib.into_iter().next() {
        let paper =
            parse_entry(entry).map_err(|err| anyhow!("Failed to parse bibliography\n{}", err))?;
        add_paper_to_stack(paper)
    } else {
        Err(anyhow!("Empty bibtex"))
    }
}

fn read_url(url: String) -> URL {
    if url.is_empty() {
        return URL::Empty;
    }
    return URL::Arxiv(url);
}

pub fn add(url: String) -> Result<()> {
    //currently only implements arxiv links could be expanded
    let content = match read_url(url) {
        URL::Empty => prompt_message()?,
        URL::Arxiv(url) => parser::arxiv::get_bib(&url)?,
    };
    // let bib = read_bibtex(&content)?;
    println!("{}", content);
    add_bibtex(&content)
}
