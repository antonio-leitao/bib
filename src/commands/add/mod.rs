mod error;
mod handlers;
mod sources;

pub use error::AddError;

use crate::ai::Gemini;
use crate::core::Paper;
use crate::pdf::PdfStorage;
use crate::storage::PaperStore;
use crate::ui::{blog, blog_done, UI};
use handlers::{BibtexGenerator, PdfHandler};

pub async fn execute(
    input: Option<String>,
    notes: Option<String>,
    store: &mut PaperStore,
) -> Result<(), AddError> {
    // Get PDF source - errors bubble up with ?
    let pdf_source = PdfHandler::get_pdf_source(input).await?;
    let pdf_bytes = pdf_source.bytes.clone();

    // Start Gemini - AiError converts to AddError automatically
    let mut ai = Gemini::new()?;

    // Generate BibTeX - all errors bubble up cleanly
    let bibtex = BibtexGenerator::generate_bibtex(&mut ai, pdf_source).await?;

    // Create Paper - BibtexError converts to AddError
    let paper = Paper::from_bibtex(bibtex, notes)?;

    // Check if paper exists - StorageError converts to AddError
    if store.exists_by_key(&paper.key)? {
        blog!("Status", "Paper already exists with key: {}", paper.key);

        if paper.pdf_exists() {
            blog!(
                "PDF Status",
                "PDF already exists at: {}",
                paper.pdf_path().display()
            );
        }

        if prompt_user_confirmation("Would you like to update the existing entry?")? {
            process_paper_update(store, &mut ai, &paper, &pdf_bytes).await?;
        } else {
            blog!("Skipped", "Paper not saved");
        }
    } else {
        process_new_paper(store, &mut ai, &paper, &pdf_bytes).await?;
    }

    Ok(())
}

async fn process_new_paper(
    store: &mut PaperStore,
    ai: &mut Gemini,
    paper: &Paper,
    pdf_bytes: &[u8],
) -> Result<(), AddError> {
    let spinner = UI::spinner("Generating", "Paper embedding...");
    let embedding = ai.generate_paper_embedding().await?;
    UI::finish_with_message(
        spinner,
        "Generated",
        &format!("Paper embedding, dimensions: {}", embedding.len()),
    );

    store.create(&paper)?;
    store.save_embedding(paper.id, &embedding)?;

    let spinner = UI::spinner("Saving", "PDF to disk...");
    let pdf_path = PdfStorage::save_pdf(&pdf_bytes, &paper)?;
    let size_str = PdfStorage::format_file_size(pdf_bytes.len());
    UI::finish_with_message(
        spinner,
        "Saved PDF",
        &format!(
            "{} ({})",
            pdf_path.file_name().unwrap().to_string_lossy(),
            size_str
        ),
    );

    blog_done!("Saved", "{}", paper.title);
    blog!("PDF Path", "{}", pdf_path.display());

    Ok(())
}

async fn process_paper_update(
    store: &mut PaperStore,
    ai: &mut Gemini,
    paper: &Paper,
    pdf_bytes: &[u8],
) -> Result<(), AddError> {
    store.update(&paper)?;

    let spinner = UI::spinner("Updating", "Paper embedding...");
    let embedding = ai.generate_paper_embedding().await?;
    UI::finish_with_message(
        spinner,
        "Updated",
        &format!("Paper embedding, dimensions: {}", embedding.len()),
    );

    store.save_embedding(paper.id, &embedding)?;

    let spinner = UI::spinner("Updating", "PDF on disk...");
    let pdf_path = PdfStorage::save_pdf(&pdf_bytes, &paper)?;
    let size_str = PdfStorage::format_file_size(pdf_bytes.len());
    UI::finish_with_message(
        spinner,
        "Updated PDF",
        &format!(
            "{} ({})",
            pdf_path.file_name().unwrap().to_string_lossy(),
            size_str
        ),
    );

    blog_done!("Updated", "Paper successfully updated in database");
    blog!("PDF Path", "{}", pdf_path.display());

    Ok(())
}

fn prompt_user_confirmation(message: &str) -> Result<bool, AddError> {
    println!("\n{} (y/n)", message);

    use std::io::{self, BufRead};
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut response = String::new();
    handle.read_line(&mut response)?;

    Ok(response.trim().to_lowercase() == "y")
}
