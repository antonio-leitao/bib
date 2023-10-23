use crate::base::{MetaData, Paper};
use crate::utils::bibfile::{parse_entry, read_bibtex};
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
    #[serde(rename = "paperId")]
    scholar_id: String,
    #[serde(rename = "citationStyles")]
    citation: Option<HashMap<String, String>>,
}

fn make_request(query: &str, limit: usize) -> Result<SemanticResponse, reqwest::Error> {
    let url = format!(
        "https://api.semanticscholar.org/graph/v1/paper/search?query={}&limit={}&fields=citationStyles",
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

fn parse_online_bibliography(bibliography: Bibliography, semantic_id: String) -> Result<Paper> {
    let meta = MetaData {
        semantic_id: Some(semantic_id),
        pdf: None,
        notes: None,
    };
    match bibliography.into_iter().next() {
        Some(entry) => {
            parse_entry(entry, Some(meta)).map_err(|err| anyhow!("Error reading paper {}", err))
        }
        None => Err(anyhow!("Error reading paper")),
    }
}

pub fn remove_already_present(bibfile: Bibliography, papers: &mut Vec<Paper>) {
    papers.retain(|paper| bibfile.get(&paper.entry.key).is_none());
}

pub fn query_papers(query: &str, limit: usize) -> Result<Vec<Paper>> {
    let response = make_request(query, limit)?;
    let mut papers = Vec::new();
    for paper in response.data {
        let bibliography = match read_citation(&paper) {
            Some(bib) => bib,
            None => continue,
        };
        match parse_online_bibliography(bibliography, paper.scholar_id) {
            Ok(paper) => papers.push(paper),
            Err(_) => continue,
        }
    }
    Ok(papers)
}
