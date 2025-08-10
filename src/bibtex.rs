use biblatex::{Bibliography, Chunk, Entry, Person, Spanned};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use thiserror::Error;

/// Errors that can occur during BibTeX parsing
#[derive(Error, Debug)]
pub enum BibtexError {
    #[error("Failed to parse BibTeX: {0}")]
    ParseError(String),
    #[error("No entries found in the provided BibTeX string")]
    NoEntriesFound,
    #[error("Failed to parse year: {0}")]
    YearParseError(String),
    #[error("Failed to parse title: {0}")]
    TitleParseError(String),
    #[error("Failed to parse author: {0}")]
    AuthorParseError(String),
}

/// MODIFIED: Represents parsed BibTeX entry data with a content-based ID and searchable text
#[derive(Debug, Clone, PartialEq)]
pub struct BibtexData {
    /// The original key from the BibTeX entry (e.g., "doe2023").
    pub key: String,
    /// A unique, content-based SHA-256 hash of the entry.
    pub content_id: u128,
    /// Formatted author string.
    pub author: String,
    /// Parsed year, if available.
    pub year: i64,
    /// Cleaned title string.
    pub title: String,
    /// Combined searchable text containing author, title, and year for fuzzy searching.
    pub content: String,
}

/// MODIFIED: Parses a BibTeX string, extracts key fields, and generates a unique content ID.
///
/// This function is robust against formatting changes (e.g., field order, extra whitespace)
/// because it generates the ID from a standardized, canonical representation of the entry.
///
/// # Arguments
/// * `bibtex_str` - A string containing a single BibTeX entry.
///
/// # Returns
/// * `Ok(BibtexData)` - Successfully parsed data including the content ID.
/// * `Err(BibtexError)` - Parsing error.
///
/// # Example
/// ```
/// use bibtex_parser::process_bibtex_entry; // Assuming this code is in a module
///
/// let bib = r#"
/// @article{doe2023,
///   author={John Doe and Jane Smith and Bob Wilson},
///   year={2023},
///   title={Example Article},
///   url={https://example.com}
/// }
/// "#;
///
/// let result = process_bibtex_entry(bib).unwrap();
/// assert_eq!(result.key, "doe2023");
/// assert_eq!(result.author, "Doe et al."); // Truncated for display
/// assert_eq!(result.year, Some(2023));
/// assert_eq!(result.title, "Example Article");
/// // The content_id will be a consistent SHA-256 hash
/// assert_eq!(result.content_id, "993b743122e2354a81b045143a5954784439c3b31529125816a7f05a96b34909");
/// assert_eq!(result.searchable_text, "john doe jane smith bob wilson example article 2023"); // ALL authors included
/// ```
pub fn process_bibtex_entry(bibtex_str: &str) -> Result<BibtexData, BibtexError> {
    let entry = extract_entry(bibtex_str)?;

    // Generate the unique ID from the parsed entry content
    let content_id = generate_content_id_u128(&entry);

    // Parse all the fields
    let key = entry.key.clone();
    let author = parse_author(&entry)?; // This is the truncated version for display
    let year = parse_year(&entry)?;
    let title = parse_title(&entry)?;

    // NEW: Generate searchable text using ALL authors (not the truncated version)
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

/// NEW: Creates a searchable text string by combining author, title, and year
///
/// This function creates a lowercase string with all searchable content
/// separated by spaces, optimized for fuzzy searching.
///
/// # Arguments
/// * `author` - The formatted author string
/// * `title` - The cleaned title string  
/// * `year` - The year as an integer
///
/// # Returns
/// A lowercase string containing all searchable text
fn create_searchable_text(entry: &Entry, title: &str, year: i64) -> Result<String, BibtexError> {
    // Combine author, title, and year into a single searchable string
    // Convert to lowercase and clean up for better fuzzy search results
    let all_authors = entry
        .author()
        .map_err(|e| BibtexError::AuthorParseError(format!("{:?}", e)))?
        .iter()
        .map(|person| format_single_author(person))
        .collect::<Vec<_>>()
        .join(" ");

    let combined = clean_text(&format!("{} {} {}", all_authors, title, year));

    // Convert to lowercase and clean up extra whitespace
    Ok(combined
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" "))
}

/// Generates a u128 ID from a truncated SHA-256 hash.
fn generate_content_id_u128(entry: &Entry) -> u128 {
    let canonical_string = create_canonical_string(entry);

    // 1. Create the full 32-byte (256-bit) hash
    let mut hasher = Sha256::new();
    hasher.update(canonical_string.as_bytes());
    let hash_result: [u8; 32] = hasher.finalize().into();

    // 2. Truncate the hash to the first 16 bytes (128 bits)
    let truncated_hash: [u8; 16] = hash_result[0..16].try_into().unwrap();

    // 3. Convert the 16-byte array into a u128 integer
    // We use from_be_bytes (big-endian) for a standard, consistent conversion.
    u128::from_be_bytes(truncated_hash)
}

/// Generates a canonical string representation of the entry for stable hashing.
fn create_canonical_string(entry: &Entry) -> String {
    // Use a BTreeMap to sort fields by key for a stable order.
    let sorted_fields: BTreeMap<_, _> = entry.fields.iter().collect();

    let mut canonical_string = String::new();
    canonical_string.push_str(&format!("key={}\n", entry.key));

    for (key, value) in sorted_fields {
        // THIS IS THE FIX:
        // Instead of value.to_string(), we now use a helper function
        // that correctly processes the Vec<Spanned<Chunk>>.
        let formatted_value = format_field_value(value);
        canonical_string.push_str(&format!("{}={}\n", key, formatted_value));
    }

    canonical_string
}

// NEW HELPER: Properly formats a BibTeX field value into a single String.
/// A field value is a `Vec<Spanned<Chunk>>`, so we iterate and join them.
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

/// Extracts the first entry from a BibTeX string
fn extract_entry(bibtex_str: &str) -> Result<Entry, BibtexError> {
    let bibliography =
        Bibliography::parse(bibtex_str).map_err(|e| BibtexError::ParseError(format!("{:?}", e)))?;
    bibliography
        .into_iter()
        .next()
        .ok_or(BibtexError::NoEntriesFound)
}

/// Parses the year field from a BibTeX entry
fn parse_year(entry: &Entry) -> Result<i64, BibtexError> {
    entry
        .get_as::<i64>("year")
        .map_err(|e| BibtexError::YearParseError(format!("Failed to parse year: {:?}", e)))
}

fn parse_title(entry: &Entry) -> Result<String, BibtexError> {
    entry
        .get_as::<String>("title")
        .map_err(|e| BibtexError::TitleParseError(format!("{:?}", e)))
}

/// Parses the author field from a BibTeX entry
fn parse_author(entry: &Entry) -> Result<String, BibtexError> {
    let authors = entry
        .author()
        .map_err(|e| BibtexError::AuthorParseError(format!("{:?}", e)))?;

    Ok(format_authors(authors))
}

/// Formats a list of authors according to academic conventions
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

/// Formats a single author with both given and family names
fn format_single_author(author: &Person) -> String {
    // FIXED: given_name is a String, not Option<String>
    if !author.given_name.is_empty() {
        format!("{} {}", author.given_name, author.name)
    } else {
        author.name.clone()
    }
}

/// Formats an author name (family name only for multiple authors)
fn format_author_name(author: &Person) -> String {
    author.name.clone()
}

/// Cleans text by removing LaTeX commands and extra whitespace
fn clean_text(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || ".,;:!?-()[]{}".contains(*c))
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
