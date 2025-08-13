use thiserror::Error;

#[derive(Error, Debug)]
pub enum FindError {
    #[error("Storage error: {0}")]
    Storage(#[from] crate::storage::StorageError),

    #[error("UI error: {0}")]
    Ui(#[from] std::io::Error),

    #[error("PDF handling error: {0}")]
    Pdf(#[from] crate::pdf::PdfError),

    #[error("AI processing error: {0}")]
    Ai(#[from] crate::ai::AiError),

    #[error("No results found matching the query")]
    NoResults,
}
