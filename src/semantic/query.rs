use crate::base::Paper;
use crate::utils::bibfile::{parse_bibliography, read_bibtex};
use anyhow::Result;
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
    #[serde(rename = "paperId")]
    scholar_id: String,
    #[serde(rename = "openAccessPdf")]
    pdf: Option<HashMap<String, String>>,
    #[serde(rename = "citationStyles")]
    citation: Option<HashMap<String, String>>,
}

fn make_request(query: &str, limit: usize) -> Result<SemanticResponse, reqwest::Error> {
    let url = format!(
        "https://api.semanticscholar.org/graph/v1/paper/search?query={}&limit={}&fields=openAccessPdf,citationStyles",
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

fn read_citation(payload: Payload) -> Option<Bibliography> {
    let citation = payload.citation?;
    let bibtex = citation.get("bibtex")?;
    match read_bibtex(&bibtex) {
        Ok(bib) => Some(bib),
        Err(_) => None,
    }
}

pub fn remove_already_present(bibfile: Bibliography, papers: &mut Vec<Paper>) {
    papers.retain(|paper| bibfile.get(&paper.entry.key).is_none());
}

pub fn query_papers(query: &str, limit: usize) -> Result<Vec<Paper>> {
    let response = make_request(query, limit)?;
    let mut papers = Vec::new();
    for paper in response.data {
        let bibliography = match read_citation(paper) {
            Some(entry) => entry,
            None => continue,
        };
        let paper = parse_bibliography(bibliography);
        papers.extend(paper)
    }
    Ok(papers)
}
