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
        /// Flag to indicate if it's a PDF
        #[clap(long, short, action, group = "from")]
        pdf: bool,
        /// Flag to indicate if it's a PDF
        #[clap(long, short, action, group = "from")]
        web: bool,
    },
    /// Open pdf manually
    Open {
        /// Initial query for searching
        #[clap(value_name = "PROMPT", default_value_t = String::from(""))]
        query: String,
    },
    /// Open pdf manually
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
    Delete,

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
    /// Creates new stack with the same papers
    Merge {
        /// The new name for the stack
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
            (Some(stack), Some(StackAction::Delete)) => Ok(commands::stack::delete(stack)),
            (Some(stack), Some(StackAction::Rename { new_name })) => {
                Ok(commands::stack::rename(stack, new_name))
            }
            (Some(stack), Some(StackAction::Fork { new_stack })) => {
                Ok(commands::stack::fork(stack, new_stack))
            }
            (Some(stack), Some(StackAction::Merge { target })) => {
                Ok(commands::stack::merge(target, stack))
            }
            _ => Ok(println!("Invalid stack usage")),
        },
        Commands::Unstack => commands::stack::unstack(),
        Commands::Add { url, pdf, web } => commands::add::add(url, pdf, web),
        Commands::Open { query } => commands::prompt::open(query),
        Commands::Yank { query } => Ok(commands::prompt::yank(query)),
        Commands::List { max } => commands::prompt::list(max),
        Commands::Export => Ok(commands::export::export()),
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
