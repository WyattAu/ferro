use crate::embedding::{EmbeddingModel, cosine_similarity};
use crate::error::AiError;
use dashmap::DashMap;
use serde_json::Value;
use sha2::{Digest, Sha256};

pub struct IndexEntry {
    pub id: String,
    pub path: String,
    pub embedding: Vec<f32>,
    pub content_hash: [u8; 32],
    pub metadata: Value,
}

pub struct SearchResult {
    pub id: String,
    pub path: String,
    pub score: f32,
    pub metadata: Value,
}

pub struct SemanticIndex {
    model: Box<dyn EmbeddingModel>,
    entries: DashMap<String, IndexEntry>,
    dimension: usize,
}

impl SemanticIndex {
    pub fn new(model: Box<dyn EmbeddingModel>) -> Self {
        let dim = model.dimension();
        Self {
            model,
            entries: DashMap::new(),
            dimension: dim,
        }
    }

    pub fn add(&self, id: &str, path: &str, content: &str, metadata: Value) -> Result<(), AiError> {
        let embedding = self.model.embed_text(content)?;
        if embedding.len() != self.dimension {
            return Err(AiError::EmbeddingFailed {
                reason: format!(
                    "embedding dimension {} does not match index dimension {}",
                    embedding.len(),
                    self.dimension
                ),
            });
        }
        let hash = Sha256::digest(content.as_bytes());
        let mut content_hash = [0u8; 32];
        content_hash.copy_from_slice(&hash);

        let entry = IndexEntry {
            id: id.to_string(),
            path: path.to_string(),
            embedding,
            content_hash,
            metadata,
        };
        self.entries.insert(id.to_string(), entry);
        Ok(())
    }

    pub fn remove(&self, id: &str) -> bool {
        self.entries.remove(id).is_some()
    }

    pub fn search(&self, query: &str, top_k: usize, min_similarity: f32) -> Result<Vec<SearchResult>, AiError> {
        let query_embedding = self.model.embed_text(query)?;
        if query_embedding.len() != self.dimension {
            return Err(AiError::EmbeddingFailed {
                reason: format!(
                    "query embedding dimension {} does not match index dimension {}",
                    query_embedding.len(),
                    self.dimension
                ),
            });
        }

        let mut scored: Vec<(String, f32)> = self
            .entries
            .iter()
            .map(|entry| {
                let sim = cosine_similarity(&query_embedding, &entry.embedding);
                (entry.id.clone(), sim)
            })
            .filter(|(_, sim)| *sim >= min_similarity)
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);

        let results: Vec<SearchResult> = scored
            .into_iter()
            .filter_map(|(id, score)| {
                self.entries.get(&id).map(|entry| SearchResult {
                    id: entry.id.clone(),
                    path: entry.path.clone(),
                    score,
                    metadata: entry.metadata.clone(),
                })
            })
            .collect();

        Ok(results)
    }

    pub fn reindex(&self, id: &str, content: &str) -> Result<(), AiError> {
        let mut entry = self.entries.get_mut(id).ok_or_else(|| AiError::InvalidInput {
            reason: format!("entry '{}' not found for reindexing", id),
        })?;
        let embedding = self.model.embed_text(content)?;
        let hash = Sha256::digest(content.as_bytes());
        entry.embedding = embedding;
        entry.content_hash.copy_from_slice(&hash);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding::MockEmbeddingModel;

    fn make_index() -> SemanticIndex {
        SemanticIndex::new(Box::new(MockEmbeddingModel::new(64)))
    }

    #[test]
    fn test_add_and_search() {
        let idx = make_index();
        idx.add("doc1", "/docs/report.txt", "quarterly financial report", Value::Null)
            .unwrap();
        let results = idx.search("quarterly report", 5, 0.0).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "doc1");
        assert!(results[0].score > 0.0);
    }

    #[test]
    fn test_search_relevance_ordering() {
        let idx = make_index();
        idx.add("a", "/a.txt", "rust programming language", Value::Null)
            .unwrap();
        idx.add("b", "/b.txt", "cooking recipes pasta", Value::Null).unwrap();
        idx.add("c", "/c.txt", "advanced rust systems programming", Value::Null)
            .unwrap();

        let results = idx.search("rust programming", 5, 0.0).unwrap();
        assert!(results.len() >= 2);
        for i in 1..results.len() {
            assert!(
                results[i - 1].score >= results[i].score,
                "results should be sorted by score descending"
            );
        }
    }

    #[test]
    fn test_min_similarity_threshold() {
        let idx = make_index();
        idx.add("doc1", "/a.txt", "quantum physics", Value::Null).unwrap();
        let results = idx.search("quantum physics", 5, 0.99).unwrap();
        assert_eq!(results.len(), 1);
        let results2 = idx.search("baking cookies", 5, 0.5).unwrap();
        assert_eq!(results2.len(), 0);
    }

    #[test]
    fn test_remove() {
        let idx = make_index();
        idx.add("doc1", "/a.txt", "hello world", Value::Null).unwrap();
        assert!(idx.remove("doc1"));
        assert!(!idx.remove("doc1"));
        let results = idx.search("hello", 5, 0.0).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_reindex() {
        let idx = make_index();
        idx.add("doc1", "/a.txt", "original content here", Value::Null).unwrap();
        let before = idx.search("original content", 1, 0.0).unwrap();
        assert_eq!(before.len(), 1);

        idx.reindex("doc1", "completely different text now").unwrap();
        let after_orig = idx.search("original content", 1, 0.5).unwrap();
        assert_eq!(after_orig.len(), 0);
    }

    #[test]
    fn test_empty_index_search_returns_empty() {
        let idx = make_index();
        let results = idx.search("anything", 5, 0.0).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_multiple_results() {
        let idx = make_index();
        idx.add("d1", "/a.txt", "machine learning", Value::Null).unwrap();
        idx.add("d2", "/b.txt", "deep learning neural networks", Value::Null)
            .unwrap();
        idx.add("d3", "/c.txt", "data science statistics", Value::Null).unwrap();
        let results = idx.search("learning", 10, 0.0).unwrap();
        assert!(results.len() >= 2);
    }
}
