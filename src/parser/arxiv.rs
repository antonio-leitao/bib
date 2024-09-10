extern crate quick_xml;
use crate::utils::fmt::Clean;
use crate::utils::io;
use anyhow::{anyhow, Result};
use regex::Regex;
use reqwest::blocking::get;
use serde::Deserialize;
use std::fs::File;
use std::io::Write;

const STOP_WORD: [&str; 34] = [
    "a", "an", "and", "are", "as", "at", "be", "but", "by", "for", "if", "in", "into", "is", "it",
    "no", "not", "of", "on", "or", "such", "that", "the", "their", "then", "there", "these",
    "they", "this", "to", "was", "will", "and", "with",
];

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct Feed {
    entry: Entry,
}

#[derive(Debug, Deserialize)]
struct Entry {
    published: String,
    title: String,
    author: Vec<Author>,
    link: Vec<Link>,
    category: Vec<Category>,
}

#[derive(Debug, Deserialize)]
struct Author {
    name: String,
}

#[derive(Debug, Deserialize)]
struct Link {
    #[serde(rename = "@href")]
    href: String,
    #[serde(rename = "@title")]
    title: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Category {
    #[serde(rename = "@term")]
    term: String,
}

fn get_arxiv_id(link: &str) -> Option<&str> {
    // Check if the link starts with "https://arxiv.org/" or "http://arxiv.org/"
    if link.starts_with("https://arxiv.org/") || link.starts_with("http://arxiv.org/") {
        // Split the link by "/"
        let parts: Vec<&str> = link.split('/').collect();
        // Find the index of the "abs" or "pdf" segment
        if let Some(index) = parts.iter().position(|&x| x == "abs" || x == "pdf") {
            // Return the element after the "abs" or "pdf" segment as the arXiv ID
            return parts.get(index + 1).copied();
        }
    }
    None
}

fn get_arxiv_pdf_link(arxiv_id: &str) -> String {
    format!("https://arxiv.org/pdf/{}.pdf", arxiv_id)
}

fn first_non_stop_word(entry: &Entry) -> Option<String> {
    let title_words: Vec<&str> = entry.title.split_whitespace().collect();
    for word in title_words {
        let clean_word = remove_non_alphabetic(word);
        if !STOP_WORD.contains(&clean_word.to_lowercase().as_str()) {
            return Some(clean_word);
        }
    }
    None
}

fn remove_non_alphabetic(input: &str) -> String {
    let re = Regex::new(r"[^a-zA-Z ]").unwrap();
    re.replace_all(input, "").to_string()
}

fn create_key(entry: &Entry) -> Option<String> {
    // Extract the surname of the first author
    let surname = entry.author.get(0).map(|author| {
        author
            .name
            .split_whitespace()
            .last()
            .unwrap_or(&author.name)
            .to_string()
    });
    // Extract the year from the published date
    let year = String::from(&entry.published[..4]);
    // Extract the first word of the title that is not in the stop words list
    let title_word = first_non_stop_word(entry);
    // Combine the surname, year, and the first non-stop word into a single string
    match (surname, title_word) {
        (Some(surname), Some(word)) => Some(format!(
            "{}{}{}",
            surname.to_lowercase(),
            year,
            word.to_lowercase()
        )),
        (Some(_), None) => None,
        (None, Some(_)) => None,
        (None, None) => None,
    }
}

fn generate_biblatex(entry: &Entry, arxiv_id: &str) -> String {
    let mut biblatex = String::new();
    //key
    biblatex.push_str("@misc{");
    let key = match create_key(entry) {
        Some(key) => key,
        None => arxiv_id.to_string(),
    };
    biblatex.push_str(&key);
    biblatex.push_str(",\n");
    //authors
    biblatex.push_str("    author = {");
    for (i, author) in entry.author.iter().enumerate() {
        if i > 0 {
            biblatex.push_str(" and ");
        }
        biblatex.push_str(&author.name);
    }
    biblatex.push_str("},\n");
    //title
    biblatex.push_str("    title = {");
    biblatex.push_str(&entry.title.clean());
    biblatex.push_str("},\n");
    //arxiv id
    biblatex.push_str("    eprint = {");
    biblatex.push_str(&arxiv_id);
    biblatex.push_str("},\n");
    biblatex.push_str("    archivePrefix = {arXiv},\n");
    //year
    biblatex.push_str("    year = {");
    biblatex.push_str(&entry.published[..4]); // Extracting the year part from the published date
                                              // PDF LINK
    biblatex.push_str("},\n");
    biblatex.push_str("    url = {");
    //get pdf link, use arxiv id if none exists
    if let Some(link) = entry
        .link
        .iter()
        .find(|link| link.title == Some("pdf".to_string()))
    {
        biblatex.push_str(&link.href);
    } else {
        let pdf_link = get_arxiv_pdf_link(&arxiv_id);
        biblatex.push_str(&pdf_link);
    }

    //CATEGORY
    biblatex.push_str("},\n");
    biblatex.push_str("    primaryClass = {");
    if let Some(cat) = entry.category.first() {
        biblatex.push_str(&cat.term);
    }
    biblatex.push_str("},\n");
    biblatex.push_str("}");

    biblatex
}

pub fn download_pdf(pdf_url: &str, paper_id: &str) -> Result<Vec<u8>> {
    let response = get(pdf_url)?; // Use blocking `get`
    let filename = io::pdf_path(paper_id)?;
    // Use blocking `File::create` and `write_all`
    let mut file = File::create(&filename)?;
    let content = response.bytes()?;
    file.write_all(&content)?;

    Ok(content.to_vec())
}

pub fn download_arxiv_pdf(link: &str, paper_id: &str) -> Result<Vec<u8>> {
    let arxiv_id = get_arxiv_id(link).ok_or(anyhow!("Invalid arxiv link"))?;
    let pdf_url = get_arxiv_pdf_link(arxiv_id);
    download_pdf(&pdf_url, paper_id)
}

pub fn arxiv2bib(link: &str) -> Result<String> {
    let arxiv_id = get_arxiv_id(link).ok_or(anyhow!("Invalid arxiv link"))?;
    let url = format!(
        "http://export.arxiv.org/api/query?id_list={}&max_results=1",
        arxiv_id
    );
    let response = get(&url)?; // Use blocking `get`
    let xml = response.text()?; // Synchronous `text`
    let feed: Feed = quick_xml::de::from_str(&xml)?;
    let bibtex = generate_biblatex(&feed.entry, arxiv_id);
    Ok(bibtex)
}
