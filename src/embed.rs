use adamastor::Agent;

const EMBEDDING_DIM: u32 = 768;
const BATCH_SIZE: usize = 100;

pub struct Embedder {
    agent: Agent,
}

impl Embedder {
    pub fn new() -> Self {
        let api_key = std::env::var("GEMINI_KEY").expect("Set GEMINI_KEY environment variable");
        Self {
            agent: Agent::new(api_key).with_requests_per_second(1.0),
        }
    }

    pub async fn embed_query(&self, query_string: &str) -> Vec<f32> {
        let mut query = self
            .agent
            .encode(query_string)
            .dimensions(EMBEDDING_DIM)
            .as_query()
            .await
            .expect("Embedding API failed");
        caravela::normalize(&mut query);
        query
    }
    pub async fn embed_texts(&self, texts: &[String]) -> Vec<Vec<f32>> {
        if texts.is_empty() {
            return Vec::new();
        }

        let mut all_embeddings = Vec::with_capacity(texts.len());

        for chunk in texts.chunks(BATCH_SIZE) {
            let mut batch = self
                .agent
                .encode_batch(chunk.to_vec())
                .dimensions(EMBEDDING_DIM)
                .await
                .expect("Embedding API failed");

            for emb in &mut batch {
                caravela::normalize(emb);
            }

            all_embeddings.extend(batch);
        }

        all_embeddings
    }
}
