mod base;
mod commands;
mod semantic;
mod utils;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

//Hall Whitehead, et al | 2023 | Evidence from sperm Whale language [PDF] [Note]

#[derive(Subcommand)]
enum Commands {
    /// Append a new node to draft
    Notes {
        /// Note content
        #[clap(value_name = "NOTE")]
        content: String,
        /// Add directly to library
        #[clap(short, long, default_value_t = false)]
        force: bool,
    },
    /// Archive notes in draft
    Push {
        /// Add new reference along with it
        #[clap(short, long, value_name = "BIBTEX")]
        reference: Option<String>,
    },
    /// Semantic Search over all notes
    Pull {
        /// Query to embed
        query: String,
        /// Size of search results.
        #[clap(short, long)]
        number: Option<usize>,
        /// Output file location. Prints to stdout if not specified.
        #[clap(short, long)]
        output: Option<String>,
        /// Output bibfile location. Won't output bib if not specified.
        #[clap(short, long)]
        bibfile: Option<String>,
    },
    /// Manage library of notes
    Peek {
        /// Retrieve only notes that cite a specific reference.
        #[clap(short, long, value_name = "REFERENCE")]
        reference: Option<String>,
    },
    /// Edit draft
    Ammend,
    //bibmanage
    /// Manually add a new reference
    Add {
        /// Reference to add
        #[clap(value_name = "REFERENCE")]
        reference: String,
        /// Bibtex file
        #[clap(short, long, group = "from", default_value_t = false)]
        doi: bool,
        /// Arxiv ID
        #[clap(short, long, group = "from", default_value_t = false)]
        arxiv: bool,
    },
    /// Mutable search
    Search {
        /// Initial query for searching
        #[clap(value_name = "PROMPT", default_value_t = String::from(""))]
        query: String,
        /// Search online instead of locally
        #[clap(short, long, default_value_t = false)]
        online: bool,
        /// Alter this file instead of main
        #[clap(short, long, default_value_t = false)]
        this: bool,
    },
    /// Fetch online for missing pdfs
    Fetch,
    /// Merge bibfile in working directory with bibliography
    Yeet,
    /// Create or (append to) bibfile from selected references
    Yank {
        /// Initial query for searching
        #[clap(short, long, value_name = "QUERY")]
        query: Option<String>,
    },
    /// Find similar references
    More {
        /// More like the bibfile in this directory
        #[clap(short, long, default_value_t = false)]
        this: bool,
    },
}
fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Notes { content, force } => println!("Adding note to draft"),
        Commands::Push { reference } => println!("pushing draft with reference"),
        Commands::Pull {
            query,
            number,
            output,
            bibfile,
        } => println!("Finfing notes that fit the query"),
        Commands::Peek { reference } => println!("printing and exploring library"),
        Commands::Ammend => println!("Opening draft and allowing editing"),
        Commands::Add {
            reference,
            doi,
            arxiv,
        } => println!("Adding new stuff to lib"),
        Commands::Search {
            query,
            online,
            this,
        } => commands::search::search(query, online, this),
        Commands::Fetch => println!("Fetching Papers"),
        Commands::Yeet => println!("Merging bibifile to library"),
        Commands::Yank { query } => {
            // this is just search + send to current dir
            println!("Searching and selecting references to create bibfile")
        }
        Commands::More { this } => println!("Finding more relevant papers"),
    }
}
