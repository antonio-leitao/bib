use biblatex::{Bibliography, Entry, Person, RetrievalError};
use regex::Regex;

#[derive(Clone)]
pub struct Paper {
    pub author: String,
    pub year: i64,
    pub title: String,
    pub pdf: bool,
    pub note: bool,
    pub slug: String,
}

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
fn parse_entry(entry: Entry) -> Result<Paper, RetrievalError> {
    let (author, author_line) = parse_author(&entry)?;
    let year = parse_year(&entry)?;
    let title = parse_title(&entry)?.replace("\\n", "").replace("\\t", "");

    let slug = format_slug(author_line, year, remove_non_alphabetic(&title));
    Ok(Paper {
        author,
        year,
        title,
        slug,
        pdf: true,
        note: false,
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
