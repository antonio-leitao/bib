mod base;
mod commands;
mod parser;
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
        stack: String,
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
        /// Url to pdf
        #[clap(short, long)]
        url: Option<String>,
    },
    /// Mutable search
    Search {
        /// Initial query for searching
        #[clap(value_name = "PROMPT", default_value_t = String::from(""))]
        query: String,
    },
    /// Immutable search of most common
    Peek,
    /// Clean up all notes and references, find pdfs etc
    Cleanup,
    Debug {
        /// Url to pdf
        #[clap(short, long, group = "from")]
        url: String,
    },
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
        Commands::Add { url } => match commands::add::add(url) {
            Ok(()) => println!("Succesfully added reference"),
            Err(err) => println!("BIB error: {}", err),
        },
        Commands::Merge { stack } => match commands::stack::merge(stack.clone()) {
            Ok(()) => println!("Merging {} stack", stack),
            Err(err) => println!("BIB error: {}", err),
        },
        Commands::Yeet { stack } => match commands::stack::yeet(stack.clone()) {
            Ok(()) => println!("Yeeting into {} stack", stack),
            Err(err) => println!("BIB error: {}", err),
        },
        Commands::Yank { stack } => match stack {
            Some(stack) => println!("Yanking from {} stack", stack),
            None => println!("Yanking from base stack"),
        },
        Commands::Fork { stack } => match commands::stack::fork(stack.clone()) {
            Ok(oldname) => println!("Forking {} stack into as {}", oldname, stack),
            Err(err) => println!("BIB error: {}", err),
        },
        Commands::Search { query } => commands::search::search(query),
        Commands::Peek => commands::search::peek(),
        Commands::Export { notes, pdfs } => {
            println!("Exporting notes:{}, Exporting Pdfs:{}", notes, pdfs)
        }
        Commands::Cleanup => println!("Cleanup on aisle 3"),
        Commands::Debug { url } => match commands::debug::run(&url) {
            Ok(()) => (),
            Err(err) => println!("BIB error: {}", err),
        },
    }
}
