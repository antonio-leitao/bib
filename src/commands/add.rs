use crate::base::{load_papers, save_papers, Paper};
use crate::embedding::{load_vectors, save_vectors, Point};
use crate::parser::arxiv::{self, download_arxiv_pdf, download_pdf};
use crate::stacks::Stack;
use crate::{blog, utils};
use anyhow::Result;
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::process::Command;
use tempfile::NamedTempFile;

const EDITOR: &str = "nvim"; //Add this to config
                             //
fn prompt_message() -> Result<String> {
    // Create a temporary file
    let temp_file = NamedTempFile::new()?;
    let temp_file_path = temp_file.path().to_owned();
    // Open Vim for user input (you might need to adjust the vim command)
    Command::new(EDITOR).arg(temp_file.path()).status()?;
    // Read the content of the file
    let mut message = String::new();
    let mut file = fs::File::open(&temp_file_path)?;
    file.read_to_string(&mut message)?;
    // Delete the temporary file
    temp_file.close()?;
    // Return the message
    Ok(message)
}

fn build_paper(url: Option<String>) -> Result<Paper> {
    let bibtex = match url {
        None => prompt_message()?,
        Some(url) => arxiv::arxiv2bib(&url)?,
    };
    Paper::from_bibtex(&bibtex)
}

fn is_duplicate(
    papers: &mut BTreeMap<String, Paper>,
    paper: &Paper,
    current_stack: Option<Stack>,
) -> bool {
    match papers.get_mut(&paper.id) {
        None => false,
        Some(dupe) => {
            if let Some(stack) = current_stack {
                if !dupe.stack.contains(&stack) {
                    dupe.stack.push(stack);
                }
            }
            true
        }
    }
}

pub fn add(url: String, pdf: bool, web: bool) -> Result<()> {
    let mut paper: Paper;
    let bytes: Vec<u8>;
    if pdf {
        paper = build_paper(None)?;
        bytes = utils::io::read_and_move_file(&url, &paper.id)?;
    } else if web {
        paper = build_paper(None)?;
        blog!("Downloading", "pdf from url: {}", url);
        bytes = download_pdf(&url, &paper.id)?;
    } else {
        paper = build_paper(Some(url.clone()))?;
        blog!("Downloading", "pdf from url: {}", &url);
        bytes = download_arxiv_pdf(&url, &paper.id)?;
    }

    //check stack conditions
    let config = utils::io::read_config_file()?;
    let mut papers = load_papers()?;

    if is_duplicate(&mut papers, &paper, config.current_stack()) {
        save_papers(&papers)?;
        return Ok(());
    } else {
        if let Some(stack) = config.current_stack() {
            paper.stack.push(stack)
        }
    }

    // Embed the dude
    let vector = Point::from_bytes(paper.id.clone(), bytes)?;
    let mut vectors = load_vectors()?;
    vectors.insert(paper.id.clone(), vector);
    save_vectors(&vectors)?;
    //save it i
    blog!("Saving", "{}", paper.title);
    papers.insert(paper.id.clone(), paper);
    save_papers(&papers)?;
    Ok(())
}
