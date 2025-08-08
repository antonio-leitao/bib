use reqwest::{header, Client};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const UPLOAD_URL: &str = "https://generativelanguage.googleapis.com/upload/v1beta/files";
const MODEL_URL: &str =
    "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent";

#[derive(Error, Debug)]
pub enum GeminiError {
    #[error("GOOGLE_API_KEY not found in environment variables")]
    ApiKeyMissing,
    #[error("File I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Network or HTTP request error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Failed to parse JSON response: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Gemini API returned an error: {0}")]
    ApiError(String),
    #[error("Could not find generated text in the API response")]
    ContentMissing,
}

// --- Structs for file upload response ---

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct FileUploadResponse {
    file: FileInfo,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct FileInfo {
    mime_type: String,
    uri: String,
}

// --- Structs for content generation ---

#[derive(Serialize)]
struct GenerateContentRequest<'a> {
    contents: Vec<Content<'a>>,
    #[serde(rename = "generationConfig", skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    response_mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_schema: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct Content<'a> {
    parts: Vec<Part<'a>>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum Part<'a> {
    Text {
        text: &'a str,
    },
    FileData {
        #[serde(rename = "fileData")]
        file_data: FileData<'a>,
    },
}

#[derive(Serialize)]
struct FileData<'a> {
    #[serde(rename = "mimeType")]
    mime_type: &'a str,
    #[serde(rename = "fileUri")]
    file_uri: &'a str,
}

#[derive(Deserialize, Debug)]
struct GenerateContentResponse {
    candidates: Option<Vec<Candidate>>,
}

#[derive(Deserialize, Debug)]
struct Candidate {
    content: Option<ApiResponseContent>,
}

#[derive(Deserialize, Debug)]
struct ApiResponseContent {
    parts: Option<Vec<ApiResponsePart>>,
}

#[derive(Deserialize, Debug)]
struct ApiResponsePart {
    text: Option<String>,
}

// --- Helper Functions ---

/// Uploads file bytes to the Gemini file service using direct file content
async fn upload_file(
    client: &Client,
    api_key: &str,
    file_bytes: Vec<u8>,
    mime_type: &str,
) -> Result<FileInfo, GeminiError> {
    // The key insight: send file content directly, not as multipart form
    let response = client
        .post(UPLOAD_URL)
        .header("X-Goog-Api-Key", api_key)
        .header(header::CONTENT_TYPE, mime_type)
        .body(file_bytes)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Could not read error body".to_string());
        return Err(GeminiError::ApiError(format!(
            "File upload failed with status {}: {}",
            status, error_text
        )));
    }

    // Debug: Print the raw response
    let response_text = response.text().await?;

    // Parse the JSON
    let upload_response: FileUploadResponse = serde_json::from_str(&response_text)?;
    Ok(upload_response.file)
}

/// Retrieves the API key from the environment.
fn get_api_key() -> Result<String, GeminiError> {
    dotenvy::dotenv().ok();
    std::env::var("GOOGLE_API_KEY").map_err(|_| GeminiError::ApiKeyMissing)
}

// --- Public API Functions ---

/// Creates a simple object schema with string properties
/// let schema = gemini::create_object_schema(&[
///    ("title", "The document title"),
///    ("author", "The document author"),
///    ("topic", "Main topic discussed")
/// ]);
pub fn create_object_schema(properties: &[(&str, &str)]) -> serde_json::Value {
    let mut props = serde_json::Map::new();
    let mut required = Vec::new();

    for (name, description) in properties {
        props.insert(
            name.to_string(),
            serde_json::json!({
                "type": "string",
                "description": description
            }),
        );
        required.push(name.to_string());
    }

    serde_json::json!({
        "type": "object",
        "properties": props,
        "required": required
    })
}

/// Creates an enum schema for classification tasks
/// let schema = gemini::create_enum_schema(
///    &["positive", "negative", "neutral"],
///    Some("Sentiment classification")
/// );
pub fn create_enum_schema(values: &[&str], description: Option<&str>) -> serde_json::Value {
    let mut schema = serde_json::json!({
        "type": "string",
        "enum": values
    });

    if let Some(desc) = description {
        schema["description"] = serde_json::Value::String(desc.to_string());
    }

    schema
}

/// Creates an array schema with object items
/// let schema = gemini::create_array_schema(&[
///    ("name", "Person's name"),
///    ("role", "Their role or job"),
///    ("importance", "How important they are to the story")
/// ]);
pub fn create_array_schema(item_properties: &[(&str, &str)]) -> serde_json::Value {
    let item_schema = create_object_schema(item_properties);

    serde_json::json!({
        "type": "array",
        "items": item_schema
    })
}

pub async fn ask_about_file(
    file_bytes: Vec<u8>,
    mime_type: &str,
    prompt: &str,
    response_schema: Option<serde_json::Value>,
) -> Result<String, GeminiError> {
    let api_key = get_api_key()?;
    let client = Client::new();

    // Step 1: Upload the file
    let file_info = upload_file(&client, &api_key, file_bytes, mime_type).await?;

    // Step 2: Create generation config if schema is provided
    let generation_config = response_schema.map(|schema| GenerationConfig {
        response_mime_type: Some("application/json".to_string()),
        response_schema: Some(schema),
    });

    // Step 3: Generate content using the uploaded file's URI
    let request_body = GenerateContentRequest {
        contents: vec![Content {
            parts: vec![
                Part::Text { text: prompt },
                Part::FileData {
                    file_data: FileData {
                        mime_type: &file_info.mime_type,
                        file_uri: &file_info.uri,
                    },
                },
            ],
        }],
        generation_config,
    };

    let response = client
        .post(MODEL_URL)
        .header("X-Goog-Api-Key", &api_key)
        .header(header::CONTENT_TYPE, "application/json")
        .json(&request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(GeminiError::ApiError(format!(
            "Content generation failed: {}",
            error_text
        )));
    }

    let gen_response: GenerateContentResponse = response.json().await?;

    gen_response
        .candidates
        .and_then(|mut c| c.pop())
        .and_then(|c| c.content)
        .and_then(|co| co.parts)
        .and_then(|mut p| p.pop())
        .and_then(|p| p.text)
        .ok_or(GeminiError::ContentMissing)
}

pub const BIBTEX_PROMPT: &str = r#"You are tasked with creating a complete and correctly formatted BibTeX entry from the content of a research article PDF. Follow these steps carefully:

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

// EXAMPLE MAIN FUNCTION
//mod gemini;
//
// #[tokio::main]
// async fn main() {
//     println!("Asking Gemini about 'sample.txt'...");
//
//     let file_path = "sample.txt";
//     let prompt = "What animal is lazy in this document? Also include what action the fox is doing.";
//
//     // Example 2: Using helper function
//     let schema = gemini::create_object_schema(&[
//         ("lazy_animal", "The animal described as lazy"),
//         ("fox_action", "What the fox is doing"),
//     ]);
//
//     // Read file and determine MIME type
//     match read_file_with_mime(file_path).await {
//         Ok((file_bytes, mime_type)) => {
//             // Example with manual structured output
//             println!("\n=== WITH MANUAL STRUCTURED OUTPUT ===");
//             match gemini::ask_about_file(file_bytes.clone(), &mime_type, prompt, Some(schema)).await
//             {
//                 Ok(response) => {
//                     println!("--- Gemini's Structured Response ---");
//                     if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&response) {
//                         println!("{}", serde_json::to_string_pretty(&parsed).unwrap());
//                     } else {
//                         println!("{}", response);
//                     }
//                 }
//                 Err(e) => {
//                     eprintln!("--- An Error Occurred ---");
//                     eprintln!("{}", e);
//                 }
//             }
//         }
//         Err(e) => {
//             eprintln!("\n--- File Reading Error ---");
//             eprintln!("{}", e);
//         }
//     }
// }
//
// async fn read_file_with_mime(
//     file_path: &str,
// ) -> Result<(Vec<u8>, String), Box<dyn std::error::Error>> {
//     // Read file bytes
//     let file_bytes = tokio::fs::read(file_path).await?;
//
//     // Determine MIME type from path
//     let mime_type = mime_guess::from_path(file_path)
//         .first_or_octet_stream()
//         .to_string();
//
//     Ok((file_bytes, mime_type))
// }
//
