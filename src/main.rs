use clap::{Parser, Subcommand};
use commands::find::find;
use commands::scan::scan;
use commands::search::interactive_search;
use std::path::PathBuf;
use std::process;

use crate::base::PdfStorage;
use std::fs;
use thiserror::Error;
mod base;
mod bibtex;
mod commands;
mod gemini;
mod store;
use store::PaperStore;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Error Adding Paper: {0}")]
    AddError(#[from] commands::add::BibError),
    #[error("Store error: {0}")]
    Store(#[from] store::StoreError),
    #[error("Search error: {0}")]
    SearchError(#[from] commands::search::SearchError),
    #[error("Find error: {0}")]
    FindError(String),
    #[error("Semantic search error: {0}")]
    SemanticSearch(#[from] commands::find::SemanticSearchError),
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
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

    /// Search all papers using fuzzy matching
    Search {
        /// Maximum number of results to show
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,
    },

    /// Find papers based on a semantic query
    Find {
        /// Search query
        query: String,
        /// Maximum number of results to show
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,
        /// Maximum number of different papers to pass to LLM
        #[arg(short = 't', long, default_value = "0.7")]
        threshold: f32,
    },
    /// Deep scan of biblioogrpahy using RAG and llms for specific query
    Scan {
        /// Search query
        query: String,
        /// Maximum number of different papers to pass to LLM
        #[arg(short = 'n', long, default_value = "20")]
        limit: usize,
        /// Maximum number of different papers to pass to LLM
        #[arg(short = 't', long, default_value = "0.7")]
        threshold: f32,
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

    // Regular command mode
    let Some(command) = cli.command else {
        // No command provided, show help

        println!("No command provided. Use --help for usage information.");
        return;
    };

    let result = match command {
        Commands::Add { url, notes } => commands::add::add(url, notes, &mut store)
            .await
            .map_err(|e| AppError::AddError(e)),
        Commands::Search { limit } => {
            interactive_search(&mut store, limit).map_err(|e| AppError::SearchError(e))
        }
        Commands::Find {
            query,
            limit,
            threshold,
        } => find(&mut store, &query, limit, threshold)
            .await
            .map_err(|e| AppError::FindError(e.to_string())),
        Commands::Scan {
            query,
            limit,
            threshold,
        } => scan(&mut store, &query, limit, threshold)
            .await
            .map_err(|e| AppError::FindError(e.to_string())),
        Commands::Stats => show_stats(&store),
    };

    match result {
        Ok(()) => (),
        Err(err) => error_message(&err.to_string()),
    }
}

fn show_stats(store: &PaperStore) -> Result<(), AppError> {
    println!("\nDatabase Statistics:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Get total number of papers
    let count = store.count()?;
    println!("  Total papers: {}", count);

    // Get database file size
    let db_path = get_db_path();
    let db_size = fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);
    println!(
        "  Database size: {}",
        PdfStorage::format_file_size(db_size as usize)
    );

    // Get total PDF storage size
    let pdf_size = PdfStorage::total_storage_size().unwrap_or(0);
    println!(
        "  PDF storage: {}",
        PdfStorage::format_file_size(pdf_size as usize)
    );

    // Calculate total storage
    let total_size = db_size + pdf_size;
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(
        "  Total storage: {}",
        PdfStorage::format_file_size(total_size as usize)
    );

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
