mod error;
pub use error::AiError;

use reqwest::{header, Client};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const UPLOAD_URL: &str = "https://generativelanguage.googleapis.com/upload/v1beta/files";
const MODEL_URL: &str =
    "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent";
const EMBEDDING_URL: &str =
    "https://generativelanguage.googleapis.com/v1beta/models/gemini-embedding-001:embedContent";

// API request/response structs...
#[derive(Serialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EmbeddingTaskType {
    RetrievalQuery,
    RetrievalDocument,
}

#[derive(Serialize)]
struct EmbedContentRequest<'a> {
    content: EmbedContent<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_type: Option<&'a EmbeddingTaskType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_dimensionality: Option<u32>,
}

#[derive(Serialize)]
struct EmbedContent<'a> {
    parts: Vec<EmbedPart<'a>>,
}

#[derive(Serialize)]
struct EmbedPart<'a> {
    text: &'a str,
}

#[derive(Deserialize, Debug)]
struct EmbedContentResponse {
    embedding: Option<Embedding>,
}

#[derive(Deserialize, Debug)]
struct Embedding {
    values: Vec<f32>,
}
// --- Structs for file upload response ---

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct FileUploadResponse {
    file: FileInfo,
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    mime_type: String,
    uri: String,
}

// --- Structs for content generation ---
#[derive(Serialize)]
pub struct SystemInstruction<'a> {
    pub parts: Vec<Part<'a>>,
}
#[derive(Serialize)]
struct GenerateContentRequest<'a> {
    contents: Vec<Content<'a>>,
    #[serde(rename = "generationConfig", skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
    pub system_instruction: Option<SystemInstruction<'a>>,
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
) -> Result<FileInfo, AiError> {
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
        return Err(AiError::ApiResponse(format!(
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
fn get_api_key() -> Result<String, AiError> {
    dotenvy::dotenv().ok();
    std::env::var("GOOGLE_API_KEY").map_err(|_| AiError::ApiKeyMissing)
}

// --- Public API Functions ---

/// Creates a simple object schema with string properties
/// let schema = gemini::create_object_schema(&[
///    ("title", "The document title"),
///    ("author", "The document author"),
///    ("topic", "Main topic discussed")
/// ]);
fn create_object_schema(properties: &[(&str, &str)]) -> serde_json::Value {
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
fn create_enum_schema(values: &[&str], description: Option<&str>) -> serde_json::Value {
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
fn create_array_schema(item_properties: &[(&str, &str)]) -> serde_json::Value {
    let item_schema = create_object_schema(item_properties);

    serde_json::json!({
        "type": "array",
        "items": item_schema
    })
}

async fn ask_about_file(
    client: &Client,
    api_key: &str,
    file_info: &FileInfo,
    prompt: &str,
    response_schema: Option<serde_json::Value>,
) -> Result<String, AiError> {
    let generation_config = response_schema.map(|schema| GenerationConfig {
        response_mime_type: Some("application/json".to_string()),
        response_schema: Some(schema),
    });

    // Step 3: Generate content using the uploaded file's URI
    let request_body = GenerateContentRequest {
        system_instruction: None,
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
        .header("X-Goog-Api-Key", api_key)
        .header(header::CONTENT_TYPE, "application/json")
        .json(&request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(AiError::ApiResponse(format!(
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
        .ok_or(AiError::EmptyResponse)
}

// --- Public API Functions ---

/// Generates text embeddings for the given input text
///
/// # Arguments
///
/// * `text` - The text to embed
/// * `task_type` - The embedding task type (e.g., RETRIEVAL_QUERY, RETRIEVAL_DOCUMENT)
/// * `output_dimensionality` - Optional output dimensionality (768, 1536, or 3072). If None, uses default 3072
///
/// # Returns
///
/// * `Result<Vec<f32>, GeminiError>` - The embedding vector
///
/// # Example
///
/// ```rust
/// use your_crate::{get_text_embedding, EmbeddingTaskType};
///
/// let embedding = get_text_embedding(
///     "What is the capital of France?",
///     Some(EmbeddingTaskType::RetrievalQuery),
///     Some(768)
/// ).await?;
/// ```
async fn get_text_embedding(
    client: &Client,
    api_key: &str,
    text: &str,
    task_type: Option<EmbeddingTaskType>,
    output_dimensionality: Option<u32>,
) -> Result<Vec<f32>, AiError> {
    // let api_key = get_api_key()?;
    // let client = Client::new();

    let request_body = EmbedContentRequest {
        content: EmbedContent {
            parts: vec![EmbedPart { text }],
        },
        task_type: task_type.as_ref(),
        output_dimensionality,
    };

    let response = client
        .post(EMBEDDING_URL)
        .header("X-Goog-Api-Key", api_key)
        .header(header::CONTENT_TYPE, "application/json")
        .json(&request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(AiError::ApiResponse(format!(
            "Embedding request failed: {}",
            error_text
        )));
    }

    let embed_response: EmbedContentResponse = response.json().await?;

    embed_response
        .embedding
        .map(|e| e.values)
        .ok_or(AiError::StructuredOutputFailed("embedding".to_string()))
}

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

const QUERY_PROMPT: &str = "r#
You are an expert research analyst and data pre-processor for a state-of-the-art AI search system. Your task is to analyze the provided research paper and generate a single, dense, keyword-rich text block optimized for vector embedding.

This output is NOT for human reading.It is designed to be embedded into a vector space for a high-recall Retrieval-Augmented Generation (RAG) system. 
The goal is to maximize the chances that this paper is found by a wide variety of relevant search queries (high recall).

**## CORE DIRECTIVES ##**

**1. Output Format - CRITICAL:**
   - Your final output must be a single, continuous block of text. Do not write prose or narrative sentences. Use keywords, key phrases, and very concise statements.
   - **You MUST NOT include the section headings (like 'KEY_RELATIONSHIPS:', 'CONCEPTS_AND_OBJECTS:', etc.) in the final output.** They are instructions for you, not text for the output.
   - Simply provide the raw extracted information, moving from one category to the next to form one seamless block of text.

**2. Token Budget and Density:**
   - The final output should be as dense as possible, aiming for a maximum of **2000 tokens**.
   - **Be Exhaustive:** Scour the entire text. Extract all significant terms, even if they are mentioned only once. Redundancy is better than omission for this task.
   - You are strongly encouraged to use all available tokens. 

**3. Content Generation - STRICT PRIORITY ORDER:**
   - You must generate the content in the following order to ensure the most critical information is included before the token limit is reached.

   **PRIORITY 1: KEY_RELATIONSHIPS:**
   - **(Generate this section first)**. This is the most important part. Extract phrases and concise statements that describe the connections *between* concepts.
   - *Examples*: 'Application of Dowker complexes to category theory,' 'Using persistent homology to analyze sensor network data,' 'A link between graph theory and spectral analysis for clustering,' 'Comparison of Dowker duality with nerve constructions.'

   **PRIORITY 2: CONCEPTS_AND_OBJECTS:**
   - **(Generate this section second)**. After exhausting relationships, list all core mathematical/scientific objects, definitions, and structures.
   - *Examples*: 'Dowker complex,' 'simplicial set,' 'nerve of a category,' 'persistent homology,' 'adjoint functor,' 'model category.'

   **PRIORITY 3: METHODS_AND_TECHNIQUES:**
   - **(Generate this section third)**. List all methods, algorithms, experimental procedures, and frameworks used or proposed.
   - *Examples*: 'spectral sequence analysis,' 'principal component analysis (PCA),' 'backpropagation,' 'finite element method.'

   **PRIORITY 4: RESULTS_AND_CONCLUSIONS:**
   - **(Generate this section fourth)**. List key findings, theorems, lemmas, and major conclusions as concise phrases.
   - *Examples*: 'Theorem 3.1: Homotopy equivalence of Dowker and Cech complexes,' 'demonstrated 5% accuracy improvement,' 'established a new lower bound.'

   **PRIORITY 5: APPLICATIONS_AND_DOMAINS:**
   - **(Generate this section last, to fill remaining tokens)**. List the fields of study, real-world problems, and application areas.
   - *Examples*: 'materials science,' 'drug discovery,' 'image recognition,' 'social network analysis,' 'theoretical computer science.'

**4. General Style:**
   - Do not write prose or narrative sentences. Use lists of keywords, key phrases, and very concise statements.
   - Include aliases and synonyms (e.g., 'Topological Data Analysis (TDA)').

**3. Provide your text in the summary_text field.** 
---

**Now, analyze the given text. Adhere strictly to all directives above to produce the single, dense text block in the summary_text field**
";

const RAG_PROMPT:&str="r## Research Paper Analysis System Prompt

You are a specialized research assistant that analyzes academic papers to find content relevant to user queries. You will receive:

1. **The PDF of a research paper** uploaded via the Gemini File API
2. **A user query/prompt** describing what information they're seeking

## Your Task

Thoroughly analyze the uploaded research paper to identify content that addresses the user's query. You MUST respond with structured JSON that follows the exact schema provided in the API configuration.

## Analysis Process

1. **Read the paper completely** - Don't just scan abstracts and conclusions
2. **Identify relevant sections** - Look for content that directly or indirectly addresses the user's query
3. **Assess relevance strength** - Determine how well each section answers the query
4. **Note page locations** - Track exactly where relevant information appears
5. **Rank paper** - By overall relevance to the query

## Critical Output Requirements

**You MUST return your response as structured JSON that conforms to the provided response schema. Do not include any text outside the JSON structure. Do not wrap the JSON in markdown code blocks.**

The response will automatically follow this structure:
- `score`: Number between 0.0-1.0 indicating relevance strength
- `explanation`: Brief explanation of relevance and contribution
- `pages`: Array of strings indicating where relevant content appears (format: 5, 10-15, etc.)

## Relevance Scoring Guidelines

- **0.9-1.0**: Paper directly addresses the query with substantial relevant content
- **0.7-0.8**: Paper addresses key aspects of the query with good detail  
- **0.5-0.6**: Paper partially addresses the query or provides background context
- **0.3-0.4**: Paper tangentially relates to the query
- **0.0-0.2**: Paper is not relevant to the query

## Explanation
- Provide a concise 1-2 sentence explanation of why you assigned this relevance score
- Focus on the key reasons for the score, mentioning specific aspects that align or don't align with the query

## Page Range Format

- List specific page numbers or ranges where relevant content is found
- Single pages: `5`
- Consecutive pages: `10-15`  
- Multiple ranges: `[1-3, 8, 12-14, 20-25]`
- Always use actual page numbers from the start of the PDF, the first page is page 1.
- Return empty array [] if no specific pages are particularly relevant or if the entire paper is uniformly relevant/irrelevant


## Analysis Guidelines

1. **Be thorough but precise** - Include all relevant content but don't inflate relevance
2. **Cite specific page ranges** - Users need to know exactly where to look
3. **Explain relevance clearly** - Help users understand why the paper matters
5. **Consider different types of relevance** - Methodological, theoretical, empirical, etc.
6. **Check all sections** - Introduction, methods, results, discussion, appendices
7. **Focus on content relevance** - not paper quality


## Special Considerations

- **Interdisciplinary queries**: Look for connections across different research domains  
- **Methodological queries**: Pay special attention to methods sections and supplementary materials
- **Recent developments**: Note if papers discuss cutting-edge or emerging topics

## Critical Reminders

- **ONLY return valid JSON** - No additional text or markdown formatting
- **Provide precise page ranges** for all relevant content
- **Be concise but informative** in explanations";

#[derive(Debug, Serialize, Deserialize)]
pub struct PaperAnalysis {
    pub score: f64,
    pub explanation: String,
    pub pages: Vec<String>,
}

/// Creates the schema for paper evaluation with relevance score, explanation, and relevant pages
/// Returns a JSON schema compatible with Gemini API's structured output requirements
fn create_paper_evaluation_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "score": {
                "type": "number",
                "description": "Relevance score between 0.0 (not relevant) and 1.0 (completely fulfills query)",
                "minimum": 0.0,
                "maximum": 1.0
            },
            "explanation": {
                "type": "string",
                "description": "Brief explanation of the relevance assessment in 1-2 sentences"
            },
            "pages": {
                "type": "array",
                "items": {
                    "type": "string"
                },
                "description": "Array of relevant page ranges (e.g., ['1', '2-5', '23-28']) or empty array if not applicable"
            }
        },
        "required": ["score", "explanation", "pages"],
        "propertyOrdering": ["score", "explanation", "pages"]
    })
}

pub struct Gemini {
    client: Client,
    api_key: String,
    uploaded_file: Option<FileInfo>,
}

impl Gemini {
    pub fn new() -> Result<Self, AiError> {
        dotenvy::dotenv().ok();
        let api_key = std::env::var("GOOGLE_API_KEY").map_err(|_| AiError::ApiKeyMissing)?;

        Ok(Self {
            client: Client::new(),
            api_key,
            uploaded_file: None,
        })
    }

    pub async fn upload_file(
        &mut self,
        file_bytes: Vec<u8>,
        mime_type: &str,
    ) -> Result<FileInfo, AiError> {
        let response = self
            .client
            .post(UPLOAD_URL)
            .header("X-Goog-Api-Key", &self.api_key)
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
            return Err(AiError::ApiResponse(format!(
                "File upload failed with status {}: {}",
                status, error_text
            )));
        }

        let response_text = response.text().await?;
        let upload_response: FileUploadResponse = serde_json::from_str(&response_text)?;

        self.uploaded_file = Some(upload_response.file.clone());
        Ok(upload_response.file)
    }

    pub async fn generate_bibtex(&self) -> Result<String, AiError> {
        let file_info = self.uploaded_file.as_ref().ok_or(AiError::NoFileUploaded)?;

        let schema = create_object_schema(&[("bibtex_entry", "The complete BibTeX entry")]);

        let response = ask_about_file(
            &self.client,
            &self.api_key,
            file_info,
            BIBTEX_PROMPT,
            Some(schema),
        )
        .await?;

        let parsed: Value = serde_json::from_str(&response)?;

        let bibtex = parsed
            .get("bibtex_entry")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AiError::StructuredOutputFailed("bibtex_entry".to_string()))?;

        Ok(bibtex.to_string())
    }

    pub async fn generate_paper_embedding(&self) -> Result<Vec<f32>, AiError> {
        let file_info = self.uploaded_file.as_ref().ok_or(AiError::NoFileUploaded)?;

        let schema = create_object_schema(&[(
            "summary_text",
            "Keyword-rich text block optimized for vector embedding",
        )]);

        let response = ask_about_file(
            &self.client,
            &self.api_key,
            file_info,
            QUERY_PROMPT,
            Some(schema),
        )
        .await?;

        let parsed: Value = serde_json::from_str(&response)?;

        let summary = parsed
            .get("summary_text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AiError::StructuredOutputFailed("summary_text".to_string()))?;

        let v = get_text_embedding(
            &self.client,
            &self.api_key,
            summary,
            Some(EmbeddingTaskType::RetrievalDocument),
            Some(786),
        )
        .await?;

        let squared_magnitude = dotzilla::dot(&v, &v);
        if squared_magnitude == 0.0 {
            return Err(AiError::EmbeddingFailed(
                "Zero magnitude vector".to_string(),
            ));
        }

        let magnitude = squared_magnitude.sqrt();
        let v = v.into_iter().map(|x| x / magnitude).collect();
        Ok(v)
    }

    pub async fn generate_query_embedding(&self, text: &str) -> Result<Vec<f32>, AiError> {
        let v = get_text_embedding(
            &self.client,
            &self.api_key,
            text,
            Some(EmbeddingTaskType::RetrievalQuery),
            Some(786),
        )
        .await?;

        let squared_magnitude = dotzilla::dot(&v, &v);
        if squared_magnitude == 0.0 {
            return Err(AiError::EmbeddingFailed(
                "Zero magnitude vector".to_string(),
            ));
        }

        let magnitude = squared_magnitude.sqrt();
        let v = v.into_iter().map(|x| x / magnitude).collect();
        Ok(v)
    }

    /// Analyzes a single research paper for relevance to a user query
    /// Returns structured, deserialized result
    pub async fn analyze_research_paper(
        &self,
        user_prompt: &str,
        file_info: &FileInfo, // Multiple files
    ) -> Result<PaperAnalysis, AiError> {
        // Create the schema automatically
        let schema = create_paper_evaluation_schema();

        // Create generation config with our schema
        let generation_config = GenerationConfig {
            response_mime_type: Some("application/json".to_string()),
            response_schema: Some(schema),
        };

        // System instruction that explains the behavior
        let system_instruction = SystemInstruction {
            parts: vec![Part::Text { text: RAG_PROMPT }],
        };

        // Build parts vector with the prompt and all files
        let mut parts = vec![Part::Text { text: user_prompt }];

        parts.push(Part::FileData {
            file_data: FileData {
                mime_type: &file_info.mime_type,
                file_uri: &file_info.uri,
            },
        });

        // Create the request
        let request_body = GenerateContentRequest {
            contents: vec![Content { parts }],
            generation_config: Some(generation_config),
            system_instruction: Some(system_instruction),
        };

        // Send request to Gemini
        let response = self
            .client
            .post(MODEL_URL)
            .header("X-Goog-Api-Key", self.api_key.clone())
            .header(header::CONTENT_TYPE, "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(AiError::ApiResponse(format!(
                "Content generation failed: {}",
                error_text
            )));
        }

        // Parse the response
        let gen_response: GenerateContentResponse = response.json().await?;

        // Extract the JSON text from the response
        let json_text = gen_response
            .candidates
            .and_then(|mut c| c.pop())
            .and_then(|c| c.content)
            .and_then(|co| co.parts)
            .and_then(|mut p| p.pop())
            .and_then(|p| p.text)
            .ok_or(AiError::EmptyResponse)?;

        // Deserialize the structured JSON into our struct
        let analysis: PaperAnalysis = serde_json::from_str(&json_text).map_err(|e| {
            AiError::StructuredOutputFailed(format!(
                "Failed to deserialize response: {}. Raw response: {}",
                e, json_text
            ))
        })?;

        Ok(analysis)
    }
}

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
