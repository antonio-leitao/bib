use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Paper not found with id: {0}")]
    PaperNotFound(u128),

    #[error("Paper already exists with key: {0}")]
    DuplicateKey(String),

    #[error("Failed to serialize data: {0}")]
    Serialization(String),

    #[error("Failed to deserialize data: {0}")]
    Deserialization(String),
}
