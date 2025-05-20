use clap::{Parser, Subcommand};
use termion::color;
mod base;
mod commands;
mod embedding;
mod parser;
mod stacks;
mod utils;

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
        #[clap(value_name = "URL", default_value_t = String::from(""))]
        url: String,
    },
    /// Open pdf manually
    Open {
        /// Initial query for searching
        #[clap(value_name = "PROMPT", default_value_t = String::from(""))]
        query: String,
    },
    /// Copy bibtex to clipboard
    Yank {
        /// Initial query for searching
        #[clap(value_name = "PROMPT", default_value_t = String::from(""))]
        query: String,
    },
    /// Lists the references in the stack
    List {
        #[clap(value_name = "LENGTH", short, long)]
        max: Option<usize>,
    },
    /// Export bib file
    Export,
    /// Unset the current stack
    Unstack,
    /// Manage stacks
    Stack {
        /// The stack name (optional for certain subcommands)
        #[arg(value_name = "NAME")]
        name: Option<String>,

        /// Stack subcommands (new, delete, rename)
        #[command(subcommand)]
        action: Option<StackAction>,
    },
}

#[derive(Subcommand)]
enum StackAction {
    /// Create a new stack
    New,

    /// Delete the specified stack
    Drop,

    /// Toggle paper to/from stack
    Toggle {
        /// Initial query for searching
        #[clap(value_name = "PROMPT", default_value_t = String::from(""))]
        query: String,
    },

    /// Rename the specified stack
    Rename {
        /// The new name for the stack
        #[arg(value_name = "NEW NAME")]
        new_name: String,
    },
    /// Creates new stack with the same papers
    Fork {
        /// The new name for the stack
        #[arg(value_name = "NEW STACK")]
        new_stack: String,
    },
    /// Brings all items into stack
    Merge {
        /// The target stack
        #[arg(value_name = "TARGET")]
        target: String,
    },
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Stack { name, action } => match (name, action) {
            (None, None) => commands::stack::list(),
            (Some(stack), None) => commands::stack::switch(stack),
            (Some(stack), Some(StackAction::New)) => commands::stack::new(stack),
            (Some(stack), Some(StackAction::Drop)) => commands::stack::drop(stack),
            (Some(stack), Some(StackAction::Rename { new_name })) => {
                commands::stack::rename(stack, new_name)
            }
            (Some(stack), Some(StackAction::Fork { new_stack })) => {
                commands::stack::fork(stack, new_stack)
            }
            (Some(stack), Some(StackAction::Merge { target })) => {
                commands::stack::merge(target, stack)
            }
            (Some(stack), Some(StackAction::Toggle { query })) => {
                commands::prompt::toggle(stack, query)
            }
            _ => Ok(println!("Invalid stack usage")),
        },
        Commands::Unstack => commands::stack::unstack(),
        Commands::Add { url } => commands::add::add(url),
        Commands::Open { query } => commands::prompt::open(query),
        Commands::Yank { query } => commands::prompt::yank(query),
        Commands::List { max } => commands::prompt::list(max),
        Commands::Export => commands::export::export(),
    };
    match result {
        Ok(()) => (),
        Err(err) => erro(err.to_string()),
    }
}

fn erro(err: String) {
    println!(
        "{}error{}: {}",
        color::Fg(color::Red),
        color::Fg(color::Reset),
        err
    );
}
