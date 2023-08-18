mod commands;
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
    /// Adds files to myapp
    Add,
    Xplore {
        query: Option<String>,
    },
    Weave {
        query: Option<String>,
    },
}
fn main() {
    let cli = Cli::parse();

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Commands::Add => commands::add::add(),
        Commands::Weave { query } => commands::weave::weave(query.clone()),
        Commands::Xplore { query } => commands::xplore::execute(query.clone()),
    }
}
