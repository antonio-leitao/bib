use crate::utils;
use anyhow::Result;
use bincode::{deserialize, serialize};
use dotzilla;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BinaryHeap};
use std::fs::File;
use std::io::{Read, Write};

#[derive(Serialize, Deserialize)]
pub struct Point {
    id: String,
    coords: Vec<f32>,
}

impl Point {
    pub fn new(id: String, embedding: Vec<f32>) -> Self {
        Point {
            id,
            coords: embedding,
        }
    }
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
