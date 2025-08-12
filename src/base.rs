use crate::bibtex::{self, BibtexError};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;
use termion::color;
use thiserror::Error;
use url::Url;
use webbrowser;

#[derive(Error, Debug)]
pub enum PdfError {
    #[error("Failed to create PDF directory: {0}")]
    CreateDirectory(#[from] std::io::Error),
    #[error("Failed to save PDF: {0}")]
    SaveError(String),
    #[error("PDF file not found at path")]
    NotFound,
    #[error("Invalid path")]
    InvalidPath,
    #[error("Failed to open PDF: {0}")]
    OpenError(String),
}

pub struct Embedding {
    pub id: u128,
    pub coords: Vec<f32>,
}

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
        // Parse the BibTeX string to get BibtexData
        let bibtex_data = bibtex::process_bibtex_entry(&bibtex_str)?;

        // Convert BibtexData to Paper
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

    /// Get the path to this paper's PDF file
    pub fn pdf_path(&self) -> PathBuf {
        PdfStorage::get_pdf_path(&self.key, self.id)
    }

    /// Check if a PDF file exists for this paper
    pub fn pdf_exists(&self) -> bool {
        self.pdf_path().exists()
    }

    /// Open this paper's PDF file
    pub fn open_pdf(&self, open_in_browser: bool) -> Result<(), PdfError> {
        let path = self.pdf_path();
        if !path.exists() {
            return Err(PdfError::NotFound);
        }
        PdfStorage::open_pdf(path, open_in_browser)
    }
}

/// Handles PDF file storage and retrieval
pub struct PdfStorage;

impl PdfStorage {
    /// Get the base papers directory
    fn get_base_dir() -> PathBuf {
        let mut base_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        base_dir.push(".bib");
        base_dir
    }

    /// Get the PDF storage directory path
    pub fn get_pdf_dir() -> PathBuf {
        let mut pdf_dir = Self::get_base_dir();
        pdf_dir.push("pdfs");
        pdf_dir
    }

    /// Generate a PDF filename for a paper
    fn generate_filename(paper_key: &str, paper_id: u128) -> String {
        let id_prefix = paper_id.to_string();
        let id_prefix = &id_prefix[..id_prefix.len().min(5)];
        format!("{}_{}.pdf", paper_key, id_prefix)
    }

    /// Get the path to a PDF file for a given paper key and ID
    pub fn get_pdf_path(paper_key: &str, paper_id: u128) -> PathBuf {
        let pdf_dir = Self::get_pdf_dir();
        let filename = Self::generate_filename(paper_key, paper_id);
        pdf_dir.join(filename)
    }

    /// Save PDF bytes to file with formatted name
    pub fn save_pdf(pdf_bytes: &[u8], paper: &Paper) -> Result<PathBuf, PdfError> {
        // Create the PDF directory if it doesn't exist
        let pdf_dir = Self::get_pdf_dir();
        fs::create_dir_all(&pdf_dir)?;

        // Generate the path
        let pdf_path = Self::get_pdf_path(&paper.key, paper.id);

        // Write the PDF bytes to file
        let mut file = File::create(&pdf_path)
            .map_err(|e| PdfError::SaveError(format!("Failed to create PDF file: {}", e)))?;

        file.write_all(pdf_bytes)
            .map_err(|e| PdfError::SaveError(format!("Failed to write PDF data: {}", e)))?;

        Ok(pdf_path)
    }

    /// Delete a PDF file for a given paper
    pub fn delete_pdf(paper: &Paper) -> Result<(), PdfError> {
        let pdf_path = Self::get_pdf_path(&paper.key, paper.id);
        if pdf_path.exists() {
            fs::remove_file(pdf_path)?;
        }
        Ok(())
    }

    /// Opens a PDF file using either the default PDF application or browser
    ///
    /// # Arguments
    /// * `path` - Path to the PDF file
    /// * `open_in_browser` - If true, opens in browser; if false, opens with default PDF app
    pub fn open_pdf<P: AsRef<Path>>(path: P, open_in_browser: bool) -> Result<(), PdfError> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(PdfError::InvalidPath);
        }

        if open_in_browser {
            // Convert file path to file:// URL for browser
            let file_url = format!("file://{}", path.canonicalize()?.display());
            webbrowser::open(&file_url)
                .map_err(|e| PdfError::OpenError(format!("Failed to open in browser: {}", e)))?;
            // open::that(file_url)
        } else {
            // Opens with default PDF viewer (Adobe Reader, Preview, etc.)
            open::that(path).map_err(|e| {
                PdfError::OpenError(format!("Failed to open with default app: {}", e))
            })?;
        }

        Ok(())
    }

    /// List all PDF files in the storage directory
    pub fn list_pdfs() -> Result<Vec<PathBuf>, PdfError> {
        let pdf_dir = Self::get_pdf_dir();
        if !pdf_dir.exists() {
            return Ok(Vec::new());
        }

        let mut pdfs = Vec::new();
        for entry in fs::read_dir(pdf_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("pdf") {
                pdfs.push(path);
            }
        }
        Ok(pdfs)
    }

    /// Get the total size of all stored PDFs in bytes
    pub fn total_storage_size() -> Result<u64, PdfError> {
        let pdfs = Self::list_pdfs()?;
        let mut total = 0u64;
        for pdf in pdfs {
            if let Ok(metadata) = fs::metadata(&pdf) {
                total += metadata.len();
            }
        }
        Ok(total)
    }

    /// Format file size for display
    pub fn format_file_size(bytes: usize) -> String {
        if bytes > 1024 * 1024 {
            format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
        } else if bytes > 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else {
            format!("{} bytes", bytes)
        }
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

#[macro_export]
macro_rules! blog {
    ($category:expr, $($arg:tt)*) => {{
        use termion::color;
        let formatted_args = format!($($arg)*);
        println!("{}{:>12}{} {}",color::Fg(color::Green), $category,color::Fg(color::Reset), formatted_args);
    }};
}

#[macro_export]
macro_rules! blog_warning {
    ($category:expr, $($arg:tt)*) => {{
        use termion::color;
        let formatted_args = format!($($arg)*);
        println!("{}{:>12}{} {}",color::Fg(color::Yellow), $category,color::Fg(color::Reset), formatted_args);
    }};
}

#[macro_export]
macro_rules! blog_working {
    ($category:expr, $($arg:tt)*) => {{
        use termion::color;
        let formatted_args = format!($($arg)*);
        println!("{}{:>12}{} {}",color::Fg(color::Blue), $category,color::Fg(color::Reset), formatted_args);
    }};
}

#[macro_export]
macro_rules! blog_done {
    ($category:expr, $($arg:tt)*) => {{
        use termion::color;
        let formatted_args = format!($($arg)*);
        println!("{}{:>12}{} {}",color::Fg(color::Green), $category,color::Fg(color::Reset), formatted_args);
    }};
}

/// Handles UI progress indicators
pub struct UI;

impl UI {
    pub fn download_progress(total_size: u64, url: &str) -> ProgressBar {
        let pb = ProgressBar::new(total_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{prefix:.blue.bold} {spinner:.blue} [{bar:30}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
                .expect("Invalid progress template")
                .progress_chars("=> "),
        );

        let domain = Url::parse(url)
            .ok()
            .and_then(|u| u.domain().map(|d| d.to_string()))
            .unwrap_or_else(|| "source".to_string());

        pb.set_prefix(format!("{:>12}", "Downloading"));
        pb.set_message(format!("from {}", domain));
        pb
    }
    /// Creates a progress bar for PDF upload operations
    pub fn pdf_upload_progress(filename: &str) -> ProgressBar {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{prefix:.blue.bold} {spinner:.blue} {msg}")
                .expect("Invalid spinner template")
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        pb.set_prefix(format!("{:>12}", "Uploading"));
        pb.set_message(format!("Reading {}", filename));
        pb.enable_steady_tick(Duration::from_millis(80));
        pb
    }

    /// Creates a multi-progress container for multiple uploads
    pub fn multi_progress() -> MultiProgress {
        MultiProgress::new()
    }

    pub fn spinner(category: &str, message: &str) -> ProgressBar {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{prefix:.blue.bold} {spinner:.blue} {msg}")
                .expect("Invalid spinner template")
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        pb.set_prefix(format!("{:>12}", category));
        pb.set_message(message.to_string());
        pb.enable_steady_tick(Duration::from_millis(80));
        pb
    }

    pub fn finish_with_message(pb: ProgressBar, completed_category: &str, message: &str) {
        pb.finish_and_clear();
        blog_done!(completed_category, "{}", message);
    }
}

// impl Paper {
//     pub fn open_pdf(&self) -> Result<()> {
//         let pdf_path = utils::io::pdf_path(&self.id)?;
//         open::that(pdf_path).map_err(|err| anyhow!("Could not open pdf: {}", err))
//     }
//     fn get_slack(&self) -> usize {
//         self.stack
//             .iter()
//             .fold(0, |acc, stack| acc + stack.name.len() + 3)
//     }
//     fn trim_details(&self, details: &str, max_length: u16) -> String {
//         let mut length = max_length as usize;
//         length -= 4 + 2;
//         length -= self.author.len() + 4;
//         length -= self.get_slack();
//         fit_string_to_length(details, length)
//     }
//     pub fn display(&self, max_width: u16, display_notes: bool) -> String {
//         let mut display_string = format!(
//             "{} {}|{} {} {}| ",
//             self.year,
//             color::Fg(color::Rgb(83, 110, 122)),
//             color::Fg(color::Reset),
//             self.author,
//             color::Fg(color::Rgb(83, 110, 122)),
//         );
//
//         // Only display notes if display_notes is true AND notes field contains a value
//         if display_notes && self.notes.is_some() {
//             display_string.push_str(&format!(
//                 "{}{}",
//                 self.trim_details(self.notes.as_ref().unwrap(), max_width),
//                 color::Fg(color::Reset)
//             ))
//         } else {
//             // Default to showing title
//             display_string.push_str(&format!(
//                 "{}{}",
//                 color::Fg(color::Reset),
//                 self.trim_details(&self.title, max_width),
//             ))
//         }
//
//         for stack in self.stack.iter() {
//             display_string.push_str(&format!(" {}", stack));
//         }
//         display_string
//     }
// }
//
