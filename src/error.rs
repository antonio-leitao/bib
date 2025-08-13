use thiserror::Error;

/// Top-level application error
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Failed to add paper: {0}")]
    Add(#[from] crate::commands::add::AddError),

    #[error("Search failed: {0}")]
    Search(#[from] crate::commands::search::SearchError),

    #[error("Semantic search failed: {0}")]
    Find(#[from] crate::commands::find::FindError),

    #[error("Paper scan failed: {0}")]
    Scan(#[from] crate::commands::scan::ScanError),

    #[error("Storage operation failed: {0}")]
    Storage(#[from] crate::storage::StorageError),

    #[error("PDF operation failed: {0}")]
    Pdf(#[from] crate::pdf::PdfError),
}
