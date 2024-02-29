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
    /// Initialize Bib
    Init,
    /// Add new reference
    Add {
        /// Initial query for searching
        #[clap(value_name = "URL", default_value_t = String::from(""))]
        url: String,
    },
    /// Mutable bibliography search
    Search {
        /// Initial query for searching
        #[clap(value_name = "PROMPT", default_value_t = String::from(""))]
        query: String,
    },
    /// Immutable bliography search
    Peek,
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
        /// Yeet to remote
        #[clap(value_name = "REMOTE")]
        remote: Option<String>,

        /// Stack to yeet towards
        #[clap(value_name = "STACK")]
        stack: String,
    },
    /// Pull references from target stack
    Yank {
        /// If target is remote
        #[clap(value_name = "REMOTE")]
        remote: Option<String>,

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
        Commands::Yeet { remote, stack } => match commands::stack::yeet(remote, stack.clone()) {
            Ok(()) => println!("Yeeting into {} stack", stack),
            Err(err) => println!("BIB error: {}", err),
        },
        Commands::Yank { remote, stack } => match commands::stack::yank(remote, stack.clone()) {
            Ok(()) => println!("Yanking from {} stack", stack),
            Err(err) => println!("BIB error: {}", err),
        },
        Commands::Fork { stack } => match commands::stack::fork(stack.clone()) {
            Ok(oldname) => println!("Forking {} stack into as {}", oldname, stack),
            Err(err) => println!("BIB error: {}", err),
        },
        Commands::Search { query } => commands::search::search(query),
        Commands::Peek => commands::search::peek(),
        Commands::Export { out } => match out {
            Some(out) => println!("Printing current stack to: {},bib", out),
            None => println!("Printing current stack to stack.bib"),
        },
        Commands::Cleanup => println!("Cleanup on aisle 3"),
    }
}
