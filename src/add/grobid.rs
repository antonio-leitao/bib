use crate::embed::Embedder;
use crate::ui::StatusUI;
use reqwest::multipart;
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use thiserror::Error;

// --- ERRORS ---

#[derive(Error, Debug)]
pub enum GrobidError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Grobid API returned status {0}")]
    ApiError(u16),

    #[error("XML parse error: {0}")]
    XmlParse(#[from] roxmltree::Error),
}

pub type Result<T> = std::result::Result<T, GrobidError>;

// --- DATA STRUCTURES ---

#[derive(Debug, Clone)]
pub struct Paper {
    pub key: String,
    pub title: String,
    pub authors: String,
    pub year: Option<String>,
    pub paragraphs: Vec<Paragraph>,
    pub references: Vec<Reference>,
}

#[derive(Debug, Clone)]
pub struct Paragraph {
    pub text: String,            // Enriched text with [key1, key2] format
    pub cited_keys: Vec<String>, // For indexing/junction table
}

#[derive(Debug, Clone)]
pub struct Reference {
    pub key: String,
    pub title: String,
    pub authors: String,
    pub year: Option<String>,
    pub link: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EmbeddedPaper {
    pub key: String,
    pub title: String,
    pub authors: String,
    pub year: Option<String>,
    pub paragraphs: Vec<EmbeddedParagraph>,
    pub references: Vec<Reference>,
}

#[derive(Debug, Clone)]
pub struct EmbeddedParagraph {
    pub text: String,            // Enriched text preserved for LLM
    pub cited_keys: Vec<String>, // For junction table
    pub embedding: Vec<f32>,
}

impl Paper {
    pub async fn embed(self, embedder: &Embedder) -> EmbeddedPaper {
        let texts: Vec<String> = self.paragraphs.iter().map(|p| p.text.clone()).collect();
        let embeddings = embedder.embed_texts(&texts).await;

        let paragraphs = self
            .paragraphs
            .into_iter()
            .zip(embeddings)
            .map(|(p, emb)| EmbeddedParagraph {
                text: p.text,
                cited_keys: p.cited_keys,
                embedding: emb,
            })
            .collect();

        EmbeddedPaper {
            key: self.key,
            title: self.title,
            authors: self.authors,
            year: self.year,
            paragraphs,
            references: self.references,
        }
    }
}

// --- CITATION KEY GENERATION ---

fn generate_citation_key(authors: &[String], title: &str) -> String {
    let surname = authors.first().map(|a| a.as_str()).unwrap_or("unknown");

    let surname_key: String = surname
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect();

    let stop_words: HashSet<String> = stop_words::get(stop_words::LANGUAGE::English)
        .into_iter()
        .collect();

    let title_words: Vec<String> = title
        .split_whitespace()
        .map(|w| {
            w.to_lowercase()
                .chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
        })
        .filter(|w| !w.is_empty() && !stop_words.contains(w.as_str()))
        .take(2)
        .collect();

    if title_words.is_empty() {
        surname_key
    } else {
        format!("{}_{}", surname_key, title_words.join(""))
    }
}

// --- GROBID CLIENT ---

pub struct GrobidClient {
    base_url: String,
    client: reqwest::Client,
}

impl GrobidClient {
    pub async fn new() -> Result<Self> {
        let client = Self::with_url("https://antonio-leitao-grobid.hf.space")?;
        if client.is_alive().await {
            StatusUI::success("Grobid is ready");
        } else {
            let pb = StatusUI::spinner("Grobid is sleeping, waking up...");
            if !client.wait_until_ready_with_spinner(300, &pb).await {
                StatusUI::finish_spinner_error(pb, "Grobid failed to start");
                std::process::exit(1);
            }
            StatusUI::finish_spinner_success(pb, "Grobid is ready");
        }
        Ok(client)
    }

    pub fn with_url(base_url: &str) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .danger_accept_invalid_certs(true)
            .build()?;

        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client,
        })
    }

    pub async fn is_alive(&self) -> bool {
        let url = format!("{}/api/isalive", self.base_url);
        self.client
            .get(&url)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    async fn wait_until_ready_with_spinner(
        &self,
        max_wait_secs: u64,
        pb: &indicatif::ProgressBar,
    ) -> bool {
        let start = std::time::Instant::now();
        while start.elapsed().as_secs() < max_wait_secs {
            if self.is_alive().await {
                return true;
            }
            let remaining = max_wait_secs.saturating_sub(start.elapsed().as_secs());
            pb.set_message(format!("Grobid waking up... (~{}s remaining)", remaining));
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
        false
    }

    pub async fn process_pdf(&self, file_bytes: Vec<u8>) -> Result<Paper> {
        let url = format!("{}/api/processFulltextDocument", self.base_url);

        let part = multipart::Part::bytes(file_bytes)
            .file_name("research_paper.pdf")
            .mime_str("application/pdf")
            .unwrap();

        let form = multipart::Form::new()
            .part("input", part)
            .text("consolidateHeader", "1")
            .text("consolidateCitations", "1")
            .text("includeRawCitations", "1")
            .text("includeRawAffiliations", "1")
            .text("segmentSentences", "1");

        let res = self.client.post(&url).multipart(form).send().await?;

        if !res.status().is_success() {
            return Err(GrobidError::ApiError(res.status().as_u16()));
        }

        let xml = res.text().await?;
        parse_grobid_xml(&xml)
    }
}

// --- PARSING ---

const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";

fn parse_grobid_xml(xml: &str) -> Result<Paper> {
    let doc = roxmltree::Document::parse(xml)?;
    let root = doc.root_element();

    // Extract references first to build marker → key map
    let (references, marker_to_key) = extract_references(&root);

    // Extract metadata
    let (title, authors, year) = extract_metadata(&root);
    let authors_vec: Vec<String> = authors.split(", ").map(|s| s.to_string()).collect();
    let key = generate_citation_key(&authors_vec, &title);

    // Extract paragraphs with enriched text (keys inline)
    let paragraphs = extract_paragraphs(&root, &marker_to_key);

    Ok(Paper {
        key,
        title,
        authors,
        year,
        paragraphs,
        references,
    })
}

fn extract_metadata(root: &roxmltree::Node) -> (String, String, Option<String>) {
    let header = find_element(root, "teiHeader");

    let title = header
        .as_ref()
        .and_then(|n| find_element(n, "title"))
        .map(get_all_text)
        .unwrap_or_else(|| "Unknown Title".to_string());

    let mut authors = Vec::new();
    if let Some(h) = header {
        if let Some(source) = find_element(&h, "sourceDesc") {
            for author in find_elements(&source, "author") {
                if let Some(name) = extract_author_surname(&author) {
                    authors.push(name);
                }
            }
        }
    }

    let year = header
        .as_ref()
        .and_then(|n| find_element(n, "date"))
        .and_then(|t| t.attribute("when").map(extract_year));

    (title, authors.join(", "), year)
}

fn extract_references(root: &roxmltree::Node) -> (Vec<Reference>, HashMap<String, String>) {
    let mut references = Vec::new();
    let mut marker_to_key = HashMap::new();

    let Some(list) = find_element(root, "listBibl") else {
        return (references, marker_to_key);
    };

    for node in find_elements(&list, "biblStruct") {
        let marker = node.attribute((XML_NS, "id")).map(|s| s.to_string());

        let title = find_elements(&node, "title")
            .find(|n| n.attribute("level") == Some("a"))
            .or_else(|| find_element(&node, "title"))
            .map(get_all_text)
            .unwrap_or_else(|| "Untitled".to_string());

        let authors: Vec<String> = find_elements(&node, "author")
            .filter_map(|a| extract_author_surname(&a))
            .collect();

        let year = find_elements(&node, "date")
            .find(|n| n.attribute("type") == Some("published"))
            .and_then(|n| n.attribute("when").map(extract_year));

        let key = generate_citation_key(&authors, &title);

        let link = resolve_link(&node);

        if let Some(m) = &marker {
            marker_to_key.insert(m.clone(), key.clone());
        }

        references.push(Reference {
            key,
            title,
            authors: authors.join(", "),
            year,
            link,
        });
    }

    (references, marker_to_key)
}

fn extract_paragraphs(
    root: &roxmltree::Node,
    marker_to_key: &HashMap<String, String>,
) -> Vec<Paragraph> {
    let Some(body) = find_element(root, "body") else {
        return Vec::new();
    };

    let mut result = Vec::new();

    // Buffers for the "current logical paragraph" being constructed
    let mut current_text = String::new();
    let mut current_cites: Vec<String> = Vec::new();
    let mut is_buffer_active = false;

    for p_node in find_elements(&body, "p") {
        // 1. Extract data from the current XML node
        let text_chunk = extract_paragraph_text(p_node, marker_to_key);
        let cites_chunk = extract_node_citations(p_node, marker_to_key);

        if text_chunk.trim().is_empty() {
            continue;
        }

        // 2. Determine if this chunk continues the previous one
        // Heuristic: If it starts with a lowercase letter, it's a continuation.
        let starts_lowercase = text_chunk
            .trim_start() // Handle potential leading formatting spaces
            .chars()
            .next()
            .map(char::is_lowercase)
            .unwrap_or(false);

        if is_buffer_active && starts_lowercase {
            // MERGE: This is the second half of a split sentence.

            // Handle hyphenation (e.g., "approxi-" + "mation")
            if current_text.trim_end().ends_with('-') {
                // Remove the hyphen and join directly
                let trimmed = current_text.trim_end().trim_end_matches('-');
                current_text = format!("{}{}", trimmed, text_chunk);
            } else {
                // Standard join with a space
                current_text.push(' ');
                current_text.push_str(&text_chunk);
            }

            // Accumulate citations
            current_cites.extend(cites_chunk);
        } else {
            // NEW BLOCK: The previous buffer is finished.
            // Flush it if it was active and valid.
            if is_buffer_active {
                push_if_valid(&mut result, current_text, current_cites);
            }

            // Start a new buffer
            current_text = text_chunk;
            current_cites = cites_chunk;
            is_buffer_active = true;
        }
    }

    // Flush the final remaining buffer
    if is_buffer_active {
        push_if_valid(&mut result, current_text, current_cites);
    }

    result
}

// --- Helper to finalize and store a paragraph ---
fn push_if_valid(result: &mut Vec<Paragraph>, text: String, mut cites: Vec<String>) {
    // Deduplicate citations accumulated from multiple chunks
    cites.sort();
    cites.dedup();

    // CRITERIA: Only keep paragraphs that actually cite something
    if !cites.is_empty() {
        result.push(Paragraph {
            text,
            cited_keys: cites,
        });
    }
}

// --- Helper to extract citation keys from a specific node ---
fn extract_node_citations(
    node: roxmltree::Node,
    marker_to_key: &HashMap<String, String>,
) -> Vec<String> {
    let mut markers = Vec::new();
    for ref_node in find_elements(&node, "ref") {
        if let Some(target) = ref_node.attribute("target") {
            for part in target.split_whitespace() {
                let clean = part.trim_start_matches('#');
                if clean.starts_with('b') {
                    markers.push(clean.to_string());
                }
            }
        }
    }

    markers
        .iter()
        .filter_map(|m| marker_to_key.get(m).cloned())
        .collect()
}

fn extract_paragraph_text(
    node: roxmltree::Node,
    marker_to_key: &HashMap<String, String>,
) -> String {
    let mut buffer = String::new();
    extract_text_recursive(node, &mut buffer, marker_to_key);
    buffer.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn extract_text_recursive(
    node: roxmltree::Node,
    buffer: &mut String,
    marker_to_key: &HashMap<String, String>,
) {
    if node.is_text() {
        if let Some(text) = node.text() {
            buffer.push_str(text);
        }
        return;
    }

    // Check if this is a citation reference
    if node.tag_name().name() == "ref" {
        if let Some(target) = node.attribute("target") {
            if target.contains("#b") {
                // Resolve markers to keys and format as [key1, key2]
                let keys: Vec<String> = target
                    .split_whitespace()
                    .filter_map(|part| {
                        let clean = part.trim_start_matches('#');
                        if clean.starts_with('b') {
                            marker_to_key.get(clean).cloned()
                        } else {
                            None
                        }
                    })
                    .collect();

                if !keys.is_empty() {
                    buffer.push_str(&format!("[{}]", keys.join(", ")));
                }
                return;
            }
        }
    }

    for child in node.children() {
        extract_text_recursive(child, buffer, marker_to_key);
    }
}

fn extract_idno(node: &roxmltree::Node, id_type: &str) -> Option<String> {
    find_elements(node, "idno")
        .find(|n| {
            n.attribute("type")
                .map(|t| t.eq_ignore_ascii_case(id_type))
                .unwrap_or(false)
        })
        .and_then(|n| n.text())
        .map(|s| s.trim().to_string())
}

fn resolve_link(node: &roxmltree::Node) -> Option<String> {
    if let Some(doi) = extract_idno(node, "DOI") {
        let clean = doi
            .trim_start_matches("DOI:")
            .trim_start_matches("doi:")
            .trim();
        return Some(format!("https://doi.org/{}", clean));
    }

    if let Some(arxiv) = extract_idno(node, "arxiv") {
        let clean = arxiv
            .trim_start_matches("arXiv:")
            .trim_start_matches("arxiv:")
            .trim();
        return Some(format!("https://arxiv.org/abs/{}", clean));
    }

    if let Some(pmid) = extract_idno(node, "PMID") {
        let clean = pmid
            .trim_start_matches("PMID:")
            .trim_start_matches("pmid:")
            .trim();
        return Some(format!("https://pubmed.ncbi.nlm.nih.gov/{}", clean));
    }

    find_element(node, "ptr")
        .and_then(|n| n.attribute("target").or_else(|| n.text()))
        .map(|s| s.trim().to_string())
}
// --- HELPERS ---

fn extract_year(date_str: &str) -> String {
    date_str.split('-').next().unwrap_or(date_str).to_string()
}

fn find_element<'a>(
    node: &'a roxmltree::Node<'a, 'a>,
    name: &str,
) -> Option<roxmltree::Node<'a, 'a>> {
    node.descendants().find(|n| n.tag_name().name() == name)
}

fn find_elements<'a>(
    node: &'a roxmltree::Node<'a, 'a>,
    name: &str,
) -> impl Iterator<Item = roxmltree::Node<'a, 'a>> {
    node.descendants()
        .filter(move |n| n.tag_name().name() == name)
}

fn get_all_text(node: roxmltree::Node) -> String {
    let mut result = String::new();
    collect_text(node, &mut result);
    result.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn collect_text(node: roxmltree::Node, buffer: &mut String) {
    // FIX: Only append text if the node IS a text node.
    // Calling .text() on an Element node returns the first child's text,
    // which causes duplication when we subsequently iterate over children.
    if node.is_text() {
        if let Some(text) = node.text() {
            buffer.push_str(text);
        }
    }

    for child in node.children() {
        collect_text(child, buffer);
    }
}

fn extract_author_surname(node: &roxmltree::Node) -> Option<String> {
    find_element(node, "surname")?.text().map(|s| s.to_string())
}
