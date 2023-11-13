use crate::base::{self, MetaData, Paper};
use crate::settings;
use anyhow::{anyhow, Result};
use biblatex::{Bibliography, Entry, Person, RetrievalError};
use regex::Regex;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

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

fn format_slug(authors: String, year: i64, title: String) -> String {
    format!("{} {} {}", authors, year, title)
}

pub fn parse_entry(entry: Entry, meta: Option<MetaData>) -> Result<Paper, RetrievalError> {
    let (author, author_line) = parse_author(&entry)?;
    let year = parse_year(&entry)?;
    let title = parse_title(&entry)?.replace("\\n", "").replace("\\t", "");
    let slug = format_slug(author_line, year, remove_non_alphabetic(&title));
    Ok(Paper {
        id: entry.key.clone(),
        entry,
        author,
        year,
        title,
        slug,
        meta,
    })
}

pub fn parse_bibliography(bibliography: Bibliography) -> Vec<Paper> {
    let mut papers: Vec<Paper> = Vec::new();
    let all_metadata = base::read_metadata().expect("cannot read metadata");
    for entry in bibliography.into_iter() {
        let metadata = all_metadata.get(&entry.key).cloned();
        match parse_entry(entry, metadata) {
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

pub fn save_bibliography(bibliography: Bibliography, local: bool) -> Result<()> {
    let bib_path: PathBuf;
    if local {
        bib_path = Path::new("bibliography.bib").to_path_buf();
    } else {
        let base_dir = settings::base_dir()?;
        bib_path = Path::new(&base_dir).join("bibliography.bib");
    }
    let mut file = fs::File::create(bib_path)?;
    file.write_all(bibliography.to_biblatex_string().as_bytes())?;
    Ok(())
}

pub fn read_bibliography() -> Result<Bibliography> {
    let base_dir = settings::base_dir()?;
    let bib_path = Path::new(&base_dir).join("bibliography.bib");
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
