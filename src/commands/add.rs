use crate::base::{Paper, PdfStorage, UI};
use crate::gemini::Gemini;
use crate::store::{PaperStore, StoreError};
use crate::{bibtex, blog, blog_done, gemini};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{anyhow, Result};
use arboard::Clipboard;
use quick_xml::{events::Event, Reader};
use regex::Regex;
use reqwest::header;
use thiserror::Error;
use url::Url;

// Constants
const ARXIV_DOMAINS: &[&str] = &["arxiv.org", ".arxiv.org"];
const ARXIV_API_URL: &str = "http://export.arxiv.org/api/query?id_list=";
const DOI_API_URL: &str = "http://dx.doi.org/";

#[derive(Error, Debug)]
pub enum BibError {
    #[error("Clipboard is empty or contains unsupported content")]
    ClipboardEmpty,
    #[error("Clipboard error: {0}")]
    Clipboard(#[from] arboard::Error),
    #[error("Invalid arXiv URL: {0}")]
    InvalidArxivUrl(String),
    #[error("URL does not point to a PDF: {0}")]
    NonPdfUrl(String),
    #[error("Path does not point to a PDF file: {0}")]
    NonPdfPath(PathBuf),
    #[error("Download failed: {0}")]
    DownloadError(#[from] anyhow::Error),
    #[error("File I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Gemini API error: {0}")]
    GeminiError(#[from] gemini::GeminiError),
    #[error("Failed to parse BibTeX response: {0}")]
    BibTeXParseError(String),
    #[error("Network request failed: {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("Failed to parse XML response: {0}")]
    XmlParseError(#[from] quick_xml::Error),
    #[error("Failed to parse biblatex: {0}")]
    BibtexParse(#[from] bibtex::BibtexError),
    #[error("Store error: {0}")]
    Store(#[from] StoreError),
    #[error("PDF storage error: {0}")]
    PdfStorage(#[from] crate::base::PdfError),
}

#[derive(Debug)]
pub struct PdfSource {
    pub bytes: Vec<u8>,
    pub arxiv_id: Option<String>,
}

#[derive(Debug, PartialEq)]
enum InputType {
    ArxivUrl(String),
    PdfUrl(String),
    PdfPath(PathBuf),
}

/// Handles PDF acquisition and source identification
struct PdfHandler;

impl PdfHandler {
    async fn get_pdf_source(input: Option<String>) -> Result<PdfSource, BibError> {
        let input = match input {
            Some(input) => input,
            None => Self::get_clipboard_content()?,
        };

        let input_type = Self::classify_input(&input)?;
        Self::fetch_pdf_source(input_type).await
    }

    fn get_clipboard_content() -> Result<String, BibError> {
        let mut clipboard = Clipboard::new()?;
        let text = clipboard.get_text()?;

        if text.trim().is_empty() {
            return Err(BibError::ClipboardEmpty);
        }

        Ok(text)
    }

    fn classify_input(input: &str) -> Result<InputType, BibError> {
        let input = input.trim();

        if input.contains("://") {
            match Url::parse(input) {
                Ok(url) => {
                    if Self::is_arxiv_url(&url) {
                        return Ok(InputType::ArxivUrl(input.to_string()));
                    }
                    if Self::is_pdf_url(&url) {
                        return Ok(InputType::PdfUrl(input.to_string()));
                    }
                    return Err(BibError::NonPdfUrl(input.to_string()));
                }
                Err(_) => return Err(BibError::NonPdfUrl(input.to_string())),
            }
        }

        let path = PathBuf::from(input);
        if Self::is_pdf_file(&path) {
            Ok(InputType::PdfPath(path))
        } else {
            Err(BibError::NonPdfPath(path))
        }
    }

    fn is_arxiv_url(url: &Url) -> bool {
        if let Some(domain) = url.domain() {
            ARXIV_DOMAINS.iter().any(|&arxiv_domain| {
                domain.eq_ignore_ascii_case("arxiv.org") || domain.ends_with(arxiv_domain)
            })
        } else {
            false
        }
    }

    fn is_pdf_url(url: &Url) -> bool {
        url.path().to_lowercase().ends_with(".pdf")
    }

    fn is_pdf_file(path: &Path) -> bool {
        path.is_file()
            && path
                .extension()
                .map_or(false, |ext| ext.to_ascii_lowercase() == "pdf")
    }

    async fn fetch_pdf_source(input_type: InputType) -> Result<PdfSource, BibError> {
        match input_type {
            InputType::ArxivUrl(url) => {
                let arxiv_id = Self::extract_arxiv_id(&url)?;
                blog!("Source", "arXiv paper ({})", arxiv_id);

                let pdf_url = format!("https://arxiv.org/pdf/{}.pdf", arxiv_id);
                let bytes = Self::download_pdf(&pdf_url).await?;

                Ok(PdfSource {
                    bytes,
                    arxiv_id: Some(arxiv_id),
                })
            }
            InputType::PdfUrl(url) => {
                blog!("Source", "PDF URL");
                let bytes = Self::download_pdf(&url).await?;
                Ok(PdfSource {
                    bytes,
                    arxiv_id: None,
                })
            }
            InputType::PdfPath(path) => {
                blog!("Source", "local file: {}", path.display());
                let bytes = Self::read_pdf_file(&path)?;
                Ok(PdfSource {
                    bytes,
                    arxiv_id: None,
                })
            }
        }
    }

    fn extract_arxiv_id(arxiv_url: &str) -> Result<String, BibError> {
        let url =
            Url::parse(arxiv_url).map_err(|_| BibError::InvalidArxivUrl(arxiv_url.to_string()))?;

        let path_segments: Vec<&str> = url
            .path_segments()
            .ok_or_else(|| BibError::InvalidArxivUrl(arxiv_url.to_string()))?
            .collect();

        for (i, &segment) in path_segments.iter().enumerate() {
            if matches!(segment, "abs" | "pdf") {
                if let Some(&arxiv_id) = path_segments.get(i + 1) {
                    return Ok(arxiv_id.to_string());
                }
            }
        }

        Err(BibError::InvalidArxivUrl(arxiv_url.to_string()))
    }

    async fn download_pdf(url: &str) -> Result<Vec<u8>, BibError> {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .timeout(Duration::from_secs(30))
            .build()?;

        let response = client
            .get(url)
            .header("Accept", "application/pdf,application/octet-stream,*/*")
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_msg = match status.as_u16() {
                403 => {
                    format!(
                        "Access forbidden (HTTP 403) when downloading from '{}'.\n\
                        This usually means:\n\
                        • The website requires authentication or a subscription\n\
                        • The URL contains an expired access token\n\
                        • The website blocks automated downloads\n\
                        \n\
                        For academic papers, try:\n\
                        • Try downloading the paper manually and provide the local path.\n\
                        • Using your institutional access through a library\n\
                        • Checking if the paper is available on arXiv\n\
                        • Using Sci-Hub (if legally permitted in your jurisdiction)\n\
                        • Contacting the authors for a copy",
                        url
                    )
                }
                404 => format!("File not found (HTTP 404): '{}'", url),
                401 => format!("Authentication required (HTTP 401): '{}'", url),
                429 => format!("Rate limited (HTTP 429): '{}'. Try again later.", url),
                _ => format!("HTTP {} when downloading from '{}'", status, url),
            };
            return Err(anyhow!(error_msg).into());
        }

        let total_size = response.content_length().unwrap_or(0);
        let progress_bar = if total_size > 0 {
            UI::download_progress(total_size, url)
        } else {
            UI::spinner("Downloading", "PDF content...")
        };

        let content = response.bytes().await?;

        let size_str = Self::format_file_size(content.len());
        UI::finish_with_message(progress_bar, "Downloaded", &size_str);

        Ok(content.to_vec())
    }

    fn read_pdf_file(path: &Path) -> Result<Vec<u8>, BibError> {
        let mut file = File::open(path)?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;

        let size_str = Self::format_file_size(contents.len());
        blog_done!("read", "{}", size_str);

        Ok(contents)
    }

    fn format_file_size(bytes: usize) -> String {
        if bytes > 1024 * 1024 {
            format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
        } else if bytes > 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else {
            format!("{} bytes", bytes)
        }
    }
}

/// Handles DOI lookup from arXiv
struct ArxivApi;

impl ArxivApi {
    async fn get_doi(arxiv_id: &str) -> Result<Option<String>, BibError> {
        let url = format!("{}{}", ARXIV_API_URL, arxiv_id);
        let response_xml = reqwest::get(&url).await?.text().await?;

        let mut reader = Reader::from_str(&response_xml);
        reader.trim_text(true);
        let mut buf = Vec::new();
        let mut in_doi_tag = false;

        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(e) if e.name().as_ref() == b"arxiv:doi" => in_doi_tag = true,
                Event::Text(e) if in_doi_tag => {
                    return Ok(Some(e.unescape()?.to_string()));
                }
                Event::End(e) if e.name().as_ref() == b"arxiv:doi" => in_doi_tag = false,
                Event::Eof => break,
                _ => (),
            }
            buf.clear();
        }

        Ok(None)
    }
}

/// Handles BibTeX retrieval from CrossRef
struct CrossRefApi;

impl CrossRefApi {
    async fn get_bibtex(doi: &str) -> Result<String, BibError> {
        let client = reqwest::Client::new();
        let url = format!("{}{}", DOI_API_URL, doi);

        let response = client
            .get(&url)
            .header(header::ACCEPT, "text/bibliography; style=bibtex")
            .send()
            .await?;

        if response.status().is_success() {
            Ok(response.text().await?)
        } else {
            Err(anyhow!("DOI API returned status: {}", response.status()).into())
        }
    }
}

/// Handles BibTeX parsing and DOI extraction
struct BibtexParser;

impl BibtexParser {
    fn extract_doi(bibtex: &str) -> Option<String> {
        let bibtex_lower = bibtex.to_lowercase();

        // Try DOI field first
        if let Some(doi) = Self::extract_doi_field(&bibtex_lower) {
            return Some(doi);
        }

        // Try URL field
        Self::extract_doi_from_url(&bibtex_lower)
    }

    fn extract_doi_field(bibtex: &str) -> Option<String> {
        let re = Regex::new(r#"doi\s*=\s*[{"]([^"}]+)["}]"#).ok()?;
        re.captures(bibtex)?.get(1).map(|m| m.as_str().to_string())
    }

    fn extract_doi_from_url(bibtex: &str) -> Option<String> {
        let re = Regex::new(r#"url\s*=\s*[{"]https?://(?:dx\.)?doi\.org/([^"}]+)["}]"#).ok()?;
        re.captures(bibtex)?.get(1).map(|m| m.as_str().to_string())
    }
}

/// Main BibTeX generation orchestrator
struct BibTeXGenerator;

impl BibTeXGenerator {
    async fn generate_bibtex(ai: &mut Gemini, pdf_source: PdfSource) -> Result<String, BibError> {
        if let Some(arxiv_id) = pdf_source.arxiv_id {
            Self::generate_bibtex_arxiv(ai, pdf_source.bytes, &arxiv_id).await
        } else {
            Self::generate_bibtex_with_doi_upgrade(ai, pdf_source.bytes).await
        }
    }

    async fn generate_bibtex_arxiv(
        ai: &mut Gemini,
        pdf_bytes: Vec<u8>,
        arxiv_id: &str,
    ) -> Result<String, BibError> {
        // Try DOI lookup first
        let spinner = UI::spinner("checking", "for DOI on arXiv...");

        match ArxivApi::get_doi(arxiv_id).await {
            Ok(Some(doi)) => {
                UI::finish_with_message(spinner, "Found", &format!("DOI: {}", doi));

                // Get official BibTeX
                let crossref_spinner = UI::spinner("fetching", "official bibtex from DOI...");

                match CrossRefApi::get_bibtex(&doi).await {
                    Ok(bibtex) => {
                        //if we are skipping ai we have to make sure we are uploading the file
                        ai.upload_file(pdf_bytes, "application/pdf").await?;
                        UI::finish_with_message(
                            crossref_spinner,
                            "Retrieved",
                            "official bibtex from DOI",
                        );
                        return Ok(bibtex);
                    }
                    Err(_) => {
                        UI::finish_with_message(
                            crossref_spinner,
                            "Failed",
                            "DOI lookup, using AI fallback",
                        );
                    }
                }
            }
            Ok(None) => {
                UI::finish_with_message(spinner, "no DOI", "Found on arXiv, using AI");
            }
            Err(_) => {
                UI::finish_with_message(spinner, "Failed", "arXiv lookup, using AI fallback");
            }
        }

        // Fallback to AI
        Self::generate_bibtex_ai(ai, pdf_bytes).await
    }

    async fn generate_bibtex_with_doi_upgrade(
        ai: &mut Gemini,
        pdf_bytes: Vec<u8>,
    ) -> Result<String, BibError> {
        // Generate with AI first
        let mut bibtex = Self::generate_bibtex_ai(ai, pdf_bytes).await?;

        // Try to upgrade with official version
        if let Some(doi) = BibtexParser::extract_doi(&bibtex) {
            let upgrade_spinner = UI::spinner("upgrading", "bibtex with official version...");

            match CrossRefApi::get_bibtex(&doi).await {
                Ok(official_bibtex) => {
                    UI::finish_with_message(
                        upgrade_spinner,
                        "upgraded",
                        "to official bibtex from DOI",
                    );
                    bibtex = official_bibtex;
                }
                Err(_) => {
                    UI::finish_with_message(
                        upgrade_spinner,
                        "failed",
                        "to upgrade, using AI version",
                    );
                }
            }
        }

        Ok(bibtex)
    }

    async fn generate_bibtex_ai(ai: &mut Gemini, pdf_bytes: Vec<u8>) -> Result<String, BibError> {
        let spinner = UI::spinner("Extracting", "bibtex using Gemini AI...");
        ai.upload_file(pdf_bytes, "application/pdf").await?;
        let bibtex_entry = ai.generate_bibtex().await?;
        UI::finish_with_message(spinner, "Extracted", "bibtex using Gemini AI");
        Ok(bibtex_entry)
    }
}

/// Helper function to prompt user for confirmation
fn prompt_user_confirmation(message: &str) -> Result<bool, BibError> {
    println!("\n{} (y/n)", message);

    use std::io::{self, BufRead};
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut response = String::new();
    handle.read_line(&mut response)?;

    Ok(response.trim().to_lowercase() == "y")
}

/// Process a new paper entry (create and save everything)
async fn process_new_paper(
    store: &mut PaperStore,
    ai: &mut Gemini,
    paper: &Paper,
    pdf_bytes: &[u8],
) -> Result<(), BibError> {
    // Generate embedding
    let spinner = UI::spinner("Generating", "Paper embedding...");
    let embedding = ai.generate_paper_embedding().await?;
    UI::finish_with_message(
        spinner,
        "Generated",
        &format!("Paper embedding, dimensions: {}", embedding.len()),
    );

    // Save paper to database
    store.create(&paper)?;

    // Save embedding to database
    store.save_embedding(paper.id, &embedding)?;

    // Save PDF file
    let spinner = UI::spinner("Saving", "PDF to disk...");
    let pdf_path = PdfStorage::save_pdf(&pdf_bytes, &paper)?;
    let size_str = PdfStorage::format_file_size(pdf_bytes.len());
    UI::finish_with_message(
        spinner,
        "Saved PDF",
        &format!(
            "{} ({})",
            pdf_path.file_name().unwrap().to_string_lossy(),
            size_str
        ),
    );

    blog_done!("Saved", "{}", paper.title);
    blog!("PDF Path", "{}", pdf_path.display());

    Ok(())
}

/// Process an update to existing paper
async fn process_paper_update(
    store: &mut PaperStore,
    ai: &mut Gemini,
    paper: &Paper,
    pdf_bytes: &[u8],
) -> Result<(), BibError> {
    // Update paper in database
    store.update(&paper)?;

    // Generate and update embedding
    let spinner = UI::spinner("Updating", "Paper embedding...");
    let embedding = ai.generate_paper_embedding().await?;
    UI::finish_with_message(
        spinner,
        "Updated",
        &format!("Paper embedding, dimensions: {}", embedding.len()),
    );

    store.save_embedding(paper.id, &embedding)?;

    // Save or update PDF file
    let spinner = UI::spinner("Updating", "PDF on disk...");
    let pdf_path = PdfStorage::save_pdf(&pdf_bytes, &paper)?;
    let size_str = PdfStorage::format_file_size(pdf_bytes.len());
    UI::finish_with_message(
        spinner,
        "Updated PDF",
        &format!(
            "{} ({})",
            pdf_path.file_name().unwrap().to_string_lossy(),
            size_str
        ),
    );

    blog_done!("Updated", "Paper successfully updated in database");
    blog!("PDF Path", "{}", pdf_path.display());

    Ok(())
}

/// Main entry point - now saves to the store with cleaned up logic
pub async fn add(
    input: Option<String>,
    notes: Option<String>,
    store: &mut PaperStore,
) -> Result<(), BibError> {
    // Get PDF source (bytes + optional arXiv ID)
    let pdf_source = PdfHandler::get_pdf_source(input).await?;
    let pdf_bytes = pdf_source.bytes.clone();

    // Start Gemini
    let mut ai = Gemini::new()?;

    // Generate BibTeX using appropriate strategy
    let bibtex = BibTeXGenerator::generate_bibtex(&mut ai, pdf_source).await?;

    // Create Paper from BibTeX
    let paper = Paper::from_bibtex(bibtex, notes)?;

    // Check if paper already exists
    if store.exists_by_key(&paper.key)? {
        blog!("Status", "Paper already exists with key: {}", paper.key);

        // Check if PDF already exists
        if paper.pdf_exists() {
            blog!(
                "PDF Status",
                "PDF already exists at: {}",
                paper.pdf_path().display()
            );
        }

        // Ask user if they want to update
        if prompt_user_confirmation("Would you like to update the existing entry?")? {
            process_paper_update(store, &mut ai, &paper, &pdf_bytes).await?;
        } else {
            blog!("Skipped", "Paper not saved");
        }
    } else {
        // New paper - create everything
        process_new_paper(store, &mut ai, &paper, &pdf_bytes).await?;
    }

    Ok(())
}
