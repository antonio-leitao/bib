use crate::base::Paper;
use crate::utils::fmt::Clean;
use anyhow::{anyhow, Result};
use biblatex::{Bibliography, Entry, Person};

fn parse_year(entry: &Entry) -> Result<i64> {
    entry
        .get_as::<i64>("year")
        .map_err(|e| anyhow!("Failed to year: {:?}", e))
}

fn parse_title(entry: &Entry) -> Result<String> {
    entry
        .get_as::<String>("title")
        .map_err(|e| anyhow!("Failed to title: {:?}", e))
}
fn parse_author(entry: &Entry) -> Result<String> {
    entry
        .get_as::<Vec<Person>>("author")
        .map(|authors| format_authors(authors))
        .map_err(|e| anyhow!("Failed to author: {:?}", e))
}
fn format_authors(authors: Vec<Person>) -> String {
    let formatted = match authors.len() {
        1 => format!("{} {}", authors[0].given_name, authors[0].name),
        2 => format!("{} and {}", authors[0].name, authors[1].name),
        _ => format!("{} et al.", authors[0].name),
    };
    let formatted_authors = formatted.clean();

    formatted_authors
}

//fn remove_non_alphabetic(input: &str) -> String {
//    let re = Regex::new(r"[^a-zA-Z ]").unwrap();
//    re.replace_all(input, "").to_string()
//}
//
//fn parse_url(entry: &Entry) -> Option<String> {
//    match entry.get_as::<String>("url") {
//        Ok(val) => Some(val),
//        Err(_) => None,
//    }
//}

fn extract_entry(bibtex_str: &str) -> Result<Entry> {
    // Parse the bibliography (this will handle multiple entries, but we'll take the first one)
    let bibliography = Bibliography::parse(bibtex_str)
        .map_err(|e| anyhow!("Failed to parse BibTeX entry: {:?}", e))?;

    // If the bibliography contains at least one entry, return the first entry
    bibliography
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("No entries found in the provided BibTeX string"))
}

impl Paper {
    pub fn new(bibtex: &str) -> Result<Self> {
        let entry = extract_entry(bibtex)?;
        let title = parse_title(&entry)?.replace("\\n", "").replace("\\t", "");
        let author = parse_author(&entry)?;
        let year = parse_year(&entry)?;
        Ok(Paper {
            id: entry.key.clone(),
            author,
            year,
            title,
            stack: Vec::new(),
            bibtex: bibtex.to_owned(),
        })
    }
}
