use clap::{Parser, Subcommand};
// use std::fmt::Result;
use std::path::PathBuf;
use std::process;

mod ai;
mod bibtex;
mod commands;
mod core;
mod error;
mod pdf;
mod storage;
mod ui;

use error::AppError;
use pdf::PdfStorage;
use storage::PaperStore;

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
        #[clap(value_name = "URL")]
        url: Option<String>,
        #[arg(value_name = "NOTES", short, long)]
        notes: Option<String>,
    },
    /// Search all papers using fuzzy matching
    Search {
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,
    },
    /// Find papers based on a semantic query
    Find {
        query: String,
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,
        #[arg(short = 't', long, default_value = "0.7")]
        threshold: f32,
    },
    /// Deep scan of bibliography using RAG
    Scan {
        query: String,
        #[arg(short = 'n', long, default_value = "20")]
        limit: usize,
        #[arg(short = 't', long, default_value = "0.7")]
        threshold: f32,
    },
    /// Show database statistics
    Stats,
}

fn get_db_path() -> PathBuf {
    let mut db_path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    db_path.push(".bib");
    std::fs::create_dir_all(&db_path).ok();
    db_path.push("papers.db");
    db_path
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let cli = Cli::parse();
    let db_path = get_db_path();

    let mut store = match PaperStore::new(&db_path) {
        Ok(store) => store,
        Err(e) => {
            ui::error_message(&format!(
                "Failed to initialize database at {}: {}",
                db_path.display(),
                e
            ));
            process::exit(1);
        }
    };

    let Some(command) = cli.command else {
        commands::search::execute(&mut store, 10)?;
        return Ok(());
    };

    match command {
        Commands::Add { url, notes } => commands::add::execute(url, notes, &mut store).await?,
        Commands::Search { limit } => commands::search::execute(&mut store, limit)?,
        Commands::Find {
            query,
            limit,
            threshold,
        } => commands::find::execute(&mut store, &query, limit, threshold).await?,
        Commands::Scan {
            query,
            limit,
            threshold,
        } => commands::scan::execute(&mut store, &query, limit, threshold).await?,
        Commands::Stats => show_stats(&store)?,
    };
    Ok(())

    // if let Err(err) = result {
    //     ui::error_message(&err.to_string());
    //     // process::exit(1);
    // }
}

fn show_stats(store: &PaperStore) -> Result<(), AppError> {
    println!("\nDatabase Statistics:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let count = store.count()?;
    println!("  Total papers: {}", count);

    let db_path = get_db_path();
    let db_size = std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);
    println!(
        "  Database size: {}",
        PdfStorage::format_file_size(db_size as usize)
    );

    let pdf_size = PdfStorage::total_storage_size()?;
    println!(
        "  PDF storage: {}",
        PdfStorage::format_file_size(pdf_size as usize)
    );

    let total_size = db_size + pdf_size;
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(
        "  Total storage: {}",
        PdfStorage::format_file_size(total_size as usize)
    );

    Ok(())
}
