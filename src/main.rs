use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;

mod base;
mod bibtex;
mod commands;
mod gemini;
mod store;

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
    #[command(subcommand)]
    command: Commands,
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

    /// List all papers in the database
    List {
        /// Maximum number of papers to display
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },

    /// Search papers by title
    Search {
        /// Search query
        query: String,

        /// Search in title (default)
        #[arg(short = 't', long, conflicts_with = "author")]
        title: bool,

        /// Search by author
        #[arg(short = 'a', long)]
        author: bool,
    },

    /// Show database statistics
    Stats,
}

/// Get the default database path
fn get_db_path() -> PathBuf {
    let mut db_path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    db_path.push(".papers");
    std::fs::create_dir_all(&db_path).ok(); // Create directory if it doesn't exist
    db_path.push("papers.db");
    db_path
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Initialize the database
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

    blog!("Database", "Using {}", db_path.display());

    let result = match cli.command {
        Commands::Add { url, notes } => commands::add::add(url, notes, &mut store)
            .await
            .map_err(|e| e.to_string()),

        Commands::List { limit } => list_papers(&store, limit),

        Commands::Search {
            query,
            title: _,
            author,
        } => search_papers(&store, &query, author),

        Commands::Stats => show_stats(&store),
    };

    match result {
        Ok(()) => (),
        Err(err) => error_message(&err),
    }
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

fn search_papers(store: &PaperStore, query: &str, by_author: bool) -> Result<(), String> {
    let papers = if by_author {
        store.search_by_author(query)
    } else {
        store.search_by_title(query)
    }
    .map_err(|e| format!("Search failed: {}", e))?;

    if papers.is_empty() {
        println!("No papers found matching '{}'", query);
        return Ok(());
    }

    println!("\nFound {} paper(s):\n", papers.len());
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
        // Get year range
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

        // Count papers by year
        let mut year_counts = std::collections::HashMap::new();
        for paper in &papers {
            *year_counts.entry(paper.year).or_insert(0) += 1;
        }

        println!("\n  Papers by year:");
        let mut years: Vec<_> = year_counts.keys().collect();
        years.sort_by(|a, b| b.cmp(a)); // Sort descending

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
