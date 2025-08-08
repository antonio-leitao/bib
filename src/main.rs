use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;

mod base;
mod bibtex;
mod commands;
mod completion;
mod fuzzy;
mod gemini;
mod store;

use completion::{CompletionContext, CompletionHandler};
use fuzzy::{FuzzyConfig, FuzzySearcher, SearchableItem};
use store::PaperStore;

#[macro_export]
macro_rules! blog {
    ($category:expr, $($arg:tt)*) => {{
        use termion::color;
        let formatted_args = format!($($arg)*);
        println!("{}{:>12}{} {}",color::Fg(color::Green), $category,color::Fg(color::Reset), formatted_args);
    }};
}

#[macro_export]
macro_rules! blog_working {
    ($category:expr, $($arg:tt)*) => {{
        use termion::color;
        let formatted_args = format!($($arg)*);
        println!("{}{:>12}{} {}",color::Fg(color::Blue), $category,color::Fg(color::Reset), formatted_args);
    }};
}

#[macro_export]
macro_rules! blog_done {
    ($category:expr, $($arg:tt)*) => {{
        use termion::color;
        let formatted_args = format!($($arg)*);
        println!("{}{:>12}{} {}",color::Fg(color::Green), $category,color::Fg(color::Reset), formatted_args);
    }};
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Shell completion mode (hidden from help)
    #[arg(long, hide = true)]
    complete: Option<String>,

    /// Completion context (hidden from help)
    #[arg(long, hide = true)]
    complete_context: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Add new reference
    Add {
        /// Initial query for searching
        #[clap(value_name = "URL")]
        url: Option<String>,

        /// Optional comments and observations
        #[arg(value_name = "NOTES", short, long)]
        notes: Option<String>,
    },

    /// Search papers using fuzzy matching
    Search {
        /// Search query
        query: String,

        /// Search by author instead of title
        #[arg(short = 'a', long)]
        author: bool,

        /// Maximum number of results to show
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,
    },

    /// List all papers in the database
    List {
        /// Maximum number of papers to display
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },

    /// Show database statistics
    Stats,
}

/// Get the default database path
fn get_db_path() -> PathBuf {
    let mut db_path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    db_path.push(".bib");
    std::fs::create_dir_all(&db_path).ok();
    db_path.push("papers.db");
    db_path
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Initialize the database for normal operations
    let db_path = get_db_path();

    let mut store = match PaperStore::new(&db_path) {
        Ok(store) => store,
        Err(e) => {
            error_message(&format!(
                "Failed to initialize database at {}: {}",
                db_path.display(),
                e
            ));
            process::exit(1);
        }
    };

    // Handle completion mode FIRST, before any other logic
    if let Some(query) = cli.complete {
        handle_completion(&store, &query, cli.complete_context);
        return;
    }

    // Regular command mode
    let Some(command) = cli.command else {
        // No command provided, show help
        println!("No command provided. Use --help for usage information.");
        return;
    };

    let result = match command {
        Commands::Add { url, notes } => commands::add::add(url, notes, &mut store)
            .await
            .map_err(|e| e.to_string()),

        Commands::Search {
            query,
            author,
            limit,
        } => fuzzy_search_papers(&store, &query, author, limit),

        Commands::List { limit } => list_papers(&store, limit),

        Commands::Stats => show_stats(&store),
    };

    match result {
        Ok(()) => (),
        Err(err) => error_message(&err),
    }
}

/// Handle shell completion requests
fn handle_completion(store: &PaperStore, query: &str, context: Option<String>) {
    let context = context.as_deref().unwrap_or("search-title");

    let completion_context = match context {
        "search-title" => CompletionContext::SearchTitle,
        "search-author" => CompletionContext::SearchAuthor,
        "key" => CompletionContext::PaperKey,
        "year" => CompletionContext::Year,
        _ => CompletionContext::SearchTitle,
    };

    let handler = CompletionHandler::new(store);

    match handler.complete(completion_context, query) {
        Ok(completions) => {
            // Output in zsh format by default
            completion::output_completions_zsh(completions);
        }
        Err(_) => {
            // Output nothing on error - this prevents shell from falling back to files
            // This is the correct behavior for completion scripts
        }
    }
}

/// Fuzzy search papers by title or author
fn fuzzy_search_papers(
    store: &PaperStore,
    query: &str,
    by_author: bool,
    limit: usize,
) -> Result<(), String> {
    let papers = store
        .list_all(None)
        .map_err(|e| format!("Failed to load papers: {}", e))?;

    if papers.is_empty() {
        println!("No papers found in the database.");
        return Ok(());
    }

    // Create searchable items based on search type
    let items: Vec<SearchableItem> = if by_author {
        // When searching by author, make author primary and title secondary
        papers
            .into_iter()
            .map(|paper| SearchableItem {
                primary: paper.author.clone(),
                secondary: Some(paper.title.clone()),
                context: Some(paper.year.to_string()),
                id: paper.id.to_string(),
                display: format!("{} | {}", paper.author, paper.title),
            })
            .collect()
    } else {
        // When searching by title, make title primary and author secondary
        papers
            .into_iter()
            .map(|paper| SearchableItem {
                primary: paper.title.clone(),
                secondary: Some(paper.author.clone()),
                context: Some(format!("{} - {}", paper.author, paper.year)),
                id: paper.id.to_string(),
                display: paper.title.clone(),
            })
            .collect()
    };

    // Configure and perform fuzzy search
    let config = FuzzyConfig {
        max_results: limit,
        search_secondary: true,
        ..Default::default()
    };

    let searcher = FuzzySearcher::new(config);
    let results = searcher.search(query, items);

    if results.is_empty() {
        println!("No papers found matching '{}'", query);
        return Ok(());
    }

    // Display results
    println!("\nFound {} paper(s) matching '{}':\n", results.len(), query);

    // Reload papers to display full info
    let papers = store
        .list_all(None)
        .map_err(|e| format!("Failed to load papers: {}", e))?;

    for result in results {
        // Find the original paper by ID
        if let Some(paper) = papers.iter().find(|p| p.id.to_string() == result.item.id) {
            println!("{}", paper.display());

            // Show match quality for debugging (can remove later)
            if result.score > 0 {
                println!(
                    "  {} Match score: {}",
                    termion::color::Fg(termion::color::Rgb(100, 100, 100)),
                    result.score
                );
                println!("{}", termion::color::Fg(termion::color::Reset));
            }
        }
    }

    Ok(())
}

fn list_papers(store: &PaperStore, limit: usize) -> Result<(), String> {
    let papers = store
        .list_all(Some(limit))
        .map_err(|e| format!("Failed to list papers: {}", e))?;

    if papers.is_empty() {
        println!("No papers found in the database.");
        return Ok(());
    }

    println!("\nPapers in database (showing up to {}):\n", limit);
    for paper in papers {
        println!("{}", paper.display());
    }

    Ok(())
}

fn show_stats(store: &PaperStore) -> Result<(), String> {
    let count = store
        .count()
        .map_err(|e| format!("Failed to get count: {}", e))?;

    println!("\nDatabase Statistics:");
    println!("  Total papers: {}", count);

    if count > 0 {
        let papers = store
            .list_all(None)
            .map_err(|e| format!("Failed to get papers: {}", e))?;

        if let (Some(min), Some(max)) =
            papers
                .iter()
                .map(|p| p.year)
                .fold((None, None), |(min, max), year| {
                    (
                        Some(min.map_or(year, |m: i64| m.min(year))),
                        Some(max.map_or(year, |m: i64| m.max(year))),
                    )
                })
        {
            println!("  Year range: {} - {}", min, max);
        }

        let mut year_counts = std::collections::HashMap::new();
        for paper in &papers {
            *year_counts.entry(paper.year).or_insert(0) += 1;
        }

        println!("\n  Papers by year:");
        let mut years: Vec<_> = year_counts.keys().collect();
        years.sort_by(|a, b| b.cmp(a));

        for year in years.iter().take(10) {
            let count = year_counts[*year];
            println!("    {}: {} paper(s)", year, count);
        }
    }

    Ok(())
}

fn error_message(err: &str) {
    println!(
        "{}{:>12}{} {}",
        termion::color::Fg(termion::color::Red),
        "Error",
        termion::color::Fg(termion::color::Reset),
        err
    );
}
