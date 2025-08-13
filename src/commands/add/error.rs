use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AddError {
    #[error("Clipboard error: {0}")]
    Clipboard(#[from] arboard::Error),

    #[error("Clipboard is empty")]
    EmptyClipboard,

    #[error("Invalid input: {0}")]
    InvalidInput(#[from] InputError),

    #[error("Download failed: {0}")]
    Download(#[from] DownloadError),

    #[error("AI processing failed: {0}")]
    Ai(#[from] crate::ai::AiError),

    #[error("BibTeX processing failed: {0}")]
    Bibtex(#[from] crate::bibtex::BibtexError),

    #[error("Storage failed: {0}")]
    Storage(#[from] crate::storage::StorageError),

    #[error("PDF handling failed: {0}")]
    Pdf(#[from] crate::pdf::PdfError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum InputError {
    #[error("Invalid arXiv URL: {0}")]
    InvalidArxivUrl(String),

    #[error("URL does not point to a PDF: {0}")]
    NotPdfUrl(String),

    #[error("Path does not point to a PDF file: {}", .0.display())]
    NotPdfPath(PathBuf),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
}

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Access forbidden (HTTP 403) - authentication may be required")]
    Forbidden,

    #[error("Resource not found (HTTP 404)")]
    NotFound,

    #[error("Authentication required (HTTP 401)")]
    Unauthorized,

    #[error("Rate limited (HTTP 429) - try again later")]
    RateLimited,

    #[error("HTTP {code}: {message}")]
    HttpError { code: u16, message: String },

    #[error("Failed to parse XML response: {0}")]
    XmlParseError(#[from] quick_xml::Error),
}
