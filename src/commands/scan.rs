use crate::base::{Embedding, Paper, PdfStorage, UI};
use crate::gemini::{Gemini, PaperAnalysis};
use crate::store::PaperStore;
use crate::{blog_done, blog_warning};
use anyhow::Result;
use dotzilla;
use futures;
use indicatif::{ProgressBar, ProgressStyle};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use termion;
use termion::color;
use tokio::fs;
use tokio::sync::{Mutex, Semaphore};
use tokio::time::sleep;

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

    // Partition so that top k elements are at the beginning
    scores.select_nth_unstable_by(k - 1, |a, b| {
        b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal)
    });

    // Sort only the top k elements
    scores[..k].sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
    scores.truncate(k);

    scores
        .into_iter()
        .filter(|(_, sim)| *sim >= threshold)
        .collect()
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
    semaphore: Arc<Semaphore>,
) -> Result<AnalysisResult, AnalysisError> {
    // Acquire semaphore permit to control concurrency
    let _permit = semaphore.acquire().await.map_err(|e| AnalysisError {
        paper_id: paper.id,
        error: format!("Failed to acquire semaphore: {}", e),
    })?;

    let paper_id = paper.id;
    let paper_title = paper.title.clone();

    // Try to analyze the paper, converting any error to AnalysisError
    let result = async {
        // Read PDF file
        let pdf_path = PdfStorage::get_pdf_path(&paper.key, paper.id);
        let file_bytes = fs::read(&pdf_path)
            .await
            .map_err(|e| format!("Failed to read PDF: {}", e))?;

        // Create a new Gemini instance for this paper (since upload_file takes &mut self)
        let mut paper_ai =
            Gemini::new().map_err(|e| format!("Failed to create Gemini client: {}", e))?;

        // Upload file to Gemini
        let file_handle = paper_ai
            .upload_file(file_bytes, "application/pdf")
            .await
            .map_err(|e| format!("Failed to upload PDF: {}", e))?;

        // Analyze the paper
        let analysis = paper_ai
            .analyze_research_paper(&query, &file_handle)
            .await
            .map_err(|e| format!("Analysis failed: {}", e))?;

        Ok::<PaperAnalysis, String>(analysis)
    }
    .await;

    // Update progress bar regardless of success/failure
    let mut progress_guard = progress.lock().await;
    progress_guard.1 += 1; // Increment completed count
    let completed = progress_guard.1;
    let _total = progress_guard.2;

    match result {
        Ok(analysis) => {
            // Update progress bar with success
            progress_guard.0.set_position(completed as u64);
            progress_guard.0.set_message(format!(
                "analyzed: {}",
                fit_string_to_length(&paper_title, 40)
            ));

            Ok(AnalysisResult { paper_id, analysis })
        }
        Err(error) => {
            // Update progress bar with error indication
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

pub async fn scan(store: &mut PaperStore, query: &str, limit: usize, threshold: f32) -> Result<()> {
    let spinner = UI::spinner("Generating", "Query embedding...");

    let ai = Gemini::new()?;
    let query_vector = ai.generate_query_embedding(query).await?;
    UI::finish_with_message(spinner, "Generated", "query embedding.");

    let spinner = UI::spinner("Searching", "data for relevant papers...");
    let vectors = store.load_all_embeddings()?;
    let relevant_papers = similarity_threshold_filter(vectors, &query_vector, limit, threshold);
    UI::finish_with_message(
        spinner,
        "Found",
        &format!("{} relevant papers.", relevant_papers.len()),
    );

    let id_list: Vec<u128> = relevant_papers.into_iter().map(|(id, _)| id).collect();
    let papers = store.get_by_ids(&id_list)?;

    // Limit to 8 papers for testing (can be removed or made configurable)
    let papers_to_analyze: Vec<_> = papers.iter().take(8).cloned().collect();
    let total_papers = papers_to_analyze.len();

    if total_papers == 0 {
        blog_warning!("No papers", "to analyze");
        return Ok(());
    }

    // Configure rate limiting based on number of papers
    // Gemini free tier: ~60 requests/minute, each paper needs 2 requests (upload + analyze)
    let (max_concurrent, stagger_delay_ms) = match total_papers {
        1..=3 => (2, 1000),  // Few papers: 2 concurrent, 1 sec delay
        4..=8 => (2, 2000),  // Medium: 2 concurrent, 2 sec delay
        9..=15 => (3, 3000), // Many: 3 concurrent, 3 sec delay
        _ => (3, 4000),      // Lots: 3 concurrent, 4 sec delay
    };

    // Create semaphore to limit concurrent API calls
    let semaphore = Arc::new(Semaphore::new(max_concurrent));

    blog_done!(
        "Rate limit",
        "Max {} concurrent, {}ms delay between starts",
        max_concurrent,
        stagger_delay_ms
    );

    // Create progress bar for analysis
    let pb = ProgressBar::new(total_papers as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{prefix:.blue.bold} [{bar:30}] {pos}/{len} papers ({msg})")
            .expect("Invalid progress template")
            .progress_chars("=> "),
    );
    pb.set_prefix(format!("{:>12}", "Analyzing"));
    pb.set_message("starting...");

    // Shared progress state: (progress_bar, completed_count, total_count)
    let progress = Arc::new(Mutex::new((pb.clone(), 0usize, total_papers)));

    // Create futures for all paper analyses with staggered starts
    let query_string = query.to_string();
    let analysis_futures: Vec<_> = papers_to_analyze
        .into_iter()
        .enumerate()
        .map(|(index, paper)| {
            let query_clone = query_string.clone();
            let progress_clone = Arc::clone(&progress);
            let semaphore_clone = Arc::clone(&semaphore);

            // Stagger the start of each task
            let delay = (index as u64) * stagger_delay_ms;

            tokio::spawn(async move {
                // Initial stagger delay
                if delay > 0 {
                    sleep(Duration::from_millis(delay)).await;
                }
                analyze_single_paper(paper, query_clone, progress_clone, semaphore_clone).await
            })
        })
        .collect();

    // Wait for all analyses to complete
    let results = futures::future::join_all(analysis_futures).await;

    // Finish progress bar
    pb.finish_and_clear();

    // Separate successful and failed analyses
    let mut successful_analyses = Vec::new();
    let mut failed_analyses = Vec::new();

    for result in results {
        match result {
            Ok(Ok(analysis)) => successful_analyses.push(analysis),
            Ok(Err(error)) => failed_analyses.push(error),
            Err(join_error) => {
                blog_warning!("Task panic", "{}", join_error);
            }
        }
    }

    // Report results
    blog_done!(
        "Analyzed",
        "{}/{} papers successfully",
        successful_analyses.len(),
        total_papers
    );

    // Sort successful analyses by score (higher is better) and filter out score 0
    let mut successful_analyses: Vec<_> = successful_analyses
        .into_iter()
        .filter(|result| result.analysis.score > 0.2)
        .collect();
    successful_analyses.sort_by(|a, b| {
        b.analysis
            .score
            .partial_cmp(&a.analysis.score)
            .unwrap_or(Ordering::Equal)
    });

    // Display successful analyses
    if !successful_analyses.is_empty() {
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
                print!("{}", paper.display(available_width));
                println!(
                    " {}{}{}",
                    color::Fg(color::Yellow),
                    page_display,
                    color::Fg(color::Reset)
                );

                // Display score and explanation with indentation
                println!(
                    "      Score: {}{:.1}/10{}",
                    color::Fg(color::Red),
                    result.analysis.score,
                    color::Fg(color::Reset)
                );

                // Wrap and indent the explanation
                let explanation_width = (width as usize).saturating_sub(6); // 6 spaces for indentation
                let wrapped_explanation =
                    wrap_text(&result.analysis.explanation, explanation_width);
                for line in wrapped_explanation {
                    println!(
                        "    {}{}{}",
                        color::Fg(color::Rgb(83, 110, 122)),
                        line,
                        color::Fg(color::Reset)
                    );
                }
            }
        }
    } else {
        println!("\n❌ No relevant papers found with score > 0");
    }

    // Report any errors
    if !failed_analyses.is_empty() {
        println!("{}", "=".repeat(80));
        println!("⚠️  FAILED ANALYSES");
        println!("{}", "=".repeat(80));

        for error in failed_analyses {
            blog_warning!("Failed", "Paper ID {}: {}", error.paper_id, error.error);
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
