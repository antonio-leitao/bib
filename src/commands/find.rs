use crate::base::{Embedding, PdfStorage, UI};
use crate::gemini::Gemini;
use crate::store::PaperStore;
use anyhow::Result;
use dotzilla;
use std::cmp::Ordering;
use std::path::PathBuf;

fn k_nearest(vectors: Vec<Embedding>, query: &[f32], k: usize) -> Vec<(u128, f32)> {
    if vectors.is_empty() || k == 0 {
        return Vec::new();
    }

    let mut scores: Vec<_> = vectors
        .into_iter()
        .map(|e| (e.id, dotzilla::dot(query, &e.coords)))
        .collect();

    let k = k.min(scores.len());

    // Partition so that top k elements are at the beginning
    scores.select_nth_unstable_by(k - 1, |a, b| {
        b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal)
    });

    // Sort only the top k elements
    scores[..k].sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
    scores.truncate(k);
    scores
}

pub async fn find(store: &mut PaperStore, query: &str, limit: usize) -> Result<()> {
    let spinner = UI::spinner("Generating", "Query embedding...");

    let ai = Gemini::new()?;
    let query_vector = ai.generate_query_embedding(query).await?;
    UI::finish_with_message(spinner, "Generated", "query embedding.");
    let spinner = UI::spinner("Searching", "data for relevant papers...");
    let vectors = store.load_all_embeddings()?;
    let top_k = k_nearest(vectors, &query_vector, limit);
    UI::finish_with_message(spinner, "Found", "initial pool of relevant papers.");
    let id_list: Vec<u128> = top_k.into_iter().map(|(id, _)| id).collect();
    let papers = store.get_by_ids(&id_list)?;
    let pdf_paths: Vec<PathBuf> = papers
        .iter()
        .take(8) //Max 8 papers sent to the gemini API maybe we can bump to 10 lets see
        .map(|paper| PdfStorage::get_pdf_path(&paper.key, paper.id))
        .collect();
    let uploads = ai.concurrent_upload_pdfs_with_progress(&pdf_paths).await;
    // Use successful_uploads with your analysis function
    let spinner = UI::spinner("Analyzing", "relevant papers...");
    let analysis = ai
        .analyze_research_papers(query, &uploads.iter().collect::<Vec<_>>().as_slice())
        .await?;
    UI::finish_with_message(spinner, "Analzed", "relevant papers.");

    println!("{}", serde_json::to_string_pretty(&analysis)?);
    // for paper in  {
    //     println!("{}", paper.display(88));
    // }
    Ok(())
}
