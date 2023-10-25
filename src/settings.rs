use anyhow::Result;
use shellexpand::tilde;
use std::fs;
// Add scopes
// Add editor
pub const QUERY_LIMIT: usize = 100;
pub const EDITOR: &str = "vim";
// Add llm path
pub fn base_dir() -> Result<String> {
    let dir = tilde("~/.ark").to_string();
    fs::create_dir_all(&dir)?;
    Ok(dir)
}
pub fn notes_dir() -> Result<String> {
    let dir = tilde("~/.ark/notes").to_string();
    fs::create_dir_all(&dir)?;
    Ok(dir)
}
pub fn pdf_dir() -> Result<String> {
    let dir = tilde("~/.ark/pdf").to_string();
    fs::create_dir_all(&dir)?;
    Ok(dir)
}
