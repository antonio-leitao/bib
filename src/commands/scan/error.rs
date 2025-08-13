use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScanError {
    #[error("Storage error: {0}")]
    Storage(#[from] crate::storage::StorageError),

    #[error("AI processing error: {0}")]
    Ai(#[from] crate::ai::AiError),

    #[error("PDF handling error: {0}")]
    Pdf(#[from] crate::pdf::PdfError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("No papers found matching criteria")]
    NoPapersFound,
}
