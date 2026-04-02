use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// A single entry in the vector store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorEntry {
    pub id: Uuid,
    pub text: String,
    pub embedding: Vec<f32>,
    pub metadata: serde_json::Value,
}

/// In-memory vector store with cosine similarity search.
/// For production, replace with Qdrant, Pinecone, or pgvector.
#[derive(Clone, Default)]
pub struct InMemoryVectorStore {
    entries: Arc<DashMap<Uuid, VectorEntry>>,
}

impl InMemoryVectorStore {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
        }
    }

    /// Insert an entry. Returns the assigned ID.
    pub fn insert(&self, text: impl Into<String>, embedding: Vec<f32>, metadata: serde_json::Value) -> Uuid {
        let entry = VectorEntry {
            id: Uuid::new_v4(),
            text: text.into(),
            embedding,
            metadata,
        };
        let id = entry.id;
        self.entries.insert(id, entry);
        id
    }

    /// Search for the top-k most similar entries by cosine similarity.
    pub fn search(&self, query: &[f32], top_k: usize) -> Vec<(VectorEntry, f32)> {
        let mut scored: Vec<(VectorEntry, f32)> = self
            .entries
            .iter()
            .map(|e| {
                let sim = cosine_similarity(query, &e.embedding);
                (e.clone(), sim)
            })
            .collect();

        // Sort descending by similarity score
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        scored
    }

    /// Delete an entry by ID.
    pub fn delete(&self, id: &Uuid) {
        self.entries.remove(id);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Cosine similarity between two vectors.
/// Returns a value in [-1.0, 1.0]; 1.0 = identical direction.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_similarity_identical_vectors() {
        let v = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&v, &v) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_similarity_orthogonal_vectors() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!((cosine_similarity(&a, &b)).abs() < 1e-6);
    }

    #[test]
    fn search_returns_top_k() {
        let store = InMemoryVectorStore::new();
        store.insert("doc1", vec![1.0, 0.0, 0.0], serde_json::json!({}));
        store.insert("doc2", vec![0.0, 1.0, 0.0], serde_json::json!({}));
        store.insert("doc3", vec![0.9, 0.1, 0.0], serde_json::json!({}));

        let results = store.search(&[1.0, 0.0, 0.0], 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0.text, "doc1"); // most similar
    }
}
