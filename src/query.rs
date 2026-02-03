use crate::database::{CitationDb, DbError, Paper, ParagraphContext, format_authors};
use crate::embed::Embedder;
use crate::ui::StatusUI;
use adamastor::{Agent, schema};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum QueryError {
    #[error("Database Error: {0}")]
    DatabaseError(#[from] DbError),
    #[error("LLM Error: {0}")]
    LlmError(#[from] adamastor::AdamastorError),
    #[error("Pandoc Error: {0}")]
    PandocError(String),
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, QueryError>;

// --- STRUCTURED OUTPUT ---

#[schema]
pub struct QueryResponse {
    /// Ordered list of papers that best answer the query. Most relevant first.
    pub papers: Vec<RankedPaper>,
}

#[schema]
pub struct RankedPaper {
    /// The paper key exactly as it appears in the citation brackets, e.g. "smith_topology"
    pub key: String,
    /// Two or three sentences explaining why this paper answers the query, based on how it is cited
    pub explanation: String,
    /// Keys of papers that cite this paper in a way relevant to the query (max 3)
    pub cited_by: Vec<String>,
}

// --- PROMPT ---

const RERANK_PROMPT: &str = r#"You are a research librarian helping find papers that answer a specific query.

## Your Task
Given a query and a set of citation contexts from academic papers, identify which CITED papers best answer the query.

## How This Works
Each context is a paragraph from a paper that cites other papers. Citations appear as [paper_key] or [key1, key2].
Your job is to find papers that are cited IN THE CONTEXT OF answering the query.

## Critical Rules
1. ANSWER THE QUERY FIRST. Only include papers that directly address what the user is asking.
2. DO NOT include papers just because they are frequently cited or foundational. A famous paper cited 100 times is IRRELEVANT if it doesn't answer the specific query.
3. Look for papers cited when authors discuss the query topic. If someone asks about "applications of X" and a context says "X has been applied to images [paper_a] and audio [paper_b]", those papers answer the query.
4. IGNORE papers cited for background, methodology, or unrelated context.
5. The explanation must come from HOW THE PAPER IS CITED, not from your general knowledge.

## Example
Query: "topological data analysis for protein structure"

Context (from: chen_review, similarity: 0.87):
"Persistent homology has been successfully applied to protein structure analysis [smith_proteins, jones_homology], building on earlier work in algebraic topology [munkres_topology]."

Good response:
- smith_proteins: "Applied persistent homology to protein structure analysis" (cited_by: chen_review)
- jones_homology: "Applied persistent homology to protein structure analysis" (cited_by: chen_review)

Do NOT include munkres_topology - it's cited as background, not as an application to proteins.

## Output Format
Order the papers by relevance to query (best first). Include max 20 papers.
Each paper needs: key (exact match from citations), explanation (2-3 sentences from context), cited_by (source papers, max 3).

If NO papers in the contexts actually answer the query, return an empty papers array. Do not force irrelevant results.

---

QUERY: {query}

CONTEXTS:
{contexts}"#;

// const REPORT_PROMPT: &str = r#"You are an academic researcher writing a "Previous Research" or "Background" section for a paper.
//
// ## Your Task
// Write a comprehensive, cohesive literature review answering the provided query based STRICTLY on the provided contexts.
//
// ## Output Format
// - Return PURE Markdown content (with LaTeX math).
// - Do NOT include YAML frontmatter or document structure.
// - Use [@paper_key] for single citations (Pandoc format).
// - Use [@key1; @key2] for grouped citations.
//
// ## Mathematical Notation
// - Inline math: $d_I(M, N) \leq \epsilon$ renders inline
// - Display math: $$d_I(M, N) = \inf\{\epsilon \geq 0 : M \text{ and } N \text{ are } \epsilon\text{-interleaved}\}$$
// - For multi-line equations use align: \begin{align} ... \end{align}
// - Common symbols: \mathbb{R}, \mathcal{F}, \partial, \otimes, \cong, \hookrightarrow
//
// ## Formatting Guidelines
// - Use Markdown headers (##, ###) to organize by theme
// - Use bullet lists (-) for enumerating properties or approaches
// - Use numbered lists (1.) for sequential steps or chronological developments
// - Use **bold** for key terms on first introduction
// - Avoid code blocks (```) - this is prose, not code
//
// ## Style Guidelines
// 1. **Aggregation**: Do not just list papers. Group them by theme or finding.
//    - BAD: "Smith studied X [@smith]. Jones studied X [@jones]."
//    - GOOD: "Several studies have investigated X, focusing on its stability [@smith; @jones] and performance [@brown]."
// 2. **Relevance**: Focus entirely on the user's query.
// 3. **Tone**: Formal, academic, objective.
// 4. **Accuracy**: Only attribute findings to a paper if the provided context explicitly supports it.
//
// ---
//
// QUERY: {query}
//
// CONTEXTS:
// {contexts}"#;
//
const REPORT_PROMPT: &str = r#"
## Your Task
Write a comprehensive, cohesive literature review answering the provided query based **STRICTLY AND EXCLUSIVELY** on the provided contexts. This should read like a "Related Work" or "Background" section of an academic paper.

---

## CRITICAL: Strict Grounding Requirement

You are provided with a **context pool**: excerpts from academic papers where specific works have been cited. These contexts are your ONLY source of information.

### Absolute Rules
1. **EVERY claim, finding, or statement must originate from a provided context**
2. **EVERY claim must cite the paper(s) from which it derives**
3. **If a context doesn't support a claim, you CANNOT make that claim**
4. **DO NOT use outside knowledge**—even if you "know" something is true, if it's not in the contexts, don't include it
5. **If contexts are insufficient to answer the query, explicitly state what cannot be addressed**

### What Counts as "Inventing"
❌ Generalizing beyond what contexts state
❌ Inferring results that aren't explicitly mentioned
❌ Connecting papers in ways the contexts don't support
❌ Adding background knowledge not present in contexts
❌ Assuming a method/finding exists because it's "common" in the field

### Traceability Test
For every sentence, you must be able to point to the specific context(s) that justify it. If you cannot, delete the sentence.

---

## Citation & Attribution Requirements

### Every Sentence Needs Grounding
Since all information comes from contexts, nearly every substantive sentence should have a citation.

❌ BAD (ungrounded claims):
> Deep learning has revolutionized NLP. Transformer models are particularly effective for sequence tasks. BERT introduced bidirectional pre-training [@bert].

Only the last sentence is grounded. The first two are "common knowledge" insertions—**forbidden**.

✅ GOOD (fully grounded):
> Bidirectional pre-training, as introduced in BERT [@bert], addresses the limitation of unidirectional language models by conditioning on both left and right context [@bert]. This approach was later extended to generation tasks through various masking strategies [@t5; @bart].

### Attribution Precision
Match claims to the papers that actually make them in your contexts.

❌ BAD (misattribution):
> Context says: "Smith et al. found X, extending the work of Jones."
> You write: "X was discovered by Jones [@jones]."

✅ GOOD (precise attribution):
> Smith et al. demonstrated X, building on the foundational framework of Jones [@smith; @jones].

### When Multiple Contexts Discuss the Same Topic
Synthesize them, but maintain accurate attribution for each specific claim.

✅ EXAMPLE:
> Attention mechanisms have been applied to graphs through two main paradigms: additive attention over neighbors [@gat] and multi-head dot-product attention adapted from Transformers [@graphormer]. The former preserves the locality of message passing [@gat], while the latter enables global receptive fields at the cost of quadratic complexity [@graphormer; @nodeformer].

Each claim traces to its source.

---

## Core Principle: Query-Driven Relevance

Every sentence you write must either:
1. **Directly answer** the query using information from contexts, or
2. **Provide essential context** (from the context pool) without which the direct answer would be incomprehensible

### What to INCLUDE
- Findings from contexts that directly address the query
- Definitions/concepts from contexts *only if* the query's answer depends on them
- Methodological details from contexts *only if* the query asks about methods

### What to EXCLUDE
- Information from contexts that doesn't inform the query (even if interesting)
- Any knowledge not present in the contexts
- Speculation about what papers "might" have found

---

## Handling Insufficient Contexts

If the provided contexts do not adequately address the query:

1. **Answer what you can** from available contexts
2. **Explicitly state the gap**: "The provided sources do not address [specific aspect]."
3. **DO NOT fill gaps with outside knowledge**

✅ EXAMPLE:
> The contexts discuss over-smoothing mitigation through residual connections [@chen; @li] and normalization [@zhao]. However, the provided sources do not cover spectral approaches to this problem, limiting the scope of this review to spatial methods.

---

## Output Format
- Return PURE Markdown content (with LaTeX math where appropriate)
- Do NOT include YAML frontmatter, titles, or meta-commentary
- Start directly with substantive content

## Citation Format (Pandoc)
- Single: [@paper_key]
- Multiple: [@key1; @key2; @key3]

## Mathematical Notation
- Inline: $d_I(M, N) \leq \epsilon$
- Display: $$d_I(M, N) = \inf\{\epsilon \geq 0 : ...\}$$
- Multi-line: \begin{align}...\end{align}

## Formatting
- Use ## and ### headers to organize by **theme relevant to the query**
- Bullet lists (-) for properties or parallel approaches
- Numbered lists (1.) for sequential processes
- **Bold** key terms on first introduction only

---

## Style Requirements

### 1. Synthesize, Don't Summarize
Group findings by theme, but ensure each claim remains properly attributed.

❌ BAD (paper-by-paper, loses synthesis):
> Smith et al. proposed method A [@smith]. Jones et al. proposed method B [@jones]. Lee et al. compared methods [@lee].

❌ ALSO BAD (synthesized but attribution lost):
> Several methods have been proposed for this task, including hierarchical and flat approaches.

✅ GOOD (synthesized with precise attribution):
> Approaches to graph pooling fall into two categories: hierarchical methods that progressively coarsen the graph [@jones; @lee] and flat methods that produce a single global representation [@smith]. Comparative studies suggest hierarchical pooling excels on graphs with meaningful substructure [@lee], though flat pooling remains competitive for small molecular graphs [@smith].

### 2. Stay On-Query
Only include context information that serves the query.

❌ BAD (tangential context inclusion):
> The query asks about over-smoothing, but a context mentions a paper's dataset details.
> You write: "Chen et al. evaluated on Cora, CiteSeer, and PubMed datasets [@chen]."

✅ GOOD (relevant extraction):
> The query asks about over-smoothing.
> Context says: "Chen et al. introduced residual connections to GNNs, evaluating on Cora, CiteSeer, and PubMed, showing that depth could be increased from 2 to 16 layers without performance degradation."
> You write: "Residual connections enable training substantially deeper GNNs—up to 16 layers—without the performance degradation typically caused by over-smoothing [@chen]."

### 3. Faithful Paraphrasing
Restate context information accurately without distortion.

❌ BAD (distortion):
> Context: "Method X showed a 5% improvement on dataset Y."
> You write: "Method X dramatically outperforms all baselines."

✅ GOOD (faithful):
> Method X demonstrated a 5% improvement on dataset Y [@paper].

---

## Quality Checklist (Self-Verify Before Responding)
- [ ] Can every sentence be traced to a specific context?
- [ ] Is every claim cited with the correct paper(s)?
- [ ] Did I avoid inserting any outside knowledge?
- [ ] Does every paragraph connect to the query?
- [ ] Did I explicitly note any gaps in context coverage?
- [ ] Did I synthesize by theme rather than listing papers?

---

**QUERY:** {query}

**CONTEXTS:**
{contexts}"#;

// --- SIMILARITY RESULTS ---

pub struct SimilarityResults {
    pub contexts: Vec<ParagraphContext>,
    pub sim_map: HashMap<i64, f32>,
}

// --- SIMILARITY SEARCH ---

async fn similarity_search(
    db: &CitationDb,
    query_string: &str,
) -> Result<Option<SimilarityResults>> {
    let embedder = Embedder::new();

    let pb = StatusUI::spinner("Embedding query and searching...");
    let query_vector = embedder.embed_query(query_string).await;

    // Get all embeddings (lightweight, just vectors)
    let embeddings = db.get_all_embeddings()?;

    // Compute similarities and get top N
    let similarity_threshold = 0.3;
    let max_contexts = 500; // How many paragraphs to send to LLM

    let mut scored: Vec<(i64, f32)> = embeddings
        .into_iter()
        .map(|e| {
            let sim = caravela::dot(&query_vector, &e.embedding);
            (e.id, sim)
        })
        .filter(|(_, sim)| *sim >= similarity_threshold)
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    scored.truncate(max_contexts);

    if scored.is_empty() {
        StatusUI::finish_spinner_warning(pb, "No results found. Try a different query.");
        return Ok(None);
    }

    StatusUI::finish_spinner_info(pb, &format!("Found {} relevant contexts", scored.len()));

    // Get full paragraph contexts for top matches
    let top_ids: Vec<i64> = scored.iter().map(|(id, _)| *id).collect();
    let sim_map: HashMap<i64, f32> = scored.into_iter().collect();
    let contexts = db.get_paragraph_contexts(&top_ids)?;

    Ok(Some(SimilarityResults { contexts, sim_map }))
}

// --- AI ANALYSIS ---

async fn ai_analysis(
    db: &CitationDb,
    query_string: &str,
    results: &SimilarityResults,
    top_k: usize,
) -> Result<()> {
    // Build context string for prompt
    let context_str: String = results
        .contexts
        .iter()
        .map(|ctx| {
            let sim = results.sim_map.get(&ctx.id).unwrap_or(&0.0);
            format!(
                "Context (from: {}, similarity: {:.2}):\n\"{}\"",
                ctx.source_key, sim, ctx.text
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    // Build final prompt
    let prompt = RERANK_PROMPT
        .replace("{query}", query_string)
        .replace("{contexts}", &context_str);

    // Call LLM
    let pb = StatusUI::spinner("Analyzing with LLM...");
    let api_key = std::env::var("GEMINI_KEY").expect("Set GEMINI_KEY environment variable");
    let agent = Agent::new(api_key).with_model("gemini-2.5-flash");

    let response: QueryResponse = agent.prompt(&prompt).temperature(0.2).await?;
    StatusUI::finish_spinner_success(pb, "Analysis complete");

    if response.papers.is_empty() {
        StatusUI::warning("No papers found that directly answer the query.");
        return Ok(());
    }

    // Collect all keys we need to look up (result papers + citing papers)
    let mut all_keys: Vec<&str> = response.papers.iter().map(|p| p.key.as_str()).collect();
    for p in &response.papers {
        all_keys.extend(p.cited_by.iter().map(|s| s.as_str()));
    }
    all_keys.sort();
    all_keys.dedup();

    // Fetch paper metadata
    let papers = db.get_papers(&all_keys, false)?;
    let paper_map: HashMap<&str, &Paper> = papers.iter().map(|p| (p.key.as_str(), p)).collect();

    // Display results
    let (width, _) = termion::terminal_size().unwrap_or((80, 24));
    let width = width as usize;

    for result in response.papers.iter().take(top_k) {
        let paper = paper_map.get(result.key.as_str());

        // Paper line: YEAR Authors • Title (truncated to width)
        let (year, authors, title) = match paper {
            Some(p) => (
                p.year
                    .map(|y| format!("{}", y))
                    .unwrap_or_else(|| "----".into()),
                format_authors(p.authors.as_deref()),
                p.title.as_deref().unwrap_or("Untitled"),
            ),
            None => ("----".into(), "Unknown".into(), result.key.as_str()),
        };
        let header = format!("{} {} • {}", year, authors, title);
        let header = truncate_to_width(&header, width);
        println!("\n{}", header);

        // Explanation with tree characters, word-wrapped
        let indent = "  ";
        let tree_first = "├─ ";
        let tree_cont = "│  ";
        // Prefix width: 2 (indent) + 3 (tree chars) = 5
        let text_width = width.saturating_sub(5);

        let wrapped = wrap_text(&result.explanation, text_width);
        let has_evidence = !result.cited_by.is_empty();

        for (i, line) in wrapped.iter().enumerate() {
            if i == 0 {
                println!("{}{}{}", indent, tree_first, line);
            } else {
                println!("{}{}{}", indent, tree_cont, line);
            }
        }

        // Evidence line
        if has_evidence {
            let keys: String = result.cited_by.join(", ");
            println!("{}└─ Refs: {}", indent, keys);
        } else {
            println!("{}└─", indent);
        }
    }
    Ok(())
}

// --- REPORT GENERATION ---

async fn generate_report(
    db: &CitationDb,
    query_string: &str,
    results: &SimilarityResults,
    _top_k: usize,
) -> Result<()> {
    // 1. Gather Contexts and referenced Keys
    let context_str = format_contexts(results);

    // Identify all keys mentioned in the paragraphs (both source and cited)
    let mut relevant_keys = HashSet::new();
    for ctx in &results.contexts {
        relevant_keys.insert(ctx.source_key.clone());
        for cited in &ctx.cited_keys {
            relevant_keys.insert(cited.clone());
        }
    }

    // 2. Fetch Paper Metadata for Bibliography
    let keys_vec: Vec<&str> = relevant_keys.iter().map(|s| s.as_str()).collect();
    let papers = db.get_papers(&keys_vec, false)?;

    if papers.is_empty() {
        StatusUI::error("No bibliography data found for contexts.");
        return Ok(());
    }

    // 3. Generate Content via LLM
    let pb = StatusUI::spinner("Generating research report...");
    let prompt = REPORT_PROMPT
        .replace("{query}", query_string)
        .replace("{contexts}", &context_str);

    let api_key = std::env::var("GEMINI_KEY").expect("Set GEMINI_KEY environment variable");
    let agent = Agent::new(api_key).with_model("gemini-2.5-flash");

    // We use a schema to force the model to separate the Latex body from any conversational fluff
    let response: String = agent.prompt(&prompt).temperature(0.3).await?;
    StatusUI::finish_spinner_success(pb, "Content generated");

    // 4. Create Temporary Build Environment
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let temp_dir = std::env::temp_dir().join(format!("bib_report_{}", timestamp));
    fs::create_dir_all(&temp_dir)?;

    // 5. Generate Markdown and CSL-JSON bibliography files
    let md_content = wrap_markdown_document(query_string, &response);
    let bib_content = generate_csl_json(&papers);

    let md_path = temp_dir.join("report.md");
    let bib_path = temp_dir.join("refs.json");
    fs::write(&md_path, &md_content)?;
    fs::write(&bib_path, &bib_content)?;

    // 6. Compile PDF with Pandoc (single pass)
    let pb = StatusUI::spinner("Compiling PDF (pandoc)...");

    if let Err(e) = run_pandoc(&temp_dir, "report.md", "refs.json", "report.pdf") {
        StatusUI::finish_spinner_error(pb, "Compilation failed");
        return Err(e);
    }

    StatusUI::finish_spinner_success(pb, "PDF Compiled");

    // 7. Move result to CWD
    let pdf_source = temp_dir.join("report.pdf");
    if !pdf_source.exists() {
        return Err(QueryError::PandocError("PDF was not created".to_string()));
    }

    let sanitized_query: String = query_string
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    let output_filename = format!("report_{}.pdf", sanitized_query);
    let output_path = std::env::current_dir()?.join(&output_filename);

    fs::copy(pdf_source, &output_path)?;

    // Clean up temp directory
    let _ = fs::remove_dir_all(temp_dir);

    StatusUI::success(&format!("Report saved to: {}", output_filename));

    Ok(())
}

fn format_contexts(results: &SimilarityResults) -> String {
    results
        .contexts
        .iter()
        .map(|ctx| {
            let sim = results.sim_map.get(&ctx.id).unwrap_or(&0.0);
            format!(
                "Context (from: {}, similarity: {:.2}):\n\"{}\"",
                ctx.source_key, sim, ctx.text
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Generate CSL-JSON bibliography for Pandoc
fn generate_csl_json(papers: &[Paper]) -> String {
    let entries: Vec<String> = papers
        .iter()
        .map(|p| {
            let authors = parse_authors_to_csl(p.authors.as_deref().unwrap_or("Unknown"));
            let title = p.title.as_deref().unwrap_or("Unknown Title");
            let year = p.year.unwrap_or(0);
            let url = p.link.as_deref().unwrap_or("");

            let mut entry = format!(
                r#"  {{
    "id": "{}",
    "type": "article",
    "author": [{}],
    "title": "{}",
    "issued": {{"date-parts": [[{}]]}}"#,
                escape_json(&p.key),
                authors,
                escape_json(title),
                year
            );

            if !url.is_empty() {
                entry.push_str(&format!(",\n    \"URL\": \"{}\"", escape_json(url)));
            }

            entry.push_str("\n  }");
            entry
        })
        .collect();

    format!("[\n{}\n]", entries.join(",\n"))
}

/// Parse author string into CSL-JSON author array
fn parse_authors_to_csl(authors: &str) -> String {
    // Split on " and " or ", " to handle common formats
    let author_list: Vec<&str> = if authors.contains(" and ") {
        authors.split(" and ").collect()
    } else {
        authors.split(", ").collect()
    };

    let csl_authors: Vec<String> = author_list
        .iter()
        .map(|a| {
            let trimmed = a.trim();
            // Try to split into family/given name
            if let Some(comma_pos) = trimmed.find(',') {
                // "Family, Given" format
                let family = trimmed[..comma_pos].trim();
                let given = trimmed[comma_pos + 1..].trim();
                format!(
                    r#"{{"family": "{}", "given": "{}"}}"#,
                    escape_json(family),
                    escape_json(given)
                )
            } else {
                // "Given Family" or just "Name" format
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 {
                    let family = parts.last().unwrap();
                    let given = parts[..parts.len() - 1].join(" ");
                    format!(
                        r#"{{"family": "{}", "given": "{}"}}"#,
                        escape_json(family),
                        escape_json(&given)
                    )
                } else {
                    format!(r#"{{"family": "{}"}}"#, escape_json(trimmed))
                }
            }
        })
        .collect();

    csl_authors.join(", ")
}

/// Escape string for JSON
fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Wrap content in Markdown document with YAML frontmatter
fn wrap_markdown_document(title: &str, body: &str) -> String {
    // Generate date string from SystemTime
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    // Calculate date components from Unix timestamp
    // Days since epoch, then derive year/month/day
    let days_since_epoch = now / 86400;
    let (year, month, day) = days_to_ymd(days_since_epoch as i64);
    let date_str = format!("{:04}-{:02}-{:02}", year, month, day);

    format!(
        r#"---
title: "Research Report: {title}"
author: "Generated by Grobit"
date: "{date}"
---

{body}

## References
"#,
        title = title,
        date = date_str,
        body = body
    )
}

/// Convert days since Unix epoch to (year, month, day)
fn days_to_ymd(days: i64) -> (i32, u32, u32) {
    // Algorithm adapted from Howard Hinnant's date algorithms
    // http://howardhinnant.github.io/date_algorithms.html
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32; // day of era [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // year of era [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // day of year [0, 365]
    let mp = (5 * doy + 2) / 153; // month index [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // day [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // month [1, 12]
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m, d)
}

/// Run Pandoc to generate PDF
fn run_pandoc(dir: &Path, md_file: &str, bib_file: &str, output_file: &str) -> Result<()> {
    let output = Command::new("pandoc")
        .current_dir(dir)
        .arg("--citeproc")
        .arg(format!("--bibliography={}", bib_file))
        .arg("--pdf-engine=xelatex")
        .arg("-V")
        .arg("geometry:margin=1in")
        .arg(md_file)
        .arg("-o")
        .arg(output_file)
        .output()
        .map_err(|_| QueryError::PandocError("pandoc not found".to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(QueryError::PandocError(format!(
            "pandoc failed:\n{}",
            stderr
        )));
    }
    Ok(())
}

// --- MAIN QUERY FUNCTION ---

pub async fn query(db: &CitationDb, query_string: &str, top_k: usize, report: bool) -> Result<()> {
    let results = match similarity_search(db, query_string).await? {
        Some(r) => r,
        None => return Ok(()),
    };

    if report {
        generate_report(db, query_string, &results, top_k).await
    } else {
        ai_analysis(db, query_string, &results, top_k).await
    }
}

fn truncate_to_width(s: &str, max_width: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_width {
        s.to_string()
    } else {
        format!(
            "{}...",
            chars[..max_width.saturating_sub(3)]
                .iter()
                .collect::<String>()
        )
    }
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let words: Vec<&str> = paragraph.split_whitespace().collect();
        if words.is_empty() {
            lines.push(String::new());
            continue;
        }

        let mut current_line = String::new();
        for word in words {
            if current_line.is_empty() {
                current_line = word.to_string();
            } else if current_line.len() + 1 + word.len() <= width {
                current_line.push(' ');
                current_line.push_str(word);
            } else {
                lines.push(current_line);
                current_line = word.to_string();
            }
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}
