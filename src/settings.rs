use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use shellexpand::tilde;
use std::fs;
use std::io::{BufReader, Read, Write};
use std::path::Path;
use toml;

pub const EDITOR: &str = "nvim";

fn directory_exists(directory_path: &str) -> bool {
    let path = Path::new(directory_path);
    path.is_dir()
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub stack: String,
}

fn read_config_file() -> Result<Config> {
    // base directory if it doesn't exist
    let dir = tilde("~/.bib").to_string();

    // Read the contents of the config.toml file
    let file_path = dir + "/config.toml";
    let file = fs::File::open(&file_path)?;
    let mut reader = BufReader::new(file);
    let mut toml_content = String::new();
    reader.read_to_string(&mut toml_content)?;

    // Deserialize the TOML content into a Config struct
    let config: Config = toml::from_str(&toml_content)?;

    Ok(config)
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

pub fn current_stack() -> Result<String> {
    let config = read_config_file()?;
    Ok(config.stack)
}

pub fn list_stacks() -> Result<Vec<String>> {
    let dir = tilde("~/.bib").to_string();
    if !directory_exists(&dir) {
        bail!("Bib not initiated, run Bib init");
    };
    let entries = fs::read_dir(dir)?;
    let mut directories = Vec::new();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if let Some(dir_name) = path.file_name() {
                if let Some(dir_str) = dir_name.to_str() {
                    directories.push(dir_str.to_string());
                }
            }
        }
    }

    Ok(directories)
}
pub fn base_dir() -> Result<String> {
    let stack = current_stack()?;
    let path = format!("~/.bib/{}", stack);
    let dir = tilde(&path).to_string();
    if !directory_exists(&dir) {
        bail!("Could not find stack. Run bib stack {} to create", stack);
    };
    Ok(dir)
}
