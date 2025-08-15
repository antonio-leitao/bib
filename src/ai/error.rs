use thiserror::Error;

#[derive(Error, Debug)]
pub enum AiError {
    #[error("GEMINI_KEY environment variable not set")]
    ApiKeyMissing,

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("API returned error: {0}")]
    ApiResponse(String),

    #[error("Failed to parse API response: {0}")]
    ResponseParsing(#[from] serde_json::Error),

    #[error("No content in API response")]
    EmptyResponse,

    #[error("No file uploaded to API")]
    NoFileUploaded,

    #[error("Failed to generate embedding: {0}")]
    EmbeddingFailed(String),

    #[error("Failed to deserialize structured output: {0}")]
    StructuredOutputFailed(String),
}
