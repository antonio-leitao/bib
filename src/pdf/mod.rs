mod error;
pub use error::PdfError;

use crate::core::Paper;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct PdfStorage;

impl PdfStorage {
    fn get_base_dir() -> PathBuf {
        let mut base_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        base_dir.push(".bib");
        base_dir
    }

    pub fn get_pdf_dir() -> PathBuf {
        let mut pdf_dir = Self::get_base_dir();
        pdf_dir.push("pdfs");
        pdf_dir
    }

    fn generate_filename(paper_key: &str, paper_id: u128) -> String {
        let id_prefix = paper_id.to_string();
        let id_prefix = &id_prefix[..id_prefix.len().min(5)];
        format!("{}_{}.pdf", paper_key, id_prefix)
    }

    pub fn get_pdf_path(paper_key: &str, paper_id: u128) -> PathBuf {
        let pdf_dir = Self::get_pdf_dir();
        let filename = Self::generate_filename(paper_key, paper_id);
        pdf_dir.join(filename)
    }

    pub fn save_pdf(pdf_bytes: &[u8], paper: &Paper) -> Result<PathBuf, PdfError> {
        let pdf_dir = Self::get_pdf_dir();
        fs::create_dir_all(&pdf_dir)?;

        let pdf_path = Self::get_pdf_path(&paper.key, paper.id);

        let mut file = File::create(&pdf_path)
            .map_err(|e| PdfError::SaveFailed(format!("Failed to create PDF file: {}", e)))?;

        file.write_all(pdf_bytes)
            .map_err(|e| PdfError::SaveFailed(format!("Failed to write PDF data: {}", e)))?;

        Ok(pdf_path)
    }

    pub fn delete_pdf(paper: &Paper) -> Result<(), PdfError> {
        let pdf_path = Self::get_pdf_path(&paper.key, paper.id);
        if pdf_path.exists() {
            fs::remove_file(&pdf_path)
                .map_err(|e| PdfError::DeleteFailed(format!("Failed to delete PDF: {}", e)))?;
        }
        Ok(())
    }

    pub fn open_pdf<P: AsRef<Path>>(path: P, open_in_browser: bool) -> Result<(), PdfError> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(PdfError::FileNotFound(path.to_path_buf()));
        }

        if open_in_browser {
            let file_url = format!("file://{}", path.canonicalize()?.display());
            webbrowser::open(&file_url)
                .map_err(|e| PdfError::OpenFailed(format!("Failed to open in browser: {}", e)))?;
        } else {
            open::that(path).map_err(|e| {
                PdfError::OpenFailed(format!("Failed to open with default app: {}", e))
            })?;
        }

        Ok(())
    }

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
}
