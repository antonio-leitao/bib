use crate::base::Paper;
use crate::utils::fmt::Clean;
//use crate::{stacks, utils};
use anyhow::{anyhow, Result};
use biblatex::{Bibliography, Entry, Person};
use regex::Regex;

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
    pub fn from_bibtex(bibtex: &str) -> Result<Self> {
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

// ALL GOOD untill here

//pub fn load_bibliography_and_stack() -> Result<(Bibliography, Vec<Paper>)> {
//    let stack_name = utils::io::current_stack()?;
//    let bibliography = load_bibliography()?;
//    let papers = match stack_name == "all" {
//        true => bibliography
//            .iter()
//            .filter_map(|entry| parse_entry(entry).ok())
//            .collect(),
//        false => {
//            let stack_manager = stacks::load_stack_manager()?;
//            let paper_ids = stack_manager.get(stack_name)?;
//            paper_ids
//                .iter()
//                .filter_map(|id| bibliography.get(id))
//                .filter_map(|entry| parse_entry(entry).ok())
//                .collect()
//        }
//    };
//    Ok((bibliography, papers))
//}
