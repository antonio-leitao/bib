use crate::ui::StatusUI;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("No configuration found. Run:\n  bib config --pdf-dir <path>")]
    NotFound,
    #[error("Could not determine config directory")]
    NoProjectDirs,
    #[error("Failed to read config: {0}")]
    Read(#[from] std::io::Error),
    #[error("Failed to parse config: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("Failed to serialize config: {0}")]
    Serialize(#[from] toml::ser::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pdf_dir: PathBuf,
}

impl Config {
    fn project_dirs() -> Result<ProjectDirs, ConfigError> {
        ProjectDirs::from("", "", "bib").ok_or(ConfigError::NoProjectDirs)
    }

    fn config_path() -> Result<PathBuf, ConfigError> {
        Ok(Self::project_dirs()?.config_dir().join("config.toml"))
    }

    pub fn database_path() -> Result<PathBuf, ConfigError> {
        Ok(Self::project_dirs()?.data_dir().join("bib.db"))
    }

    pub fn pdf_dir(&self) -> &PathBuf {
        &self.pdf_dir
    }

    pub fn load() -> Result<Self, ConfigError> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Err(ConfigError::NotFound);
        }
        let contents = fs::read_to_string(&path)?;
        Ok(toml::from_str(&contents)?)
    }

    pub fn create(pdf_dir: PathBuf) -> Result<Self, ConfigError> {
        let pdf_dir: PathBuf = shellexpand::tilde(&pdf_dir.to_string_lossy())
            .into_owned()
            .into();
        fs::create_dir_all(&pdf_dir)?;

        let config = Config {
            pdf_dir: fs::canonicalize(&pdf_dir)?,
        };

        let config_path = Self::config_path()?;
        fs::create_dir_all(config_path.parent().unwrap())?;
        fs::write(&config_path, toml::to_string_pretty(&config)?)?;

        fs::create_dir_all(Self::project_dirs()?.data_dir())?;

        StatusUI::success(&format!("Config saved to: {}", config_path.display()));
        Ok(config)
    }
}
