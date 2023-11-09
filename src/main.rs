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
        /// Name of stack
        #[clap(value_name = "NAME")]
        stack: Option<String>,
        /// Delete selected stack
        #[clap(short, long, group = "from", default_value_t = false)]
        delete: bool,
        /// Rename selected stack
        #[clap(short, long, group = "from", default_value_t = false)]
        rename: bool,
    },
    /// Switch into stack
    Checkout {
        /// Target stack name
        #[clap(value_name = "NAME")]
        stack: String,
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
    /// Fork current stack into new stack
    Fork {
        /// New stack name
        #[clap(value_name = "NAME")]
        stack: String,
    },
    /// Manually add new reference
    Add {
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
    /// Clean up all notes and bibligraphies of haning references and pointers
    Cleanup,
}
fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Stack {
            stack,
            delete,
            rename,
        } => match (stack, delete, rename) {
            (None, _, _) => println!("  base\n* toread\n  phd"),
            (Some(stack), true, _) => println!("deleting stack: {}", stack),
            (Some(stack), _, true) => println!("renaming current stack to: {}", stack),
            (_, _, _) => println!("Something's wrong in here"),
        },
        Commands::Checkout { stack } => println!("Switching into {}", stack),
        Commands::Merge { stack } => println!("Merging stack {}", stack),
        Commands::Yeet { stack } => match stack {
            Some(stack) => println!("Yeeting current stack into {}", stack),
            None => println!("Yeeting current stack into base"),
        },
        Commands::Fork { stack } => println!("Forking current stack under new name {}", stack),
        Commands::Add { url, path } => println!("Adding new stuff to lib"),
        Commands::Search { query, online } => println!("Searching for papers"),
        Commands::Peek => println!("Displaying recently viewd papers/notes"),
        Commands::Cleanup => println!("Cleanup on aisle 3"),
    }
}
