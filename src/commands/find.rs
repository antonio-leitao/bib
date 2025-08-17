use crate::ai::{Gemini, PaperAnalysis};
use crate::core::{Embedding, Paper};
use crate::pdf::PdfStorage;
use crate::storage::PaperStore;
use crate::ui::StatusUI;
use futures;
use indicatif::ProgressBar;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use termion;
use termion::color;
use thiserror::Error;
use tokio::fs;
use tokio::sync::Mutex;
use tokio::time::sleep;

#[derive(Error, Debug)]
pub enum FindError {
    #[error("Storage error: {0}")]
    Storage(#[from] crate::storage::StorageError),

    #[error("AI processing error: {0}")]
    Ai(#[from] crate::ai::AiError),

    #[error("PDF handling error: {0}")]
    Pdf(#[from] crate::pdf::PdfError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("No papers found matching criteria")]
    NoPapersFound,
}

#[derive(Debug)]
struct AnalysisResult {
    paper_id: u128,
    analysis: PaperAnalysis,
}

#[derive(Debug)]
struct AnalysisError {
    paper_id: u128,
    error: String,
}

async fn analyze_single_paper(
    paper: Paper,
    query: String,
    progress: Arc<Mutex<(ProgressBar, usize, usize)>>,
) -> Result<AnalysisResult, AnalysisError> {
    let paper_id = paper.id;
    let paper_title = paper.title.clone();

    let result = async {
        let pdf_path = PdfStorage::get_pdf_path(&paper.key, paper.id);
        let file_bytes = fs::read(&pdf_path)
            .await
            .map_err(|e| format!("Failed to read PDF: {}", e))?;

        let mut paper_ai =
            Gemini::new().map_err(|e| format!("Failed to create Gemini client: {}", e))?;

        let file_handle = paper_ai
            .upload_file(file_bytes, "application/pdf")
            .await
            .map_err(|e| format!("Failed to upload PDF: {}", e))?;

        let analysis = paper_ai
            .analyze_research_paper(&query, &file_handle)
            .await
            .map_err(|e| format!("Analysis failed: {}", e))?;

        Ok::<PaperAnalysis, String>(analysis)
    }
    .await;

    let mut progress_guard = progress.lock().await;
    progress_guard.1 += 1;
    let completed = progress_guard.1;

    match result {
        Ok(analysis) => {
            progress_guard.0.set_position(completed as u64);
            progress_guard.0.set_message(format!(
                "analyzed: {}",
                fit_string_to_length(&paper_title, 40)
            ));

            Ok(AnalysisResult { paper_id, analysis })
        }
        Err(error) => {
            progress_guard.0.set_position(completed as u64);
            progress_guard
                .0
                .set_message(format!("error: {}", fit_string_to_length(&paper_title, 40)));

            Err(AnalysisError { paper_id, error })
        }
    }
}

fn fit_string_to_length(input: &str, max_length: usize) -> String {
    if input.len() <= max_length {
        return String::from(input);
    }
    let mut result = String::with_capacity(max_length);
    result.push_str(&input[..max_length - 3]);
    result.push_str("...");
    result
}

fn similarity_threshold_filter(
    vectors: Vec<Embedding>,
    query: &[f32],
    k: usize,
    threshold: f32,
) -> Vec<(u128, f32)> {
    if vectors.is_empty() || k == 0 {
        return Vec::new();
    }

    let mut scores: Vec<_> = vectors
        .into_iter()
        .map(|e| (e.id, dotzilla::dot(query, &e.coords)))
        .collect();

    let k = k.min(scores.len());

    scores.select_nth_unstable_by(k - 1, |a, b| {
        b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal)
    });

    scores[..k].sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
    scores.truncate(k);

    scores
        .into_iter()
        .filter(|(_, sim)| *sim >= threshold)
        .collect()
}

pub async fn execute(
    store: &mut PaperStore,
    query: &str,
    limit: usize,
    threshold: f32,
) -> Result<(), FindError> {
    let spinner = StatusUI::spinner("Generating query embedding...");
    let ai = Gemini::new()?;
    let query_vector = ai.generate_query_embedding(query).await?;
    StatusUI::finish_spinner_success(spinner, "Generated query embedding");

    let spinner = StatusUI::spinner("Searching data for relevant papers...");
    let vectors = store.load_all_embeddings()?;
    let relevant_papers = similarity_threshold_filter(vectors, &query_vector, limit, threshold);
    StatusUI::finish_spinner_success(
        spinner,
        &format!("Found {} relevant papers", relevant_papers.len()),
    );

    let id_list: Vec<u128> = relevant_papers.into_iter().map(|(id, _)| id).collect();
    let papers = store.get_by_ids(&id_list)?;
    let total_papers = papers.len();

    if total_papers == 0 {
        StatusUI::warning("No papers to analyze");
        return Err(FindError::NoPapersFound);
    }

    let stagger_delay_ms = match total_papers {
        1..=5 => 300,
        6..=10 => 1300,
        11..=20 => 3000,
        _ => 5000,
    };

    StatusUI::info(&format!(
        "Analyzing {} papers concurrently with {}ms stagger delay",
        total_papers, stagger_delay_ms
    ));

    let pb = StatusUI::concurrent_progress("Analyzing", total_papers as u64);
    pb.set_message("starting...");

    let progress = Arc::new(Mutex::new((pb.clone(), 0usize, total_papers)));

    let query_string = query.to_string();
    let analysis_futures: Vec<_> = papers
        .clone()
        .into_iter()
        .enumerate()
        .map(|(index, paper)| {
            let query_clone = query_string.clone();
            let progress_clone = Arc::clone(&progress);
            let delay = (index as u64) * stagger_delay_ms;

            tokio::spawn(async move {
                if delay > 0 {
                    sleep(Duration::from_millis(delay)).await;
                }
                analyze_single_paper(paper, query_clone, progress_clone).await
            })
        })
        .collect();

    let results = futures::future::join_all(analysis_futures).await;

    pb.finish_and_clear();

    let mut successful_analyses = Vec::new();
    let mut failed_analyses = Vec::new();

    for result in results {
        match result {
            Ok(Ok(analysis)) => successful_analyses.push(analysis),
            Ok(Err(error)) => failed_analyses.push(error),
            Err(join_error) => {
                StatusUI::warning(&format!("Task panic: {}", join_error));
            }
        }
    }

    StatusUI::success(&format!(
        "Analyzed {}/{} papers successfully",
        successful_analyses.len(),
        total_papers
    ));

    // Sort successful analyses by score (higher is better) and filter out score 0
    let mut successful_analyses: Vec<_> = successful_analyses
        .into_iter()
        .filter(|result| result.analysis.score > 0.25)
        .collect();
    successful_analyses.sort_by(|a, b| {
        b.analysis
            .score
            .partial_cmp(&a.analysis.score)
            .unwrap_or(Ordering::Equal)
    });

    // Display successful analyses
    if !successful_analyses.is_empty() {
        println!(); // Add spacing

        // Get terminal width for proper formatting
        let (width, _) = termion::terminal_size()?;

        // Create a HashMap for quick paper lookup using all papers
        let paper_map: HashMap<u128, &Paper> = papers.iter().map(|p| (p.id, p)).collect();

        for result in successful_analyses {
            // Get the paper details
            if let Some(paper) = paper_map.get(&result.paper_id) {
                // Format page ranges: Vec<String> -> [2,5-6,8] format
                let page_display = format_page_ranges(&result.analysis.pages);
                let page_display_len = page_display.len();

                // Calculate available width for paper display
                // Reserve space for: page display + spacing
                let available_width = width.saturating_sub(page_display_len as u16 + 2);

                // Display paper info with page ranges
                print!("   • {}", paper.display(available_width));
                println!(
                    " {}{}{}",
                    color::Fg(color::Yellow),
                    page_display,
                    color::Fg(color::Reset)
                );

                // Wrap and indent the explanation
                let explanation_width = (width as usize).saturating_sub(5); // 8 spaces for indentation
                let wrapped_explanation =
                    wrap_text(&result.analysis.explanation, explanation_width);
                for line in wrapped_explanation {
                    println!(
                        "     {}{}{}",
                        color::Fg(color::Rgb(83, 110, 122)),
                        line,
                        color::Fg(color::Reset)
                    );
                }
            }
        }
    } else {
        StatusUI::warning("No relevant papers found with score > 0.25");
    }

    // Report any errors
    if !failed_analyses.is_empty() {
        println!(); // Add spacing
        StatusUI::warning("FAILED ANALYSES:");
        for error in failed_analyses {
            StatusUI::error(&format!("Paper ID {}: {}", error.paper_id, error.error));
        }
    }

    Ok(())
}

/// Format page ranges from Vec<String> to display format like [2,5-6,8]
fn format_page_ranges(pages: &[String]) -> String {
    if pages.is_empty() {
        return String::from("");
    }

    // Parse page numbers and sort them
    let mut page_nums: Vec<i32> = pages.iter().filter_map(|p| p.trim().parse().ok()).collect();
    page_nums.sort_unstable();

    if page_nums.is_empty() {
        return format!("[{}]", pages.join(","));
    }

    // Group consecutive pages into ranges
    let mut ranges = Vec::new();
    let mut start = page_nums[0];
    let mut end = page_nums[0];

    for &page in &page_nums[1..] {
        if page == end + 1 {
            end = page;
        } else {
            if start == end {
                ranges.push(format!("{}", start));
            } else if end == start + 1 {
                ranges.push(format!("{},{}", start, end));
            } else {
                ranges.push(format!("{}-{}", start, end));
            }
            start = page;
            end = page;
        }
    }

    // Add the last range
    if start == end {
        ranges.push(format!("{}", start));
    } else if end == start + 1 {
        ranges.push(format!("{},{}", start, end));
    } else {
        ranges.push(format!("{}-{}", start, end));
    }

    format!("[{}]", ranges.join(","))
}

/// Wrap text to fit within a specified width
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let words: Vec<&str> = text.split_whitespace().collect();

    if words.is_empty() {
        return lines;
    }

    let mut current_line = String::new();
    for word in words {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + 1 + word.len() <= max_width {
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

    lines
}
