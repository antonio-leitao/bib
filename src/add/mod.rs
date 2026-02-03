pub mod grobid;
mod input;
use crate::embed::Embedder;
use crate::ui::StatusUI;
use crate::{config::Config, database::CitationDb};
use anyhow::Result;
use input::{PdfHandler, PdfSource};
use thiserror::Error;
use tokio::fs;

#[derive(Error, Debug)]
pub enum AddError {
    #[error("Failed to open PDF: {0}")]
    FileOpen(String),

    #[error("Grobid error: {0}")]
    Grobid(#[from] grobid::GrobidError),
}

pub async fn run(cfg: &Config, db: &mut CitationDb, input: Option<String>) -> Result<()> {
    let (bytes, source) = PdfHandler::get_pdf_source(input).await?;
    let paper_key = add_single(db, bytes.clone()).await?;
    match source {
        PdfSource::Online => {
            let target_path = cfg.pdf_dir().join(format!("{}.pdf", paper_key));
            fs::write(&target_path, bytes).await?;
            StatusUI::success(&format!("Saved {}.pdf", paper_key));
        }
        PdfSource::Path(path) => {
            let paper_path = fs::canonicalize(path).await?;
            let pdf_dir = cfg.pdf_dir();
            let target_path = pdf_dir.join(format!("{}.pdf", paper_key));

            if paper_path.parent() == Some(&pdf_dir) {
                fs::rename(&paper_path, &target_path).await?;
                StatusUI::info(&format!("Renamed to {}.pdf", paper_key));
            } else {
                fs::copy(&paper_path, &target_path).await?;
                StatusUI::info(&format!("Imported {}.pdf", paper_key));
            }
        }
    }
    Ok(())
}

async fn add_single(db: &mut CitationDb, pdf_bytes: Vec<u8>) -> Result<String> {
    let client = grobid::GrobidClient::new().await?;
    let embedder = Embedder::new();

    let pb = StatusUI::spinner("Parsing PDF with Grobid...");
    let paper = client.process_pdf(pdf_bytes).await?;
    StatusUI::finish_spinner_success(pb, &format!("Parsed: {}", paper.title));

    #[cfg(debug_assertions)]
    {
        println!("Key: {}", paper.key);
        println!("Title: {}", paper.title);
        println!("Year: {:?}", paper.year);
        println!("Authors: {}", paper.authors);
        println!("\nParagraphs ({}):", paper.paragraphs.len());
        for p in &paper.paragraphs {
            println!("  Text: {}", &p.text);
            println!("  Cites: {:?}", p.cited_keys);
            println!();
        }
        println!("References ({}):", paper.references.len());
        for r in &paper.references {
            println!(
                "  [{}] {} ({:?}) - title: {}",
                r.key, r.authors, r.year, r.title
            );
        }
    }

    let pb = StatusUI::spinner("Generating embeddings...");
    let embedded = paper.embed(&embedder).await;
    db.ingest(&embedded)?;
    StatusUI::finish_spinner_success(pb, &format!("Added: {} - {}", embedded.key, embedded.title));

    Ok(embedded.key)
}

pub async fn sync(cfg: &Config, db: &mut CitationDb) -> Result<()> {
    let client = grobid::GrobidClient::new().await?;
    let embedder = Embedder::new();

    let pdf_dir = cfg.pdf_dir();
    StatusUI::info(&format!("Scanning: {}", pdf_dir.display()));

    let mut entries = fs::read_dir(&pdf_dir).await?;
    let mut processed = 0;
    let mut skipped = 0;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();

        // Skip non-PDFs
        if path.extension().and_then(|s| s.to_str()) != Some("pdf") {
            continue;
        }

        let paper_path = fs::canonicalize(&path).await?;
        let current_stem = paper_path
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();

        // Already processed with correct name
        if db.is_processed(&current_stem)? {
            skipped += 1;
            continue;
        }

        let filename = paper_path.file_name().unwrap().to_string_lossy();
        let pb = StatusUI::spinner(&format!("Parsing {}...", filename));

        // Process PDF
        let file_bytes = fs::read(&paper_path)
            .await
            .map_err(|e| AddError::FileOpen(e.to_string()))?;

        let paper = match client.process_pdf(file_bytes).await {
            Ok(p) => p,
            Err(e) => {
                StatusUI::finish_spinner_error(pb, &format!("Failed: {}", filename));
                StatusUI::render_error(e);
                continue;
            }
        };

        let detected_key = &paper.key;
        let target_path = pdf_dir.join(format!("{}.pdf", detected_key));
        let key_in_db = db.is_processed(detected_key)?;
        let target_exists = fs::metadata(&target_path).await.is_ok();

        // Handle duplicates - finish spinner first, then warn
        if key_in_db {
            if target_exists {
                fs::remove_file(&paper_path).await?;
                StatusUI::finish_spinner_warning(pb, &format!("Duplicate: {}", detected_key));
            } else {
                fs::rename(&paper_path, &target_path).await?;
                StatusUI::finish_spinner_warning(
                    pb,
                    &format!("Duplicate: {} (renamed)", detected_key),
                );
            }
            skipped += 1;
            continue;
        }

        // Check if target file already exists but not in DB (edge case)
        if target_exists && paper_path != target_path {
            fs::remove_file(&paper_path).await?;
            StatusUI::finish_spinner_warning(
                pb,
                &format!("Duplicate: {} (file exists)", detected_key),
            );
            skipped += 1;
            continue;
        }

        // Update spinner for embedding stage
        pb.set_message(format!("Embedding {}...", detected_key));

        // New paper: embed and ingest
        let embedded = paper.embed(&embedder).await;
        db.ingest(&embedded)?;

        // Rename to canonical name if needed
        if paper_path != target_path {
            fs::rename(&paper_path, &target_path).await?;
        }

        StatusUI::finish_spinner_success(
            pb,
            &format!("Added: {} - {}", embedded.key, embedded.title),
        );
        processed += 1;
    }

    StatusUI::success(&format!(
        "Sync complete: {} processed, {} skipped",
        processed, skipped
    ));
    Ok(())
}
