use crate::base::{load_papers, save_papers, Paper};
use crate::embedding::{load_vectors, save_vectors, Point};
use crate::parser::arxiv;
use crate::stacks::Stack;
use crate::{blog, utils};
use anyhow::{anyhow, bail, Result};
use indexmap::IndexMap;
use std::path::PathBuf;
use url::Url; // For URL parsing

fn is_duplicate(
    papers: &mut IndexMap<String, Paper>,
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

// An enum to represent the different outcomes
#[derive(Debug, PartialEq)]
pub enum InputKind {
    ArxivUrl(String), // It's an arXiv URL
    PdfUrl(String),   // It's a URL pointing to a PDF (not arXiv)
    OtherUrl(String), // It's some other URL
    PdfPath(PathBuf), // It's a local path to an existing PDF file
    NonPdfPath(PathBuf), // It's a local path, but not to an existing PDF file (could be dir, other file, or non-existent)
                         // NotAPathOrUrl(String), // If the string is neither a parseable URL nor a path-like string.
                         // With the current logic, everything falls into URL or Path.
}

pub fn classify_input(input_str: &str) -> InputKind {
    // 1. Check if it's a URL
    // A common heuristic: URLs contain "://"
    // We also try to parse it properly with the `url` crate.
    if input_str.contains("://") {
        match Url::parse(input_str) {
            Ok(parsed_url) => {
                // Successfully parsed as a URL
                // 2. If it's a URL, check if it's an arXiv link
                if let Some(domain) = parsed_url.domain() {
                    if domain.eq_ignore_ascii_case("arxiv.org") || domain.ends_with(".arxiv.org") {
                        return InputKind::ArxivUrl(input_str.to_string());
                    }
                }

                // 3. Else (not arXiv), check if it points to a PDF (by looking at the path component)
                // This is a simple check, a server could still serve non-PDF content or hide the extension.
                if parsed_url.path().to_lowercase().ends_with(".pdf") {
                    InputKind::PdfUrl(input_str.to_string())
                } else {
                    InputKind::OtherUrl(input_str.to_string())
                }
            }
            Err(_) => {
                // It contained "://" but couldn't be parsed by the `url` crate (e.g., "foo://bar").
                // We'll still classify it as 'OtherUrl' because it's not a typical filesystem path.
                InputKind::OtherUrl(input_str.to_string())
            }
        }
    } else {
        // 4. If it's a path, check if it points to a PDF
        let path = PathBuf::from(input_str);

        // Check if it exists, is a file, and has a .pdf extension
        if path.is_file() {
            // is_file() also implies exists()
            if path
                .extension()
                .map_or(false, |ext| ext.to_ascii_lowercase() == "pdf")
            {
                InputKind::PdfPath(path)
            } else {
                InputKind::NonPdfPath(path) // It's a file, but not a PDF
            }
        } else {
            // It's not a file (could be a directory, a non-existent path, etc.)
            // The request is "check if it points to a pdf". If it's not a file, it doesn't.
            InputKind::NonPdfPath(path)
        }
    }
}

fn download_url_as_bytes(url: &str) -> Result<Vec<u8>> {
    blog!("Downloading", "from URL: {}", url);
    let response = reqwest::blocking::get(url)
        .map_err(|e| anyhow!("Failed to download from URL '{}': {}", url, e))?;

    if !response.status().is_success() {
        bail!(
            "Failed to download from URL '{}': HTTP Status {}",
            url,
            response.status()
        );
    }
    let bytes = response
        .bytes()
        .map_err(|e| anyhow!("Failed to read bytes from response of URL '{}': {}", url, e))?;
    Ok(bytes.to_vec())
}

pub fn add(input: String) -> Result<()> {
    let (bibtex, embedding, bytes) = match classify_input(&input) {
        InputKind::ArxivUrl(url) => {
            let bibtex = arxiv::arxiv2bib(&url)?;
            println!("{}", bibtex);
            let arxiv_id = arxiv::get_arxiv_id(&url).ok_or(anyhow!("Invalid arxiv link"))?;
            let arxiv_url = arxiv::get_arxiv_pdf_link(arxiv_id);
            let bytes = download_url_as_bytes(&arxiv_url)?;
            let embedding = utils::ai::pdf_embedding_sync(&bytes)?;
            (bibtex, embedding, bytes)
        }
        InputKind::PdfUrl(url) => {
            let bytes = download_url_as_bytes(&url)?;
            let (bibtex, embedding) = utils::ai::pdf_embedding_and_bibtex_sync(&bytes)?;
            (bibtex, embedding, bytes)
        }
        InputKind::PdfPath(path) => {
            let bytes = utils::io::read_file_as_bytes(path)?;
            let (bibtex, embedding) = utils::ai::pdf_embedding_and_bibtex_sync(&bytes)?;
            (bibtex, embedding, bytes)
        }
        InputKind::OtherUrl(url) => {
            bail!(
                "The provided input '{}' is a URL, but not a recognized arXiv link or direct PDF link. \
                Please provide a direct link to a PDF file or an arXiv page.",
                url
            );
        }
        InputKind::NonPdfPath(path) => {
            bail!(
                "The provided input '{}' is a local path, but it does not point to a valid PDF file, or it does not exist.",
                path.display()
            );
        }
    };
    let mut paper = Paper::new(&bibtex)?;
    utils::io::save_pdf_bytes(&paper.id, &bytes)?;

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
    let vector = Point::new(paper.id.clone(), embedding);
    let mut vectors = load_vectors()?;
    vectors.insert(paper.id.clone(), vector);
    save_vectors(&vectors)?;
    //save it i
    blog!("Saving", "{}", paper.title);
    papers.shift_insert(0, paper.id.clone(), paper);
    save_papers(&papers)?;
    Ok(())
}
