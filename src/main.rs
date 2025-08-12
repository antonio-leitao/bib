use clap::{Parser, Subcommand};
use commands::base::interactive_search;
use commands::find::find;
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
    SearchError(#[from] commands::base::SearchError),
    #[error("Find error: {0}")]
    FindError(String),
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

    /// Search papers using fuzzy matching
    Search {
        /// Maximum number of results to show
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,
    },

    /// Interactive search papers using fuzzy matching
    Find {
        /// Search query
        query: String,
        /// Maximum number of different papers to pass to LLM
        #[arg(short = 'n', long, default_value = "20")]
        limit: usize,
    },
    /// Show database statistics
    Stats,
}

/// Get the default database path
fn get_db_path() -> PathBuf {
    let mut db_path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    db_path.push(".papers");
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
        Commands::Find { query, limit } => find(&mut store, &query, limit)
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
