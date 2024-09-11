use crate::stacks::Stack;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use shellexpand::tilde;
use std::fs::{self, File};
use std::io::{BufReader, Read, Write};
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub stack: String,
    pub stacks: Vec<Stack>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            stack: "all".to_string(),
            stacks: Vec::default(),
        }
    }
}
impl Config {
    pub fn current_stack(&self) -> Option<Stack> {
        if self.stack == "all" {
            return None;
        }
        self.stacks.iter().find(|s| s.name == self.stack).cloned()
    }
}

pub fn read_config_file() -> Result<Config> {
    // Base directory
    let base_dir = tilde("~/.bib").to_string();
    let mut config_path = PathBuf::from(&base_dir);
    // Make sure the directories exist
    fs::create_dir_all(&config_path)?;
    config_path.push("config.toml");
    // Check if the file exists
    if config_path.exists() {
        // Read the contents of the config.toml file
        let file = fs::File::open(&config_path)?;
        let mut reader = BufReader::new(file);
        let mut toml_content = String::new();
        reader.read_to_string(&mut toml_content)?;
        // Deserialize the TOML content into a Config struct
        let config: Config = toml::from_str(&toml_content)?;
        Ok(config)
    } else {
        // Return default configuration if file doesn't exist
        Ok(Config::default())
    }
}

pub fn save_config_file(config: &Config) -> Result<()> {
    // Serialize the Config struct to TOML
    let toml_content = toml::to_string_pretty(config)?;
    // Create the directory if it doesn't exist
    let dir = tilde("~/.bib").to_string();
    fs::create_dir_all(&dir)?;
    // Create and write to the config.toml file
    let file_path = dir + "/config.toml";
    let mut file = fs::File::create(&file_path)?;
    file.write_all(toml_content.as_bytes())?;
    Ok(())
}

pub fn pdf_path(pdf_name: &str) -> Result<PathBuf> {
    // Expand the tilde to the user's home directory
    let base_dir = tilde("~/.bib/pdfs").to_string();
    let mut pdfs_path = PathBuf::from(&base_dir);
    // Make sure the directories exist
    fs::create_dir_all(&pdfs_path)?;
    // Append the PDF file name to the path
    pdfs_path.push(format!("{}.pdf", pdf_name));
    // Return the full path as a PathBuf
    Ok(pdfs_path)
}
pub fn vectors_path() -> Result<PathBuf> {
    // Expand the tilde to the user's home directory
    let base_dir = tilde("~/.bib").to_string();
    let mut bib_path = PathBuf::from(&base_dir);
    // Make sure the directories exist
    fs::create_dir_all(&bib_path)?;
    // Append the PDF file name to the path
    bib_path.push("vectors.bin");
    // Return the full path as a PathBuf
    Ok(bib_path)
}

pub fn papers_path() -> Result<PathBuf> {
    // Expand the tilde to the user's home directory
    let base_dir = tilde("~/.bib").to_string();
    let mut bib_path = PathBuf::from(&base_dir);
    // Make sure the directories exist
    fs::create_dir_all(&bib_path)?;
    // Append the PDF file name to the path
    bib_path.push("papers.bin");
    // Return the full path as a PathBuf
    Ok(bib_path)
}

pub fn read_and_move_file(path: &str, paper_id: &str) -> Result<Vec<u8>> {
    // Read the contents of the file
    let mut file = File::open(path)?;
    let mut contents = Vec::new();

    file.read_to_end(&mut contents)?;
    // Move the file
    let new_path = pdf_path(paper_id)?;
    fs::rename(path, new_path)?;
    // Return the contents
    Ok(contents)
}

pub fn model_dir() -> Result<PathBuf> {
    // Expand the tilde to the user's home directory
    let base_dir = tilde("~/.bib/llm").to_string();
    let pdfs_path = PathBuf::from(&base_dir);
    // Make sure the directories exist
    fs::create_dir_all(&pdfs_path)?;
    Ok(pdfs_path)
}
