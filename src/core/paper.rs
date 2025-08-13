use crate::bibtex::{self, BibtexError};
use crate::pdf::PdfStorage;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use termion::color;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Paper {
    pub id: u128,
    pub key: String,
    pub author: String,
    pub year: i64,
    pub title: String,
    pub notes: Option<String>,
    pub content: String,
    pub bibtex: String,
}

impl Paper {
    pub fn from_bibtex(bibtex_str: String, notes: Option<String>) -> Result<Self, BibtexError> {
        let bibtex_data = bibtex::process_bibtex_entry(&bibtex_str)?;

        Ok(Paper {
            id: bibtex_data.content_id,
            key: bibtex_data.key,
            author: bibtex_data.author,
            year: bibtex_data.year,
            title: bibtex_data.title,
            notes,
            content: bibtex_data.content,
            bibtex: bibtex_str,
        })
    }

    pub fn display(&self, max_width: u16) -> String {
        let mut display_string = format!(
            "{} {}|{} {} {}| ",
            self.year,
            color::Fg(color::Rgb(83, 110, 122)),
            color::Fg(color::Reset),
            self.author,
            color::Fg(color::Rgb(83, 110, 122)),
        );
        display_string.push_str(&format!(
            "{}{}",
            color::Fg(color::Reset),
            self.trim_details(&self.title, max_width),
        ));
        display_string
    }

    fn trim_details(&self, details: &str, max_length: u16) -> String {
        let mut length = max_length as usize;
        length -= 4 + 2;
        length -= self.author.len() + 4;
        fit_string_to_length(details, length)
    }

    pub fn pdf_path(&self) -> PathBuf {
        PdfStorage::get_pdf_path(&self.key, self.id)
    }

    pub fn pdf_exists(&self) -> bool {
        self.pdf_path().exists()
    }

    pub fn open_pdf(&self, open_in_browser: bool) -> Result<(), crate::pdf::PdfError> {
        let path = self.pdf_path();
        if !path.exists() {
            return Err(crate::pdf::PdfError::FileNotFound(path));
        }
        PdfStorage::open_pdf(path, open_in_browser)
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
