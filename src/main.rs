use clap::{Parser, Subcommand};
use commands::prompt::interactive_search;
use std::path::PathBuf;
use std::process;

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
    SearchError(#[from] commands::prompt::SearchError),
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
    Prompt {
        /// Search query
        query: String,
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
            interactive_search(&store, limit).map_err(|e| AppError::SearchError(e))
        }
        Commands::Prompt { query } => {
            println!("proompting");
            Ok(())
        }
        Commands::Stats => show_stats(&store),
    };

    match result {
        Ok(()) => (),
        Err(err) => error_message(&err.to_string()),
    }
}

fn show_stats(store: &PaperStore) -> Result<(), AppError> {
    let count = store.count()?;
    println!("\nDatabase Statistics:");
    println!("  Total papers: {}", count);

    if count > 0 {
        let papers = store.list_all(None)?;

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
