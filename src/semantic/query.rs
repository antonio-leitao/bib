use crate::base::{MetaData, Paper};
use crate::utils::bibfile::{parse_entry, read_bibtex};
use crate::utils::ui;
use anyhow::{anyhow, Result};
use biblatex::{Bibliography, Entry};
use reqwest::blocking::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;

#[derive(Deserialize)]
struct SemanticResponse {
    data: Vec<Payload>,
}

#[derive(Deserialize)]
struct Payload {
    #[serde(rename = "openAccessPdf")]
    pdf: Option<HashMap<String, String>>,
    #[serde(rename = "citationStyles")]
    citation: Option<HashMap<String, String>>,
}

//make single paper query (arxiv and id)

fn make_arxiv_request(query: &str) -> Result<Payload, reqwest::Error> {
    let base_url = format!(
            "https://api.semanticscholar.org/graph/v1/paper/URL:{}?&fields=citationStyles,openAccessPdf",
            query
        );
    // Fetch the API key from the environment variable
    let api_key = env::var("SCHOLAR_KEY").expect("SCHOLAR_KEY environment variable not set");
    // Create a reqwest client and set the x-api-key header
    let client = Client::new();
    // Deserialize the response into the custom structure
    let response = client.get(&base_url).header("x-api-key", api_key).send()?;
    response.json()
}

fn make_query_request(query: &str, limit: usize) -> Result<SemanticResponse, reqwest::Error> {
    let url = format!(
        "https://api.semanticscholar.org/graph/v1/paper/search?query={}&limit={}&fields=citationStyles,openAccessPdf",
        query, limit
    );
    // Fetch the API key from the environment variable
    let api_key = env::var("SCHOLAR_KEY").expect("SCHOLAR_KEY environment variable not set");
    // Create a reqwest client and set the x-api-key header
    let client = Client::new();
    // Deserialize the response into the custom structure
    let response = client.get(&url).header("x-api-key", api_key).send()?;
    response.json()
}

fn read_citation(payload: &Payload) -> Option<Bibliography> {
    let citation = payload.citation.clone()?;
    let bibtex = citation.get("bibtex")?;
    match read_bibtex(&bibtex) {
        Ok(bib) => Some(bib),
        Err(_) => None,
    }
}

fn read_metadata(paper: &Payload) -> MetaData {
    let url = match &paper.pdf {
        Some(pdf) => pdf.get("url"),
        None => None,
    };
    MetaData {
        pdf: url.cloned(),
        notes: None,
        last_accessed: None,
    }
}

fn unwravel_response(paper: &Payload) -> (Option<Entry>, MetaData) {
    let entry = match read_citation(&paper) {
        Some(bib) => bib.into_iter().next(),
        None => None,
    };
    let meta = read_metadata(&paper);
    (entry, meta)
}

fn parse_paper_request(payload: Payload) -> Result<Paper> {
    let (entry, metadata) = match unwravel_response(&payload) {
        (Some(entry), data) => (entry, data),
        (None, _) => return Err(anyhow!("Unable to read citation")),
    };
    parse_entry(entry, Some(metadata)).map_err(|err| anyhow!(err))
}

pub fn query_arxiv_paper(query: &str) -> Result<Paper> {
    let spinner = ui::Spinner::new("Searching online".to_string());
    spinner.start();
    let payload = make_arxiv_request(query)?;
    spinner.stop();
    parse_paper_request(payload)
}

pub fn query_batch_papers(query: &str, limit: usize) -> Result<Vec<Paper>> {
    let spinner = ui::Spinner::new("Searching online".to_string());
    spinner.start();
    let response = make_query_request(query, limit)?;
    spinner.stop();
    let mut papers = Vec::new();
    for payload in response.data {
        match parse_paper_request(payload) {
            Ok(paper) => papers.push(paper),
            Err(_) => continue,
        }
    }
    Ok(papers)
}
