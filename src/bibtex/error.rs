use thiserror::Error;

#[derive(Error, Debug)]
pub enum BibtexError {
    #[error("Failed to parse BibTeX: {0}")]
    ParseFailed(String),

    #[error("No entries found in BibTeX")]
    NoEntries,

    #[error("Missing required field '{field}' in BibTeX entry")]
    MissingField { field: String },

    #[error("Invalid field value for '{field}': {reason}")]
    InvalidField { field: String, reason: String },
}
