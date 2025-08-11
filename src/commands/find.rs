use crate::base::{Embedding, UI};
use crate::gemini::Gemini;
use crate::store::PaperStore;
use anyhow::Result;
use dotzilla;
use std::cmp::Ordering;

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
    UI::finish_with_message(spinner, "Generated", "Query embedding.");
    let spinner = UI::spinner("Searching", "For database for relevant papers...");
    let vectors = store.load_all_embeddings()?;
    let top_k = k_nearest(vectors, &query_vector, limit);
    UI::finish_with_message(spinner, "Found", "Relevant papers.");
    let id_list: Vec<u128> = top_k.into_iter().map(|(id, _)| id).collect();
    for paper in store.get_by_ids(&id_list)? {
        println!("{}", paper.display(88));
    }
    Ok(())
}
