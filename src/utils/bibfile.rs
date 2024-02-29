use crate::base::Paper;
use crate::settings;
use anyhow::{anyhow, Result};
use biblatex::{Bibliography, Entry, Person, RetrievalError};
use regex::Regex;
use shellexpand::tilde;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

fn parse_year(entry: &Entry) -> Result<i64, RetrievalError> {
    entry.get_as::<i64>("year")
}

fn parse_title(entry: &Entry) -> Result<String, RetrievalError> {
    entry.get_as::<String>("title")
}
fn parse_author(entry: &Entry) -> Result<(String, String), RetrievalError> {
    entry
        .get_as::<Vec<Person>>("author")
        .map(|authors| format_authors(authors))
}
fn format_authors(authors: Vec<Person>) -> (String, String) {
    let formatted = match authors.len() {
        1 => format!("{} {}", authors[0].given_name, authors[0].name),
        2 => format!("{} and {}", authors[0].name, authors[1].name),
        _ => format!("{} et al.", authors[0].name),
    };
    let formatted_authors = formatted.replace("\\n", "").replace("\\t", "");

    let author_line = authors
        .iter()
        .map(|person| person.name.clone())
        .collect::<Vec<_>>()
        .join(" ");

    (formatted_authors, remove_non_alphabetic(&author_line))
}

fn remove_non_alphabetic(input: &str) -> String {
    let re = Regex::new(r"[^a-zA-Z ]").unwrap();
    re.replace_all(input, "").to_string()
}

fn parse_url(entry: &Entry) -> Option<String> {
    match entry.get_as::<String>("url") {
        Ok(val) => Some(val),
        Err(_) => None,
    }
}

pub fn parse_entry(entry: Entry) -> Result<Paper, RetrievalError> {
    let title = parse_title(&entry)?.replace("\\n", "").replace("\\t", "");
    let (author, author_line) = parse_author(&entry)?;
    let year = parse_year(&entry)?;
    let url = parse_url(&entry);
    let slug = format!("{} {} {}", author_line, year, remove_non_alphabetic(&title));
    Ok(Paper {
        id: entry.key.clone(),
        entry,
        author,
        year,
        title,
        slug,
        url,
    })
}

pub fn parse_bibliography(bibliography: Bibliography) -> Vec<Paper> {
    let mut papers: Vec<Paper> = Vec::new();
    for entry in bibliography.into_iter() {
        match parse_entry(entry) {
            Ok(paper) => papers.push(paper),
            Err(_) => continue,
        }
    }
    papers
}

pub fn read_bibtex(bib_content: &str) -> Result<Bibliography> {
    Bibliography::parse(&bib_content)
        .map_err(|err| anyhow!("Failed to parse bibliography\n{}", err))
}

pub fn save_other_bibliography(bibliography: Bibliography, from: &str) -> Result<()> {
    let dir = tilde(&format!("~/.bib/{}.bib", from)).to_string();
    let bib_path = Path::new(&dir);
    let mut file = fs::File::create(bib_path)?;
    file.write_all(bibliography.to_biblatex_string().as_bytes())?;
    Ok(())
}
pub fn save_bibliography(bibliography: Bibliography) -> Result<()> {
    let bib_path = settings::base_bib_path()?;
    let mut file = fs::File::create(bib_path)?;
    file.write_all(bibliography.to_biblatex_string().as_bytes())?;
    Ok(())
}

pub fn read_bibliography() -> Result<Bibliography> {
    let bib_path = settings::base_bib_path()?;
    let mut bib_content = String::new();
    if !bib_path.exists() {
        // If the draft file doesn't exist, create an empty one
        let mut file = fs::File::create(&bib_path)?;
        file.write_all(b"")?;
    } else {
        // If the draft file exists, open and read its content
        let mut file = fs::File::open(&bib_path)?;
        file.read_to_string(&mut bib_content)?;
    }
    read_bibtex(&bib_content)
}

pub fn read_other_bibliography(from: &str) -> Result<Bibliography> {
    let dir = tilde(&format!("~/.bib/{}.bib", from)).to_string();
    let bib_path = Path::new(&dir);
    let mut bib_content = String::new();
    if !bib_path.exists() {
        // If the draft file doesn't exist, create an empty one
        let mut file = fs::File::create(&bib_path)?;
        file.write_all(b"")?;
    } else {
        // If the draft file exists, open and read its content
        let mut file = fs::File::open(&bib_path)?;
        file.read_to_string(&mut bib_content)?;
    }
    read_bibtex(&bib_content)
}
