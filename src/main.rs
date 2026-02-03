mod add;
mod config;
mod query;
mod search;
mod ui;
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
mod database;
use config::Config;
use database::CitationDb;
mod embed;
use ui::StatusUI;

#[derive(Parser)]
#[command(name = "bib")]
#[command(about = "Search papers by how researchers cite them")]
#[command(
    long_about = "A citation knowledge base that finds papers based on how the research community describes them.\n\nWhen you add papers, bib extracts citation contexts—paragraphs where authors describe other work. When you query, you're searching through these descriptions, finding papers based on how researchers characterize them."
)]
#[command(after_help = "Examples:
  bib add https://arxiv.org/abs/2301.00001
  bib sync
  bib query \"attention mechanisms in vision\"
  bib search")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(
        about = "Add a paper to the knowledge base",
        long_about = "Add a paper to the knowledge base from various sources.\n\nSupported sources:\n  - arXiv URLs (downloads PDF automatically)\n  - Direct PDF URLs\n  - Local PDF file paths\n  - Clipboard (if no argument given, reads URL from clipboard)\n\nThe PDF is downloaded to your configured directory, parsed for text, and citation contexts are extracted and indexed for semantic search.",
        after_help = "Examples:
  bib add https://arxiv.org/abs/2301.00001
  bib add https://example.com/paper.pdf
  bib add ~/Downloads/paper.pdf
  bib add                                   # reads URL from clipboard"
    )]
    Add {
        /// URL (arXiv/PDF) or local file path. If omitted, reads from clipboard
        #[clap(value_name = "SOURCE")]
        url: Option<String>,
    },

    #[command(
        about = "Process all PDFs in configured directory",
        long_about = "Scan the configured PDF directory and process any unindexed papers.\n\nThis is useful for batch-importing papers you've manually added to the PDF directory. Already-processed files are skipped.",
        after_help = "Example:
  bib sync"
    )]
    Sync,

    #[command(
        about = "Interactive fuzzy search UI",
        long_about = "Launch an interactive terminal UI for browsing and opening papers.\n\nFeatures:\n  - Fuzzy search by title as you type\n  - Filter to show only local PDFs with --sources\n  - Open selected paper directly from the UI",
        after_help = "Examples:
  bib search
  bib search -n 50
  bib search --sources

Keybindings:
  Type        Filter papers by title
  Enter       Open selected paper
  ↑/↓         Navigate results
  Esc/q       Quit"
    )]
    Search {
        /// Maximum papers to retrieve
        #[arg(
            short = 'n',
            long,
            default_value = "10",
            help = "Number of papers to retrieve"
        )]
        limit: usize,

        /// Only show processed papers (local PDFs)
        #[arg(long)]
        sources: bool,
    },

    #[command(
        about = "Search by citation context",
        long_about = "Search papers using semantic similarity against citation contexts.\n\nThis searches through how researchers describe papers when citing them, not just titles or abstracts. Results are ranked by semantic similarity and optionally reranked using an LLM.",
        after_help = "Examples:
  bib query \"attention mechanisms in vision\"
  bib query \"efficient transformers\" -k 10
  bib query \"graph neural networks\" --report"
    )]
    Query {
        /// Search query string
        search_query: String,
        /// Number of results to return
        #[arg(short = 'k', long, default_value_t = 20)]
        top_k: usize,
        /// Generate a research report instead of ranked results
        #[arg(long)]
        report: bool,
    },

    #[command(
        about = "Show database statistics",
        long_about = "Display statistics about the citation database.\n\nShows:\n  - Database file location\n  - Configured PDF directory\n  - Number of indexed papers\n  - Number of extracted paragraphs\n  - Number of citation contexts",
        after_help = "Example:
  bib status"
    )]
    Status,

    #[command(
        about = "Configure PDF storage directory",
        long_about = "Set or change the directory where PDFs are stored.\n\nIf you already have papers indexed, they will be migrated to the new location.",
        after_help = "Example:
  bib config --pdf-dir ~/Papers"
    )]
    Config {
        #[arg(long)]
        pdf_dir: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Commands::Config { pdf_dir } = cli.command {
        Config::create(pdf_dir)?;
        return Ok(());
    }

    let cfg = Config::load()?;
    let mut db = CitationDb::open(Config::database_path()?)?;
    match cli.command {
        Commands::Add { url } => {
            add::run(&cfg, &mut db, url).await?;
        }
        Commands::Sync => {
            add::sync(&cfg, &mut db).await?;
        }
        Commands::Search { limit, sources } => {
            search::run(&cfg, &mut db, limit, sources).await?;
        }
        Commands::Query {
            search_query,
            top_k,
            report,
        } => {
            query::query(&db, &search_query, top_k, report).await?;
        }
        Commands::Status => {
            let stats = db.stats()?;
            let db_size = std::fs::metadata(&Config::database_path()?)
                .map(|m| m.len())
                .unwrap_or(0);
            StatusUI::info(&format!("Database: {}", Config::database_path()?.display()));
            StatusUI::info(&format!("PDF directory: {}", cfg.pdf_dir().display()));
            println!();
            StatusUI::info(&format!("Papers:      {:>8}", stats.paper_count));
            StatusUI::info(&format!("Paragraphs:  {:>8}", stats.paragraph_count));
            StatusUI::info(&format!("Citations:   {:>8}", stats.citation_count));
            StatusUI::info(&format!(
                "Size:        {:>8}",
                format_file_size(db_size as usize)
            ));
        }

        Commands::Config { .. } => unreachable!(),
    }

    Ok(())
}

// Format file size utility
fn format_file_size(bytes: usize) -> String {
    if bytes > 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes > 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} bytes", bytes)
    }
}
