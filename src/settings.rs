use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use shellexpand::tilde;
use std::fs;
use std::io::{BufReader, Read, Write};
use std::path::Path;
use std::path::PathBuf;
use toml;

//maybe make this part of the config
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
    let mut stacks = Vec::new();
    let entries = fs::read_dir(dir)?;
    for entry in entries {
        let file_name = entry?.file_name();
        let file_name_str = file_name.to_string_lossy();
        if file_name_str.ends_with(".bib") {
            let stack_name = file_name_str.trim_end_matches(".bib").to_string();
            stacks.push(stack_name);
        }
    }
    Ok(stacks)
}
pub fn base_bib_path() -> Result<PathBuf> {
    let stack = current_stack()?;
    let bib_file = format!("{}.bib", stack);
    let base_dir = tilde("~/.bib").to_string();
    let bib_path = Path::new(&base_dir).join(bib_file);
    if !bib_path.exists() {
        bail!("Could not find stack. Run bib stack {} to create", stack);
    }
    Ok(bib_path)
}
