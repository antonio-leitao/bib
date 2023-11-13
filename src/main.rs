mod base;
mod commands;
mod semantic;
mod settings;
mod utils;
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
    /// Export notes
    Export {
        /// Wether to also export notes
        #[clap(short, long, default_value_t = false)]
        notes: bool,
        /// Wheter to also export pdfs
        #[clap(short, long, default_value_t = false)]
        pdfs: bool,
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
    /// Merge changes from target stack
    Merge {
        /// Add new reference along with it
        #[clap(value_name = "STACK")]
        stack: String,
    },
    /// Push changes towards target stack
    Yeet {
        /// Add new reference along with it. Defaults to base.
        #[clap(value_name = "STACK")]
        stack: Option<String>,
    },
    /// Bring references from target stack
    Yank {
        /// Target branch. Defaults to base.
        #[clap(value_name = "STACK")]
        stack: Option<String>,
    },
    /// Fork current stack into new stack
    Fork {
        /// New stack name
        #[clap(value_name = "NAME")]
        stack: String,
    },
    /// Manually add new reference
    Add {
        /// Arxiv
        #[clap(short, long, group = "from")]
        arxiv: Option<String>,
        /// Url to pdf
        #[clap(short, long, group = "from")]
        url: Option<String>,
        /// Path to Pdf in this computer
        #[clap(short, long, group = "from")]
        path: Option<String>,
    },
    /// Mutable search
    Search {
        /// Initial query for searching
        #[clap(value_name = "PROMPT", default_value_t = String::from(""))]
        query: String,
        /// Search online instead of locally
        #[clap(short, long, default_value_t = false)]
        online: bool,
    },
    /// Immutable search of most common
    Peek,
    /// Clean up all notes and references, find pdfs etc
    Cleanup,
    /// Initialize Bib
    Init,
}
fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init => match commands::stack::init() {
            Ok(()) => println!("BIB intialized"),
            Err(err) => println!("BIB error: {}", err),
        },
        Commands::Stack {
            stack,
            delete,
            rename,
        } => commands::stack::stack(stack, delete, rename),
        Commands::Checkout { stack, new } => match commands::stack::checkout(stack.clone(), new) {
            Ok(()) => println!("Switched to {} stack", stack),
            Err(err) => println!("BIB error: {}", err),
        },
        Commands::Add { arxiv, url, path } => match commands::add::add(arxiv, url, path) {
            Ok(()) => println!("Succesfully added reference"),
            Err(err) => println!("BIB error: {}", err),
        },
        Commands::Merge { stack } => println!("Merging {} stack", stack),
        Commands::Yeet { stack } => match stack {
            Some(stack) => println!("Yeeting into {} stack", stack),
            None => println!("Yeeting into base base"),
        },
        Commands::Yank { stack } => match stack {
            Some(stack) => println!("Yanking from {} stack", stack),
            None => println!("Yanking from base stack"),
        },
        Commands::Fork { stack } => println!("Forking current stack under new name {}", stack),
        Commands::Search { query, online } => commands::search::search(query, online),
        Commands::Peek => println!("Displaying recently viewd papers/notes"),
        Commands::Export { notes, pdfs } => {
            println!("Exporting notes:{}, Exporting Pdfs:{}", notes, pdfs)
        }
        Commands::Cleanup => println!("Cleanup on aisle 3"),
    }
}
