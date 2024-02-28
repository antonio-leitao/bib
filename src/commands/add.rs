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

fn add_from_url(url: &str) -> Result<()> {
    let content = prompt_message()?;
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

fn add_bibtex() -> Result<()> {
    let content = prompt_message()?;
    let bib = read_bibtex(&content)?;
    for entry in bib.into_iter() {
        let paper =
            parse_entry(entry).map_err(|err| anyhow!("Failed to parse bibliography\n{}", err))?;
        add_paper_to_stack(paper)?;
    }
    Ok(())
}

pub fn add(url: Option<String>) -> Result<()> {
    //currently only implements arxiv links could be expanded
    let content = match url {
        Some(link) => match parser::arxiv::get_bib(&link) {
            Ok(bib) => bib,
            Err(err) => {
                println!("{}", err);
                prompt_message()?
            }
        },
        None => prompt_message()?,
    };
    // let bib = read_bibtex(&content)?;
    println!("{}", content);
    Ok(())
}
