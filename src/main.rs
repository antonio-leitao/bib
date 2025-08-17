use clap::{Parser, Subcommand};
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
use ui::StatusUI;

#[derive(Parser)]
#[command(
    author = "Antonio Leitao",
    version,
    about = "A research paper manager with AI-powered extraction and semantic search",
    long_about = "bib - Intelligent bibliography management from the command line\n\n\
                  Manage your research papers with automatic BibTeX extraction, \n\
                  semantic search capabilities, and deep content analysis.\n\n\
                  Quick start:\n  \
                  bib add https://arxiv.org/abs/2301.00001  # Add paper from URL\n  \
                  bib                                        # Interactive search\n  \
                  bib find \"transformer architectures\"       # Semantic search"
)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(
        about = "Add a new paper to your bibliography",
        long_about = "Add a new paper to your bibliography",
        after_help = "Examples:\n  \
                     bib add https://arxiv.org/abs/2301.00001\n  \
                     bib add paper.pdf\n  \
                     bib add  # Uses URL/path from clipboard\n  \
                     bib add https://arxiv.org/abs/2301.00001 -n \"Key paper for chapter 3\""
    )]
    Add {
        /// URL (arXiv/PDF) or local file path. If omitted, reads from clipboard
        #[clap(value_name = "SOURCE")]
        url: Option<String>,

        /// Add notes to the paper entry
        #[arg(
            value_name = "NOTES",
            short,
            long,
            help = "Personal notes about the paper"
        )]
        notes: Option<String>,
    },

    #[command(
        about = "Find papers using semantic search",
        long_about = "Find papers using semantic search\n\n\
                     Uses vector embeddings to find conceptually similar papers\n\
                     even when they don't share exact keywords.",
        after_help = "Examples:\n  \
                     bib find \"transformer architectures in computer vision\"\n  \
                     bib find \"applications of topological data analysis\" -n 5\n  \
                     bib find \"deep learning for proteins\" -t 0.8"
    )]
    Search {
        /// Natural language query describing what you're looking for
        #[clap(value_name = "QUERY")]
        query: Option<String>,

        /// Maximum papers to retrieve
        #[arg(
            short = 'n',
            long,
            default_value = "10",
            help = "Number of papers to retrieve"
        )]
        limit: usize,

        /// Similarity threshold (0.0-1.0, higher = stricter matching)
        #[arg(
            short = 't',
            long,
            default_value = "0.5",
            help = "Minimum similarity score"
        )]
        threshold: f32,
    },

    #[command(
        about = "Deep content analysis of papers matching your query",
        long_about = "Deep content analysis of papers matching your query\n\n\
                     Performs comprehensive analysis of paper contents to find\n\
                     specific information, methodologies, or results.",
        after_help = "Examples:\n  \
                     bib find \"experimental results on MNIST dataset\"\n  \
                     bib find \"papers comparing BERT vs GPT architectures\" -n 15\n  \
                     bib find \"statistical methods for time series\" -t 0.75\n\n\
                     Note: This command may take longer as it analyzes full paper contents"
    )]
    Find {
        /// Research question or topic to investigate
        #[clap(value_name = "QUERY")]
        query: String,

        /// Maximum papers to analyze
        #[arg(
            short = 'n',
            long,
            default_value = "20",
            help = "Number of papers to analyze"
        )]
        limit: usize,

        /// Similarity threshold for initial filtering
        #[arg(
            short = 't',
            long,
            default_value = "0.5",
            help = "Minimum similarity for inclusion"
        )]
        threshold: f32,
    },

    #[command(
        about = "Show database statistics",
        long_about = "Show database statistics\n\n\
                     Displays total papers, storage usage, and database metrics"
    )]
    Stats,
}

fn get_db_path() -> PathBuf {
    let mut db_path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    db_path.push(".bib");
    std::fs::create_dir_all(&db_path).ok();
    db_path.push("papers.db");
    db_path
}

async fn run_app() -> Result<(), AppError> {
    let cli = Cli::parse();
    let db_path = get_db_path();

    let mut store = PaperStore::new(&db_path)?;

    let Some(command) = cli.command else {
        // If no command is given, run the default search
        commands::search::execute(&mut store, None, 10, 0.5).await?;
        return Ok(());
    };

    match command {
        Commands::Add { url, notes } => commands::add::execute(url, notes, &mut store).await?,
        Commands::Search {
            query,
            limit,
            threshold,
        } => commands::search::execute(&mut store, query, limit, threshold).await?,
        Commands::Find {
            query,
            limit,
            threshold,
        } => commands::find::execute(&mut store, &query, limit, threshold).await?,
        Commands::Stats => show_stats(&store)?,
    };

    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = run_app().await {
        StatusUI::render_error(e);
        process::exit(1);
    }
}

fn show_stats(store: &PaperStore) -> Result<(), AppError> {
    use termion::color;

    // Get all the stats
    let count = store.count()?;
    let db_path = get_db_path();
    let db_size = std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);
    let pdf_size = PdfStorage::total_storage_size()?;
    let total_size = db_size + pdf_size;

    // Calculate some additional useful stats
    let avg_size_per_paper = if count > 0 {
        total_size / count as u64
    } else {
        0
    };

    println!();

    // Header in the same style as search/find commands
    println!(
        "{}{}[DATABASE STATISTICS]{}",
        termion::clear::CurrentLine,
        color::Fg(color::Rgb(83, 110, 122)),
        color::Fg(color::Reset)
    );
    println!();

    // Papers count with emphasis
    println!(
        "   {} Papers in library: {}{}{}",
        StatusUI::INFO,
        color::Fg(color::Rgb(83, 110, 122)),
        count,
        color::Fg(color::Reset)
    );

    // Storage breakdown with subtle coloring
    println!(
        "   {} Database: {}{:<12}{} PDFs: {}{}{}",
        StatusUI::INFO,
        color::Fg(color::Rgb(83, 110, 122)),
        StatusUI::format_file_size(db_size as usize),
        color::Fg(color::Reset),
        color::Fg(color::Rgb(83, 110, 122)),
        StatusUI::format_file_size(pdf_size as usize),
        color::Fg(color::Reset)
    );
    // Total with success indicator
    println!(
        "   {} Total storage: {}{}{}",
        StatusUI::INFO,
        color::Fg(color::Rgb(83, 110, 122)),
        StatusUI::format_file_size(total_size as usize),
        color::Fg(color::Reset)
    );
    // Average size if meaningful
    if count > 0 {
        println!(
            "   {} Average per paper: {}{}{}",
            StatusUI::INFO,
            color::Fg(color::Rgb(83, 110, 122)),
            StatusUI::format_file_size(avg_size_per_paper as usize),
            color::Fg(color::Reset)
        );
    }
    // Storage location info
    println!(
        "   {} Location: {}{}{}",
        StatusUI::INFO,
        color::Fg(color::Rgb(83, 110, 122)),
        db_path.parent().unwrap().display(),
        color::Fg(color::Reset)
    );

    println!();

    Ok(())
}
