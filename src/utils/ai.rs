use crate::blog;
use crate::utils::fmt::Spinner;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use thiserror::Error;
use tokio::runtime::Runtime;

// --- Configuration ---
const GEMINI_API_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta"; // Base for most
const GEMINI_UPLOAD_API_BASE_URL: &str =
    "https://generativelanguage.googleapis.com/upload/v1beta/files";

const MULTIMODAL_MODEL: &str = "gemini-2.0-flash"; // For PDF interaction (BibTeX, text extraction)
const TEXT_EMBEDDING_MODEL: &str = "text-embedding-004"; // For embedding extracted text

// --- Error Handling ---
#[derive(Error, Debug)]
pub enum GeminiError {
    #[error("API Key not found. Please set the GOOGLE_API_KEY environment variable.")]
    ApiKeyMissing,
    #[error("Request error: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("API error (status: {status}, type: {error_type}): {message}")]
    ApiError {
        status: u16,
        message: String,
        error_type: String,
    },
    #[error("JSON processing error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("No content found in Gemini response")]
    NoContentFound,
    #[error("File operation failed: {0}")]
    FileApiError(String),
    #[error("Task failed: {0}")]
    TaskFailed(String),
    #[error("Unexpected API response structure: {0}")]
    UnexpectedResponse(String),
}

// --- Structs for Gemini API ---

// -- File API --
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UploadedFileInfo {
    pub name: String, // e.g., "files/file-id"
    pub uri: String,
    pub mime_type: String,
    // displayName, createTime, updateTime, expirationTime, sizeBytes etc.
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct FileUploadApiResponse {
    file: UploadedFileInfo,
}

// -- Content Generation (for BibTeX and Text Extraction) --
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileDataPart {
    mime_type: String,
    file_uri: String,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum RequestPart {
    Text {
        text: String,
    },
    FileData {
        #[serde(rename = "fileData")]
        file_data: FileDataPart,
    },
}

#[derive(Serialize)]
struct GenerateContentRequest {
    contents: Vec<ContentPart>,
    // generation_config: Option<GenerationConfig>, // For more control
}

#[derive(Serialize, Deserialize)]
struct ContentPart {
    parts: Vec<RequestPart>,
}

#[derive(Deserialize, Debug)]
struct GenerateContentResponse {
    candidates: Option<Vec<Candidate>>,
    error: Option<ApiErrorDetail>, // Top-level error
}

#[derive(Deserialize, Debug)]
struct Candidate {
    content: Option<ResponseContentPart>,
    // finishReason, safetyRatings etc.
}

#[derive(Deserialize, Debug)]
struct ResponseContentPart {
    parts: Option<Vec<ResponseTextPart>>,
    #[allow(dead_code)]
    role: Option<String>,
}

#[derive(Deserialize, Debug)]
struct ResponseTextPart {
    text: Option<String>,
}

#[derive(Deserialize, Debug)]
struct ApiErrorDetail {
    code: i32,
    message: String,
    status: String, // e.g., "INVALID_ARGUMENT"
}

// -- Embedding API --
#[derive(Serialize)]
struct EmbedContentRequest<'a> {
    model: String, // e.g., "models/text-embedding-004"
    content: ContentPartForEmbedding<'a>,
}

#[derive(Serialize)]
struct ContentPartForEmbedding<'a> {
    parts: Vec<TextPartForEmbedding<'a>>,
}

#[derive(Serialize)]
struct TextPartForEmbedding<'a> {
    text: &'a str,
}

#[derive(Deserialize, Debug)]
struct EmbedContentResponse {
    embedding: Option<EmbeddingData>,
    error: Option<ApiErrorDetail>, // Top-level error for embedding
}

#[derive(Deserialize, Debug)]
struct EmbeddingData {
    values: Vec<f32>,
}

// --- Internal Helper Functions ---

fn get_api_key() -> Result<String, GeminiError> {
    env::var("GOOGLE_API_KEY").map_err(|_| GeminiError::ApiKeyMissing)
}

async fn handle_api_error(response: reqwest::Response) -> GeminiError {
    let status = response.status().as_u16();
    let error_text = response
        .text()
        .await
        .unwrap_or_else(|_| "Failed to read error body".to_string());
    eprintln!("Raw API Error: {}", error_text); // Log raw error for debugging

    match serde_json::from_str::<GenerateContentResponse>(&error_text) // Try parsing standard error structure
        .ok()
        .and_then(|resp| resp.error)
    {
        Some(api_err) => GeminiError::ApiError {
            status: api_err.code as u16,
            message: api_err.message,
            error_type: api_err.status,
        },
        None => {
            // Try parsing a simpler error structure if the above fails
            match serde_json::from_str::<Value>(&error_text).ok() {
                Some(json_val) => {
                    let message = json_val
                        .get("error")
                        .and_then(|e| e.get("message"))
                        .and_then(|m| m.as_str())
                        .unwrap_or(&error_text)
                        .to_string();
                    let error_type = json_val
                        .get("error")
                        .and_then(|e| e.get("status"))
                        .and_then(|s| s.as_str())
                        .unwrap_or("UNKNOWN_ERROR_TYPE")
                        .to_string();
                    GeminiError::ApiError {
                        status,
                        message,
                        error_type,
                    }
                }
                None => GeminiError::ApiError {
                    // Fallback if all parsing fails
                    status,
                    message: error_text,
                    error_type: "UNKNOWN_PARSE_FAILURE".to_string(),
                },
            }
        }
    }
}

/// Uploads PDF bytes to Gemini File API.
/// This is a synchronous function because reqwest::blocking::Client is used for multipart.
fn upload_pdf_bytes_internal(pdf_bytes: &[u8]) -> Result<UploadedFileInfo, GeminiError> {
    let api_key = get_api_key()?;
    let url = format!("{}?key={}", GEMINI_UPLOAD_API_BASE_URL, api_key);

    // For file uploads, reqwest::blocking::Client is often simpler
    let client = reqwest::blocking::Client::new();
    let response = client
        .post(url)
        .header(reqwest::header::CONTENT_TYPE, "application/pdf")
        .body(pdf_bytes.to_vec())
        .send()?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let error_text = response
            .text()
            .unwrap_or_else(|_| "Failed to read upload error body".to_string());
        eprintln!("Upload error response: {}", error_text);
        // Simplified error parsing for upload
        return Err(GeminiError::FileApiError(format!(
            "Upload failed with status {}: {}",
            status, error_text
        )));
    }

    let response_body: FileUploadApiResponse = response.json()?;
    Ok(response_body.file)
}

// New structured response types
#[derive(Debug, Serialize, Deserialize)]
struct BibTexResponse {
    bibtex_entry: String,
    notes: Option<String>, // Any additional notes about the extraction
}

// Return type for the function
#[derive(Debug)]
pub struct BibTexResult {
    pub bibtex: String,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ResponseSchema {
    #[serde(rename = "type")]
    schema_type: String,
    properties: SchemaProperties,
    required: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SchemaProperties {
    bibtex_entry: PropertyDefinition,
    notes: PropertyDefinition,
}

#[derive(Debug, Serialize, Deserialize)]
struct PropertyDefinition {
    #[serde(rename = "type")]
    prop_type: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    enum_values: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GenerationConfig {
    response_mime_type: String,
    response_schema: ResponseSchema,
}

// Updated request structure to include generation config
#[derive(Serialize, Deserialize)]
struct StructuredGenerateContentRequest {
    contents: Vec<ContentPart>,
    generation_config: GenerationConfig,
}

fn create_response_schema() -> ResponseSchema {
    ResponseSchema {
        schema_type: "object".to_string(),
        properties: SchemaProperties {
            bibtex_entry: PropertyDefinition {
                prop_type: "string".to_string(),
                description:
                    "Complete BibTeX entry formatted exactly as it should appear in a .bib file"
                        .to_string(),
                enum_values: None,
            },
            notes: PropertyDefinition {
                prop_type: "string".to_string(),
                description:
                    "Any additional notes about missing information or extraction challenges"
                        .to_string(),
                enum_values: None,
            },
        },
        required: vec!["bibtex_entry".to_string()],
    }
}

/// Generates BibTeX from an uploaded PDF file handle using structured output.
async fn get_bibtex_from_handle_async(
    client: &Client,
    api_key: &str,
    file_info: &UploadedFileInfo,
) -> Result<BibTexResult, GeminiError> {
    let url = format!(
        "{}/models/{}:generateContent?key={}",
        GEMINI_API_BASE_URL, MULTIMODAL_MODEL, api_key
    );

    let request_body = StructuredGenerateContentRequest {
        contents: vec![ContentPart {
            parts: vec![
                RequestPart::FileData {
                    file_data: FileDataPart {
                        mime_type: file_info.mime_type.clone(),
                        file_uri: file_info.uri.clone(),
                    },
                },
                RequestPart::Text {
                    text: BIBTEX_PROMPT.to_string(),
                },
            ],
        }],
        generation_config: GenerationConfig {
            response_mime_type: "application/json".to_string(),
            response_schema: create_response_schema(),
        },
    };

    let response = client.post(&url).json(&request_body).send().await?;

    if !response.status().is_success() {
        return Err(handle_api_error(response).await);
    }

    let response_body: GenerateContentResponse = response.json().await?;

    if let Some(api_err) = response_body.error {
        return Err(GeminiError::ApiError {
            status: api_err.code as u16,
            message: api_err.message,
            error_type: api_err.status,
        });
    }

    let text = response_body
        .candidates
        .and_then(|mut c| c.pop())
        .and_then(|cand| cand.content)
        .and_then(|cont| cont.parts)
        .and_then(|mut p| p.pop())
        .and_then(|part| part.text)
        .ok_or(GeminiError::NoContentFound)?;

    // Parse the structured JSON response
    match serde_json::from_str::<BibTexResponse>(&text) {
        Ok(structured_response) => {
            // Return the structured result
            Ok(BibTexResult {
                bibtex: structured_response.bibtex_entry.trim().to_string(),
                notes: structured_response.notes,
            })
        }
        Err(json_err) => {
            println!("Failed to parse structured response: {}", json_err);
            println!("Raw response: {}", text);
            Err(GeminiError::NoContentFound)
        }
    }
}

const TEXT_EXTRACTION_PROMPT: &str =
    "Extract all textual content from this document. Provide only the raw text, without any additional commentary, preamble, or explanation.";

/// Extracts text from an uploaded PDF file handle.
async fn get_text_from_handle_async(
    client: &Client,
    api_key: &str,
    file_info: &UploadedFileInfo,
) -> Result<String, GeminiError> {
    let url = format!(
        "{}/models/{}:generateContent?key={}",
        GEMINI_API_BASE_URL, MULTIMODAL_MODEL, api_key
    );
    let request_body = GenerateContentRequest {
        contents: vec![ContentPart {
            parts: vec![
                RequestPart::FileData {
                    file_data: FileDataPart {
                        mime_type: file_info.mime_type.clone(),
                        file_uri: file_info.uri.clone(),
                    },
                },
                RequestPart::Text {
                    text: TEXT_EXTRACTION_PROMPT.to_string(),
                },
            ],
        }],
    };

    let response = client.post(&url).json(&request_body).send().await?;
    if !response.status().is_success() {
        return Err(handle_api_error(response).await);
    }

    let response_body: GenerateContentResponse = response.json().await?;
    if let Some(api_err) = response_body.error {
        return Err(GeminiError::ApiError {
            status: api_err.code as u16,
            message: api_err.message,
            error_type: api_err.status,
        });
    }
    response_body
        .candidates
        .and_then(|mut c| c.pop())
        .and_then(|cand| cand.content)
        .and_then(|cont| cont.parts)
        .and_then(|mut p| p.pop())
        .and_then(|part| part.text)
        .ok_or(GeminiError::NoContentFound)
}

/// Generates embedding from extracted text.
async fn get_embedding_from_text_async(
    client: &Client,
    api_key: &str,
    text: &str,
) -> Result<Vec<f32>, GeminiError> {
    let model_name_for_request = format!("models/{}", TEXT_EMBEDDING_MODEL);
    let url = format!(
        "{}/{}:embedContent?key={}", // Note: uses the specific model in path
        GEMINI_API_BASE_URL, model_name_for_request, api_key
    );

    let request_body = EmbedContentRequest {
        model: model_name_for_request, // Model also specified in body for some Gemini embed endpoints
        content: ContentPartForEmbedding {
            parts: vec![TextPartForEmbedding { text }],
        },
    };

    let response = client.post(&url).json(&request_body).send().await?;
    if !response.status().is_success() {
        return Err(handle_api_error(response).await);
    }
    let response_body: EmbedContentResponse = response.json().await?;
    if let Some(api_err) = response_body.error {
        return Err(GeminiError::ApiError {
            status: api_err.code as u16,
            message: api_err.message,
            error_type: api_err.status,
        });
    }

    response_body
        .embedding
        .map(|e_data| e_data.values)
        .ok_or(GeminiError::NoContentFound) // Or NoEmbeddingFound
}

/// Deletes an uploaded file from Gemini File API using its name.
async fn delete_file_by_name_async(
    client: &Client,
    api_key: &str,
    file_name: &str, // e.g., "files/file-id"
) -> Result<(), GeminiError> {
    let url = format!("{}/{}?key={}", GEMINI_API_BASE_URL, file_name, api_key); // Corrected: files endpoint is under API_BASE_URL
    let response = client.delete(&url).send().await?;

    if !response.status().is_success() {
        // It's possible the response body for delete errors is empty or different
        let status = response.status().as_u16();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| format!("Failed to delete file, status {}", status));
        return Err(GeminiError::FileApiError(format!(
            "Delete failed for {}: Status {}, Body: {}",
            file_name, status, error_text
        )));
    }
    Ok(())
}

// --- Public API Functions ---

/// Gets only the embedding for the given PDF bytes.
/// Uploads the PDF, extracts text, gets embedding, then deletes the uploaded file.
async fn get_pdf_embedding_async(pdf_bytes: &[u8]) -> Result<Vec<f32>, GeminiError> {
    let mut spinner = Spinner::new("Processing", "PDF for embedding");
    spinner.start();
    let client = Client::new();
    let api_key = get_api_key()?;

    // 1. Upload PDF
    //    Run blocking upload in a separate thread to not block the async runtime.
    let pdf_bytes_owned = pdf_bytes.to_vec();
    let file_info =
        tokio::task::spawn_blocking(move || upload_pdf_bytes_internal(&pdf_bytes_owned))
            .await
            .map_err(|e| GeminiError::TaskFailed(format!("Upload task panicked: {}", e)))??; // Propagate JoinError then GeminiError

    // Ensure file is deleted even if subsequent steps fail
    let result = async {
        // 2. Extract text
        let text = get_text_from_handle_async(&client, &api_key, &file_info).await?;
        // 3. Get embedding
        get_embedding_from_text_async(&client, &api_key, &text).await
    }
    .await;

    // 4. Delete file
    delete_file_by_name_async(&client, &api_key, &file_info.name).await?;
    spinner.finish(None);
    result // Return the result of text extraction + embedding
}

/// Gets both BibTeX and embedding for the given PDF bytes.
/// Uploads PDF, processes concurrently, then deletes the uploaded file.
async fn get_pdf_bibtex_and_embedding(pdf_bytes: &[u8]) -> Result<(String, Vec<f32>), GeminiError> {
    let mut spinner = Spinner::new("Processing", "PDF for BibTeX and embedding");
    spinner.start();
    let client = Client::new();
    let api_key = get_api_key()?;

    // 1. Upload PDF (blocking, so spawn_blocking)
    let pdf_bytes_owned = pdf_bytes.to_vec();
    let file_info =
        tokio::task::spawn_blocking(move || upload_pdf_bytes_internal(&pdf_bytes_owned))
            .await
            .map_err(|e| GeminiError::TaskFailed(format!("Upload task panicked: {}", e)))??;

    // Ensure file is deleted even if subsequent steps fail
    let file_info_clone_for_bibtex = file_info.clone();
    let file_info_clone_for_text = file_info.clone();

    let client_ref = &client; // Create references to satisfy lifetime requirements for async blocks
    let api_key_ref = &api_key;

    let bibtex_task = async move {
        get_bibtex_from_handle_async(client_ref, api_key_ref, &file_info_clone_for_bibtex).await
    };

    let embedding_task = async move {
        let text =
            get_text_from_handle_async(client_ref, api_key_ref, &file_info_clone_for_text).await?;
        get_embedding_from_text_async(client_ref, api_key_ref, &text).await
    };

    // 2. Concurrently fetch BibTeX and embedding
    let processing_result = tokio::try_join!(bibtex_task, embedding_task);

    // 3. Delete file
    // This delete runs regardless of whether try_join succeeded or failed, as long as upload was ok.
    delete_file_by_name_async(&client, &api_key, &file_info.name).await?;
    spinner.finish(None);
    processing_result.map(|(bibtex_result, vec)| {
        if let Some(notes) = &bibtex_result.notes {
            blog!("Note", "{}", notes);
        }
        (bibtex_result.bibtex, vec)
    })
}

/// Synchronous wrapper for get_pdf_embedding_and_bibtex
pub fn pdf_embedding_and_bibtex_sync(pdf_bytes: &[u8]) -> Result<(String, Vec<f32>), GeminiError> {
    // Create a new runtime for this call
    let rt = Runtime::new().map_err(|e| GeminiError::UnexpectedResponse(e.to_string()))?;

    // Execute the async function and return the result
    rt.block_on(get_pdf_bibtex_and_embedding(pdf_bytes))
}

/// Synchronous wrapper for get_pdf_embeding
pub fn pdf_embedding_sync(pdf_bytes: &[u8]) -> Result<Vec<f32>, GeminiError> {
    // Create a new runtime for this call
    let rt = Runtime::new().map_err(|e| GeminiError::UnexpectedResponse(e.to_string()))?;

    // Execute the async function and return the result
    rt.block_on(get_pdf_embedding_async(pdf_bytes))
}

pub fn query_embedding_sync(query: &str) -> Result<Vec<f32>, GeminiError> {
    let client = Client::new();
    let api_key = get_api_key()?;
    // Create a new runtime for this call
    let rt = Runtime::new().map_err(|e| GeminiError::UnexpectedResponse(e.to_string()))?;
    // Execute the async function and return the result
    rt.block_on(get_embedding_from_text_async(&client, &api_key, &query))
}

// Prompt template for BibTeX generation
const BIBTEX_PROMPT: &str = r#"You are tasked with creating a complete and correctly formatted BibTeX entry from the content of a research article PDF. Follow these steps carefully:

1. Carefully analyze the provided PDF to extract the following bibliographic information:
   - Author(s)
   - Title
   - Journal or Conference name (if applicable)
   - Year of publication
   - Volume and issue (if applicable)
   - Page numbers (if applicable)
   - DOI (if available)
   - Publisher
   - Any other relevant information specific to the type of publication

2. Based on the type of publication (e.g., article, inproceedings, book), create a BibTeX entry using the appropriate entry type. Follow these guidelines:
   - Use a citation key that combines the first author's last name, the year, and a word from the title (e.g., smith2023quantum)
   - Include all relevant fields for the entry type
   - Ensure proper formatting of author names (Last, First and Last, First)
   - Enclose the content of each field in curly braces {{}}
   - Use double curly braces for titles to preserve capitalization
   - Separate multiple authors with " and "
   - Use standard abbreviations for months if needed (jan, feb, mar, etc.)

3. Provide your final BibTeX entry in the bibtex_entry field. Ensure that the entry is complete, correctly formatted, and ready for use in a LaTeX document.

4. If you are unable to extract certain information from the PDF content, use your best judgment to create the most complete BibTeX entry possible with the available information. If critical information is missing, indicate this in a comment within the BibTeX entry.
"#;
