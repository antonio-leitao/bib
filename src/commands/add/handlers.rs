use super::error::{AddError, DownloadError, InputError};
use super::sources::{ArxivApi, CrossRefApi};
use crate::ai::Gemini;
use crate::bibtex::BibtexParser;
use crate::ui::StatusUI;
use arboard::Clipboard;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;
use url::Url;

const ARXIV_DOMAINS: &[&str] = &["arxiv.org", ".arxiv.org"];

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

pub struct PdfHandler;

impl PdfHandler {
    pub async fn get_pdf_source(input: Option<String>) -> Result<PdfSource, AddError> {
        let input = match input {
            Some(input) => input,
            None => Self::get_clipboard_content()?,
        };

        let input_type = Self::classify_input(&input)?;
        Self::fetch_pdf_source(input_type).await
    }

    fn get_clipboard_content() -> Result<String, AddError> {
        let mut clipboard = Clipboard::new()?;
        let text = clipboard.get_text()?;

        if text.trim().is_empty() {
            return Err(AddError::EmptyClipboard);
        }

        Ok(text)
    }

    fn classify_input(input: &str) -> Result<InputType, InputError> {
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
                    return Err(InputError::NotPdfUrl(input.to_string()));
                }
                Err(_) => return Err(InputError::InvalidUrl(input.to_string())),
            }
        }

        let path = PathBuf::from(input);
        if Self::is_pdf_file(&path) {
            Ok(InputType::PdfPath(path))
        } else {
            Err(InputError::NotPdfPath(path))
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

    async fn fetch_pdf_source(input_type: InputType) -> Result<PdfSource, AddError> {
        match input_type {
            InputType::ArxivUrl(url) => {
                let arxiv_id = Self::extract_arxiv_id(&url)?;
                StatusUI::info(&format!("Source: arXiv paper ({})", arxiv_id));

                let pdf_url = format!("https://arxiv.org/pdf/{}.pdf", arxiv_id);
                let bytes = Self::download_pdf(&pdf_url).await?;

                Ok(PdfSource {
                    bytes,
                    arxiv_id: Some(arxiv_id),
                })
            }
            InputType::PdfUrl(url) => {
                StatusUI::info("Source: PDF URL");
                let bytes = Self::download_pdf(&url).await?;
                Ok(PdfSource {
                    bytes,
                    arxiv_id: None,
                })
            }
            InputType::PdfPath(path) => {
                StatusUI::info(&format!("Source: local file: {}", path.display()));
                let bytes = Self::read_pdf_file(&path)?;
                Ok(PdfSource {
                    bytes,
                    arxiv_id: None,
                })
            }
        }
    }

    fn extract_arxiv_id(arxiv_url: &str) -> Result<String, InputError> {
        let url = Url::parse(arxiv_url)
            .map_err(|_| InputError::InvalidArxivUrl(arxiv_url.to_string()))?;

        let path_segments: Vec<&str> = url
            .path_segments()
            .ok_or_else(|| InputError::InvalidArxivUrl(arxiv_url.to_string()))?
            .collect();

        for (i, &segment) in path_segments.iter().enumerate() {
            if matches!(segment, "abs" | "pdf") {
                if let Some(&arxiv_id) = path_segments.get(i + 1) {
                    return Ok(arxiv_id.to_string());
                }
            }
        }

        Err(InputError::InvalidArxivUrl(arxiv_url.to_string()))
    }

    async fn download_pdf(url: &str) -> Result<Vec<u8>, DownloadError> {
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
            return Err(match status.as_u16() {
                403 => DownloadError::Forbidden,
                404 => DownloadError::NotFound,
                401 => DownloadError::Unauthorized,
                429 => DownloadError::RateLimited,
                code => DownloadError::HttpError {
                    code,
                    message: format!("Unexpected status code"),
                },
            });
        }

        let total_size = response.content_length().unwrap_or(0);
        let progress_bar = if total_size > 0 {
            let url_obj = Url::parse(url).ok();
            let domain = url_obj
                .and_then(|u| u.domain().map(|d| d.to_string()))
                .unwrap_or_else(|| "source".to_string());
            StatusUI::download_progress(&format!("Downloading from {}", domain), total_size)
        } else {
            StatusUI::spinner("Downloading PDF content...")
        };

        let content = response.bytes().await?;

        let size_str = StatusUI::format_file_size(content.len());
        StatusUI::finish_progress_success(progress_bar, &format!("Downloaded {}", size_str));

        Ok(content.to_vec())
    }

    fn read_pdf_file(path: &Path) -> Result<Vec<u8>, AddError> {
        let mut file = File::open(path)?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;

        let size_str = StatusUI::format_file_size(contents.len());
        StatusUI::success(&format!("Read {}", size_str));

        Ok(contents)
    }
}

pub struct BibtexGenerator;

impl BibtexGenerator {
    pub async fn generate_bibtex(
        ai: &mut Gemini,
        pdf_source: PdfSource,
    ) -> Result<String, AddError> {
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
    ) -> Result<String, AddError> {
        let spinner = StatusUI::spinner("Checking for DOI on arXiv...");

        match ArxivApi::get_doi(arxiv_id).await {
            Ok(Some(doi)) => {
                StatusUI::finish_spinner_success(spinner, &format!("Found DOI: {}", doi));

                let crossref_spinner = StatusUI::spinner("Fetching official bibtex from DOI...");

                match CrossRefApi::get_bibtex(&doi).await {
                    Ok(bibtex) => {
                        ai.upload_file(pdf_bytes, "application/pdf").await?;
                        StatusUI::finish_spinner_success(
                            crossref_spinner,
                            "Retrieved official bibtex from DOI",
                        );
                        return Ok(bibtex);
                    }
                    Err(_) => {
                        StatusUI::finish_spinner_warning(
                            crossref_spinner,
                            "DOI lookup failed, using AI fallback",
                        );
                    }
                }
            }
            Ok(None) => {
                StatusUI::finish_spinner_warning(spinner, "No DOI found on arXiv, using AI");
            }
            Err(_) => {
                StatusUI::finish_spinner_warning(spinner, "arXiv lookup failed, using AI fallback");
            }
        }

        Self::generate_bibtex_ai(ai, pdf_bytes).await
    }

    async fn generate_bibtex_with_doi_upgrade(
        ai: &mut Gemini,
        pdf_bytes: Vec<u8>,
    ) -> Result<String, AddError> {
        let mut bibtex = Self::generate_bibtex_ai(ai, pdf_bytes).await?;

        if let Some(doi) = BibtexParser::extract_doi(&bibtex) {
            let upgrade_spinner = StatusUI::spinner("Upgrading bibtex with official version...");

            match CrossRefApi::get_bibtex(&doi).await {
                Ok(official_bibtex) => {
                    StatusUI::finish_spinner_success(
                        upgrade_spinner,
                        "Upgraded to official bibtex from DOI",
                    );
                    bibtex = official_bibtex;
                }
                Err(_) => {
                    StatusUI::finish_spinner_warning(
                        upgrade_spinner,
                        "Failed to upgrade, using AI version",
                    );
                }
            }
        }

        Ok(bibtex)
    }

    async fn generate_bibtex_ai(ai: &mut Gemini, pdf_bytes: Vec<u8>) -> Result<String, AddError> {
        let spinner = StatusUI::spinner("Extracting bibtex using Gemini AI...");
        ai.upload_file(pdf_bytes, "application/pdf").await?;
        let bibtex_entry = ai.generate_bibtex().await?;
        StatusUI::finish_spinner_success(spinner, "Extracted bibtex using Gemini AI");
        Ok(bibtex_entry)
    }
}
