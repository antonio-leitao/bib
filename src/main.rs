mod base;
mod commands;
mod parser;
mod settings;
mod utils;
use crate::utils::fmt;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize Bib
    Init,
    /// Add new reference
    Add {
        /// Initial query for searching
        #[clap(value_name = "URL", default_value_t = String::from(""))]
        url: String,
    },
    /// Open pdf manually
    Open {
        /// Initial query for searching
        #[clap(value_name = "PROMPT", default_value_t = String::from(""))]
        query: String,
    },
    /// Mutable bibliography search
    Search {
        /// Initial query for searching
        #[clap(value_name = "PROMPT", default_value_t = String::from(""))]
        query: String,
    },
    /// Immutable bliography search
    Peek,
    /// Print list
    List,
    /// Manage bib stacks
    Stack {
        /// Stack name
        #[clap(value_name = "NAME", default_value_t = String::new())]
        stack: String,
        /// Delete selected stack
        #[clap(
            short,
            long,
            requires = "stack",
            group = "from",
            default_value_t = false
        )]
        delete: bool,
        /// Rename current stack
        #[clap(
            short,
            long,
            requires = "stack",
            group = "from",
            default_value_t = false
        )]
        rename: bool,
    },
    /// Export bib file
    Export {
        /// Specify filename
        #[clap(value_name = "FILENAME")]
        out: Option<String>,
    },
    /// Switch into stack
    Checkout {
        /// Target stack name
        #[clap(value_name = "NAME")]
        stack: String,
        /// Create if not exist
        #[clap(short, long, default_value_t = false)]
        new: bool,
    },
    /// Push references into target stack
    Yeet {
        /// Stack to yeet towards
        #[clap(value_name = "STACK")]
        stack: String,
    },
    /// Pull references from target stack
    Yank {
        /// Target branch. Defaults to base.
        #[clap(value_name = "STACK")]
        stack: String,
    },
    /// Merge current stack with target stack
    Merge {
        /// Add new reference along with it
        #[clap(value_name = "STACK")]
        stack: String,
    },
    /// Fork current stack into a new stack
    Fork {
        /// New stack name
        #[clap(value_name = "NAME")]
        stack: String,
    },
    /// Clean up all notes and references, find pdfs etc
    Cleanup,
}
fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Init => commands::stack::init(),
        Commands::Stack {
            stack,
            delete,
            rename,
        } => commands::stack::stack(stack, delete, rename),
        Commands::Checkout { stack, new } => commands::stack::checkout(stack, new),
        Commands::Add { url } => commands::add::add(url),
        Commands::Open { query } => commands::open::open(query),
        Commands::Merge { stack } => commands::stack::merge(stack),
        Commands::Yeet { stack } => commands::stack::yeet(stack),
        Commands::Yank { stack } => commands::stack::yank(stack),
        Commands::Fork { stack } => commands::stack::fork(stack),
        Commands::Search { query } => commands::search::search(query),
        Commands::Peek => commands::search::peek(),
        Commands::List => commands::search::list(),
        Commands::Export { out } => commands::export::export(out),
        Commands::Cleanup => Ok(println!("Cleanup on aisle 3")),
    };
    match result {
        Ok(()) => (),
        Err(err) => fmt::erro(err.to_string()),
    }
}
