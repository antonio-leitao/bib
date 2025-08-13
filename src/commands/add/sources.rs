use super::error::DownloadError;
use anyhow::Result;
use quick_xml::{events::Event, Reader};
use reqwest::header;

// Constants
const ARXIV_DOMAINS: &[&str] = &["arxiv.org", ".arxiv.org"];
const ARXIV_API_URL: &str = "http://export.arxiv.org/api/query?id_list=";
const DOI_API_URL: &str = "http://dx.doi.org/";
/// Handles DOI lookup from arXiv
pub struct ArxivApi;

impl ArxivApi {
    pub async fn get_doi(arxiv_id: &str) -> Result<Option<String>, DownloadError> {
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
pub struct CrossRefApi;

impl CrossRefApi {
    pub async fn get_bibtex(doi: &str) -> Result<String, DownloadError> {
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
            Err(DownloadError::HttpError {
                code: response.status().as_u16(),
                message: response
                    .status()
                    .canonical_reason()
                    .unwrap_or("Unknown Reason")
                    .to_string(),
            })

            // Err(anyhow!("DOI API returned status: {}", response.status()).into())
        }
    }
}
