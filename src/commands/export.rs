use crate::base::{load_papers, Paper};
use crate::utils::io::read_config_file;
use anyhow::Result;
use indexmap::IndexMap;
use std::io::{self, Write};

fn filter_by_stack(papers: &IndexMap<String, Paper>) -> Result<Vec<String>> {
    let config = read_config_file()?;
    let indicies: Vec<String> = match config.current_stack() {
        Some(current) => papers
            .iter()
            .filter(|(_key, paper)| paper.stack.iter().any(|stack| *stack == current))
            .map(|(key, _paper)| key.clone())
            .collect(),

        None => papers.keys().map(|key| key.clone()).collect(),
    };
    Ok(indicies)
}

pub fn export() -> Result<()> {
    let papers = load_papers()?;
    let indices = filter_by_stack(&papers)?;

    let bibtex_entries: String = indices
        .iter()
        .filter_map(|id| papers.get(id))
        .map(|paper| paper.bibtex.clone())
        .collect::<Vec<String>>()
        .join("\n");
    // Print the concatenated BibTeX entries to stdout
    io::stdout().write_all(bibtex_entries.as_bytes())?;
    io::stdout().flush()?;
    Ok(())
}
