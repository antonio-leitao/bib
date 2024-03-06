use crate::fmt;
use crate::settings;
use crate::utils::bibfile;
use anyhow::Result;
use std::fs::File;
use std::io::Write;

fn create_bib_file(content: &str, filename: &str) -> std::io::Result<()> {
    let mut file = File::create(format!("{}.bib", filename))?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

pub fn export(filename: Option<String>) -> Result<()> {
    let bibliography = bibfile::read_bibliography()?;
    let content = bibliography.to_biblatex_string();
    let stack = settings::current_stack()?;
    let outname = match filename {
        Some(name) => name,
        None => stack.clone(),
    };
    create_bib_file(&content, &outname)?;
    fmt::export(stack, outname);
    Ok(())
}
