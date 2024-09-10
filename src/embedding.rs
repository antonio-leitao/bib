use crate::utils::io::model_dir;
use crate::{blog, utils};
use anyhow::{bail, Result};
use bincode::{deserialize, serialize};
use dotzilla;
use fastembed::{
    read_file_to_bytes, InitOptionsUserDefined, Pooling, QuantizationMode, TextEmbedding,
    TokenizerFiles, UserDefinedEmbeddingModel,
};
use gag::Gag;
use hf_hub::api::sync::ApiBuilder;
use hf_hub::Cache;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::collections::BinaryHeap;
use std::fs::File;
use std::io::{Read, Write};

#[derive(Serialize, Deserialize)]
pub struct Point {
    id: String,
    coords: Vec<f32>,
}

impl Point {
    pub fn from_bytes(id: String, bytes: Vec<u8>) -> Result<Self> {
        //let text = extract_ascii_only(bytes)?;
        blog!("Extracting", "text from pdf");
        let text: String;
        {
            let _print_gag = Gag::stdout().unwrap();
            text = extract_text_from_pdf(bytes)?;
        }
        blog!("Embedding", "using JINA-v2-small-8k");
        let coords = encode(&text)?;
        Ok(Point { id, coords })
    }
}

fn extract_text_from_pdf(bytes: Vec<u8>) -> Result<String> {
    let text = pdf_extract::extract_text_from_mem(&bytes)?;
    Ok(text
        .chars()
        .filter(|&c| c.is_ascii() && !c.is_control())
        .collect())
}

fn load_model() -> Result<UserDefinedEmbeddingModel> {
    let cache_dir = model_dir()?;

    let cache = Cache::new(cache_dir);
    let api = ApiBuilder::from_cache(cache)
        .with_progress(true)
        .build()
        .unwrap();

    let repo = api.model("jinaai/jina-embeddings-v2-small-en".to_string());

    let tokenizer_files: TokenizerFiles = TokenizerFiles {
        tokenizer_file: read_file_to_bytes(&repo.get("tokenizer.json")?)?,
        config_file: read_file_to_bytes(&repo.get("config.json")?)?,
        special_tokens_map_file: read_file_to_bytes(&repo.get("special_tokens_map.json")?)?,
        tokenizer_config_file: read_file_to_bytes(&repo.get("tokenizer_config.json")?)?,
    };

    let onnx_file = read_file_to_bytes(&repo.get("model.onnx")?)?;

    let jina_model = UserDefinedEmbeddingModel::new(onnx_file, tokenizer_files)
        .with_pooling(Pooling::Mean)
        .with_quantization(QuantizationMode::Dynamic);

    Ok(jina_model)
}

pub fn save_vectors(vectors: &BTreeMap<String, Point>) -> Result<()> {
    let encoded: Vec<u8> = serialize(vectors)?;
    let filename = utils::io::vectors_path()?;
    let mut file = File::create(filename)?;
    file.write_all(&encoded)?;
    Ok(())
}

pub fn load_vectors() -> Result<BTreeMap<String, Point>> {
    let filename = utils::io::vectors_path()?;
    if !filename.exists() {
        return Ok(BTreeMap::new());
    }
    let mut file = File::open(filename)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let decoded: BTreeMap<String, Point> = deserialize(&buffer)?;
    Ok(decoded)
}

pub fn encode(sentence: &str) -> Result<Vec<f32>> {
    let jina_model = load_model()?;
    let jina_options = InitOptionsUserDefined::new().with_max_length(8192); // <- Jina FTW
    let model = TextEmbedding::try_new_from_user_defined(jina_model, jina_options)?;
    let documents = vec![sentence];
    let embeddings = model.embed(documents, None)?;
    // Check if there is at least one embedding
    if embeddings.is_empty() {
        bail!("No embeddings were generated.");
    }
    // Return the first embedding vector
    Ok(embeddings.into_iter().next().unwrap())
}

#[derive(Clone, Debug)]
struct PointDistance {
    id: String,
    dist: f32,
}

impl Eq for PointDistance {}

impl PartialEq for PointDistance {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Ord for PointDistance {
    fn cmp(&self, other: &Self) -> Ordering {
        other.dist.partial_cmp(&self.dist).unwrap()
    }
}

impl PartialOrd for PointDistance {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn k_nearest(
    query: &[f32],
    points: &BTreeMap<String, Point>,
    ids: &Vec<String>,
    k: usize,
) -> Vec<String> {
    let mut heap = BinaryHeap::with_capacity(k + 1);
    for id in ids {
        let dist = dotzilla::dot(query, &points[id].coords);
        let point_dist = PointDistance {
            id: id.to_string(),
            dist,
        };
        if heap.len() < k {
            heap.push(point_dist);
        } else if dist > heap.peek().unwrap().dist {
            heap.pop();
            heap.push(point_dist);
        }
    }
    heap.into_sorted_vec()
        .iter()
        .map(|pd| pd.id.clone())
        .collect()
}
