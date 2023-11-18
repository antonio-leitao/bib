use crate::settings;
use crate::utils::ui;
use anyhow::{anyhow, Result};
use biblatex::Entry;
use open;
use serde::{Deserialize, Serialize};
use shellexpand::tilde;
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use termion::color;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Pdf {
    Url(String),
    Path(String),
}
impl Pdf {
    pub fn open(&self) -> Result<()> {
        let args = match self {
            Pdf::Url(url) => url.clone(),
            Pdf::Path(filename) => {
                let directory = settings::pdf_dir()?;
                let file_path = Path::new(&directory).join(&filename);
                if !file_path.exists() {
                    return Err(anyhow!(
                        "No PDF {} found on current stack\nAdd it manually with `bib add`",
                        &filename
                    ));
                };
                let args = directory.to_string() + &filename;
                args
            }
        };
        match open::that(&args) {
            Ok(()) => Ok(()),
            Err(err) => Err(anyhow!("Could not open pdf {}", err)),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaData {
    pub pdf: Option<Pdf>,
    pub notes: Option<String>,
    pub last_accessed: Option<u64>,
}

#[derive(Clone)]
pub struct Paper {
    pub id: String,
    pub author: String,
    pub year: i64,
    pub title: String,
    pub slug: String,
    pub entry: Entry,
    pub meta: Option<MetaData>,
}

impl Paper {
    pub fn update_last_accessed(&mut self) {
        // Convert Instant to UNIX timestamp (u64)
        let start = SystemTime::now();
        let unix_timestamp = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        // Update or create the metadata
        if let Some(metadata) = &mut self.meta {
            metadata.last_accessed = Some(unix_timestamp);
        } else {
            let new_metadata = MetaData {
                pdf: None,
                notes: None,
                last_accessed: Some(unix_timestamp),
            };
            self.meta = Some(new_metadata);
        }
    }
}

pub fn save<T: Serialize>(data: &HashMap<String, T>, filename: &str) -> Result<()> {
    let base_dir = settings::base_dir()?;
    let data_dir_path = Path::new(&base_dir);
    let data_file_path = data_dir_path.join(filename);
    let mut file = fs::File::create(data_file_path)?;
    // Serialize the HashMap<String, Note> to bytes and write it to the file
    let data_bytes = bincode::serialize(data)?;
    file.write_all(&data_bytes)?;

    Ok(())
}

pub fn read_metadata() -> Result<HashMap<String, MetaData>> {
    let base_dir = settings::base_dir()?;
    let data_dir_path = Path::new(&base_dir);
    let data_file_path = data_dir_path.join("metadata.bin");

    if data_file_path.exists() {
        let mut data_bytes = Vec::new();
        fs::File::open(data_file_path)?.read_to_end(&mut data_bytes)?;
        // Deserialize the bytes into a HashMap<String, Note>
        let data = bincode::deserialize(&data_bytes)?;
        Ok(data)
    } else {
        // Return an empty HashMap if the file does not exist
        Ok(HashMap::new())
    }
}

pub fn read_other_metadata(from: &str) -> Result<HashMap<String, MetaData>> {
    let dir = tilde(&format!("~/.bib/{}", from)).to_string();
    let data_dir_path = Path::new(&dir);
    let data_file_path = data_dir_path.join("metadata.bin");

    if data_file_path.exists() {
        let mut data_bytes = Vec::new();
        fs::File::open(data_file_path)?.read_to_end(&mut data_bytes)?;
        // Deserialize the bytes into a HashMap<String, Note>
        let data = bincode::deserialize(&data_bytes)?;
        Ok(data)
    } else {
        // Return an empty HashMap if the file does not exist
        Ok(HashMap::new())
    }
}

impl Paper {
    fn get_slack(&self) -> usize {
        let mut slack: usize = 0;
        if let Some(meta) = &self.meta {
            if meta.pdf.is_some() {
                slack += 6;
            }
            if meta.notes.is_some() {
                slack += 8;
            }
        }
        slack
    }
    fn trim_title(&self, max_length: u16) -> String {
        let mut length = max_length as usize;
        length -= 4 + 2;
        length -= self.author.len() + 4;
        length -= self.get_slack() as usize;
        fit_string_to_length(&self.title, length)
    }
}

impl ui::Item for Paper {
    fn display(&self, max_width: u16) -> String {
        // let slack = self.get_slack();
        let mut display_string = format!(
            "{} {}|{} {} {}|{} {}",
            self.year,
            color::Fg(color::Rgb(83, 110, 122)),
            color::Fg(color::Reset),
            self.author,
            color::Fg(color::Rgb(83, 110, 122)),
            color::Fg(color::Reset),
            self.trim_title(max_width),
        );

        // display_string = fit_string_to_length(display_string, max_width - slack);
        if let Some(meta) = &self.meta {
            if meta.pdf.is_some() {
                display_string.push_str(&format!(
                    " {}[PDF]{}",
                    color::Fg(color::Red),
                    color::Fg(color::Reset)
                ));
            }
            if meta.notes.is_some() {
                display_string.push_str(&format!(
                    " {}[Notes]{}",
                    color::Fg(color::Yellow),
                    color::Fg(color::Reset)
                ));
            }
        }
        display_string
    }
    fn disabled(&self, max_width: u16) -> String {
        // let slack = self.get_slack();
        let mut disabled_string = format!(
            "{}  {} | {} | {}",
            color::Fg(color::Rgb(83, 110, 122)),
            self.year,
            self.author,
            self.trim_title(max_width),
        );

        if let Some(meta) = &self.meta {
            if meta.pdf.is_some() {
                disabled_string.push_str(&format!(
                    " {}[PDF]{}",
                    color::Fg(color::Red),
                    color::Fg(color::Reset)
                ));
            }
            if meta.notes.is_some() {
                disabled_string.push_str(&format!(
                    " {}[Notes]{}",
                    color::Fg(color::Yellow),
                    color::Fg(color::Reset)
                ));
            }
        }
        disabled_string
    }
    fn slug(&self) -> String {
        self.slug.clone()
    }
}

fn fit_string_to_length(input: &str, max_length: usize) -> String {
    if input.len() <= max_length {
        return String::from(input);
    }

    let mut result = String::with_capacity(max_length);
    result.push_str(&input[..max_length - 3]);
    result.push_str("...");
    result
}
