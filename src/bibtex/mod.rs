mod error;
pub use error::BibtexError;

use biblatex::{Bibliography, Chunk, Entry, Person, RetrievalError, Spanned};
use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub struct BibtexData {
    pub key: String,
    pub content_id: u128,
    pub author: String,
    pub year: i64,
    pub title: String,
    pub content: String,
}

pub struct BibtexParser;

// This section is unchanged.
impl BibtexParser {
    pub fn extract_doi(bibtex: &str) -> Option<String> {
        let bibtex_lower = bibtex.to_lowercase();
        if let Some(doi) = Self::extract_doi_field(&bibtex_lower) {
            return Some(doi);
        }
        Self::extract_doi_from_url(&bibtex_lower)
    }
    fn extract_doi_field(bibtex: &str) -> Option<String> {
        let re = Regex::new(r#"doi\s*=\s*[{"]([^"}]+)["}]"#).ok()?;
        re.captures(bibtex)?.get(1).map(|m| m.as_str().to_string())
    }
    fn extract_doi_from_url(bibtex: &str) -> Option<String> {
        let re = Regex::new(r#"url\s*=\s*[{"]https?://(?:dx\.)?doi\.org/([^"}]+)["}]"#).ok()?;
        re.captures(bibtex)?.get(1).map(|m| m.as_str().to_string())
    }
}

pub fn process_bibtex_entry(bibtex_str: &str) -> Result<BibtexData, BibtexError> {
    let entry = extract_entry(bibtex_str)?;
    let content_id = generate_content_id_u128(&entry);

    let key = entry.key.clone();
    let author = parse_author(&entry)?;
    let year = parse_year(&entry)?;
    let title = parse_title(&entry)?;
    let content = create_searchable_text(&entry, &title, year)?;

    Ok(BibtexData {
        key,
        content_id,
        author,
        year,
        title,
        content,
    })
}

fn create_searchable_text(entry: &Entry, title: &str, year: i64) -> Result<String, BibtexError> {
    // UPDATED: Using the new error variant directly.
    let all_authors = entry
        .author()
        .map_err(|e| BibtexError::InvalidField {
            field: "author".to_string(),
            reason: e.to_string(),
        })?
        .iter()
        .map(|person| format_single_author(person))
        .collect::<Vec<_>>()
        .join(" ");

    let combined = clean_text(&format!("{} {} {}", all_authors, title, year));
    Ok(combined
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" "))
}

fn extract_entry(bibtex_str: &str) -> Result<Entry, BibtexError> {
    // UPDATED: Using the new error variants directly.
    let bibliography =
        Bibliography::parse(bibtex_str).map_err(|e| BibtexError::ParseFailed(e.to_string()))?;
    bibliography
        .into_iter()
        .next()
        .ok_or(BibtexError::NoEntries)
}

fn parse_year(entry: &Entry) -> Result<i64, BibtexError> {
    // UPDATED: Directly mapping the error without a helper function.
    entry.get_as::<i64>("year").map_err(|e| match e {
        RetrievalError::Missing(e) => BibtexError::MissingField {
            field: e.to_string(),
        },
        _ => BibtexError::InvalidField {
            field: "year".to_string(),
            reason: e.to_string(),
        },
    })
}

fn parse_title(entry: &Entry) -> Result<String, BibtexError> {
    // UPDATED: Directly mapping the error without a helper function.
    // The match logic is repeated here as requested to avoid the helper.
    entry.get_as::<String>("title").map_err(|e| match e {
        RetrievalError::Missing(e) => BibtexError::MissingField {
            field: e.to_string(),
        },
        _ => BibtexError::InvalidField {
            field: "title".to_string(),
            reason: e.to_string(),
        },
    })
}

fn parse_author(entry: &Entry) -> Result<String, BibtexError> {
    // UPDATED: Using the new error variant directly.
    let authors = entry.author().map_err(|e| BibtexError::InvalidField {
        field: "author".to_string(),
        reason: e.to_string(),
    })?;

    Ok(format_authors(authors))
}

// --- The rest of your functions are unchanged ---

/// Generates a u128 ID from a truncated SHA-256 hash.
fn generate_content_id_u128(entry: &Entry) -> u128 {
    let canonical_string = create_canonical_string(entry);
    let mut hasher = Sha256::new();
    hasher.update(canonical_string.as_bytes());
    let hash_result: [u8; 32] = hasher.finalize().into();
    let truncated_hash: [u8; 16] = hash_result[0..16].try_into().unwrap();
    u128::from_be_bytes(truncated_hash)
}

/// Generates a canonical string representation of the entry for stable hashing.
fn create_canonical_string(entry: &Entry) -> String {
    let sorted_fields: BTreeMap<_, _> = entry.fields.iter().collect();
    let mut canonical_string = String::new();
    canonical_string.push_str(&format!("key={}\n", entry.key));
    for (key, value) in sorted_fields {
        let formatted_value = format_field_value(value);
        canonical_string.push_str(&format!("{}={}\n", key, formatted_value));
    }
    canonical_string
}

/// Properly formats a BibTeX field value into a single String.
fn format_field_value(value: &Vec<Spanned<Chunk>>) -> String {
    value
        .iter()
        .map(|spanned_chunk| match &spanned_chunk.v {
            Chunk::Normal(s) => s.clone(),
            Chunk::Verbatim(s) => s.clone(),
            Chunk::Math(s) => s.clone(),
        })
        .collect::<Vec<String>>()
        .join("")
}

/// Formats a list of authors according to academic conventions.
fn format_authors(authors: Vec<Person>) -> String {
    let formatted = match authors.len() {
        0 => String::new(),
        1 => format_single_author(&authors[0]),
        2 => format!(
            "{} and {}",
            format_author_name(&authors[0]),
            format_author_name(&authors[1])
        ),
        _ => format!("{} et al.", format_author_name(&authors[0])),
    };
    clean_text(&formatted)
}

/// Formats a single author with both given and family names.
fn format_single_author(author: &Person) -> String {
    if !author.given_name.is_empty() {
        format!("{} {}", author.given_name, author.name)
    } else {
        author.name.clone()
    }
}

/// Formats an author name (family name only for multiple authors).
fn format_author_name(author: &Person) -> String {
    author.name.clone()
}

/// Cleans text by removing LaTeX commands and extra whitespace.
fn clean_text(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || ".,;:!?-()[]{}".contains(*c))
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
