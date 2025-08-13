use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PdfError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("PDF file not found at: {}", .0.display())]
    FileNotFound(PathBuf),

    #[error("Invalid PDF path: {}", .0.display())]
    InvalidPath(PathBuf),

    #[error("Failed to open PDF: {0}")]
    OpenFailed(String),

    #[error("Failed to save PDF: {0}")]
    SaveFailed(String),

    #[error("Failed to delete PDF: {0}")]
    DeleteFailed(String),
}
