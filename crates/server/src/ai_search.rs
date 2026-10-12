use std::sync::Arc;

use ferro_ai::embedding::{EmbeddingModel, MockEmbeddingModel};
use ferro_ai::semantic::SemanticIndex;
use serde_json::Value;
use thiserror::Error;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum AiSearchError {
    #[error("embedding failed: {0}")]
    EmbeddingFailed(String),
    #[error("semantic index not available")]
    IndexUnavailable,
}

#[derive(Debug, Clone)]
pub struct SemanticSearchResult {
    pub id: String,
    pub path: String,
    pub score: f32,
    pub metadata: Value,
}

#[derive(Debug)]
pub struct AiSearchConfig {
    pub embedding_dim: usize,
    pub min_similarity: f32,
}

impl Default for AiSearchConfig {
    fn default() -> Self {
        Self {
            embedding_dim: 128,
            min_similarity: 0.3,
        }
    }
}

pub struct AiSearchBridge {
    semantic_index: Arc<SemanticIndex>,
    config: AiSearchConfig,
}

impl AiSearchBridge {
    pub fn new(config: AiSearchConfig) -> Self {
        let model: Box<dyn EmbeddingModel> = Box::new(MockEmbeddingModel::new(config.embedding_dim));
        let semantic_index = Arc::new(SemanticIndex::new(model));
        Self { semantic_index, config }
    }

    pub fn with_model(model: Box<dyn EmbeddingModel>, config: AiSearchConfig) -> Self {
        let semantic_index = Arc::new(SemanticIndex::new(model));
        Self { semantic_index, config }
    }

    pub fn index_document(&self, id: &str, path: &str, content: &str, owner: &str) -> Result<(), AiSearchError> {
        self.semantic_index
            .add(id, path, content, serde_json::json!({ "owner": owner }))
            .map_err(|e| AiSearchError::EmbeddingFailed(e.to_string()))
    }

    pub fn remove_document(&self, id: &str) -> bool {
        self.semantic_index.remove(id)
    }

    pub fn semantic_search(
        &self,
        query: &str,
        limit: usize,
        min_similarity: Option<f32>,
        owner: Option<&str>,
    ) -> Result<Vec<SemanticSearchResult>, AiSearchError> {
        let similarity = min_similarity.unwrap_or(self.config.min_similarity);
        let results = self
            .semantic_index
            .search(query, limit, similarity, owner)
            .map_err(|e| AiSearchError::EmbeddingFailed(e.to_string()))?;
        Ok(results
            .into_iter()
            .map(|r| SemanticSearchResult {
                id: r.id,
                path: r.path,
                score: r.score,
                metadata: r.metadata,
            })
            .collect())
    }

    pub fn is_available(&self) -> bool {
        true
    }
}

#[async_trait::async_trait]
impl ferro_server_api_core::AiSearchBridgeTrait for AiSearchBridge {
    fn is_available(&self) -> bool {
        true
    }

    fn semantic_search(
        &self,
        query: &str,
        limit: usize,
        min_similarity: Option<f32>,
        owner: Option<&str>,
    ) -> Result<Vec<ferro_server_api_core::SemanticSearchResult>, String> {
        let results = self
            .semantic_search(query, limit, min_similarity, owner)
            .map_err(|e| e.to_string())?;
        Ok(results
            .into_iter()
            .map(|r| ferro_server_api_core::SemanticSearchResult {
                id: r.id,
                path: r.path,
                score: r.score,
                metadata: r.metadata,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferro_ai::embedding::MockEmbeddingModel;

    fn make_bridge() -> AiSearchBridge {
        AiSearchBridge::new(AiSearchConfig::default())
    }

    #[test]
    fn test_index_and_search() {
        let bridge = make_bridge();
        bridge
            .index_document("doc1", "/docs/report.pdf", "quarterly financial report", "user1")
            .unwrap();
        let results = bridge
            .semantic_search("quarterly report", 5, Some(0.0), Some("user1"))
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "doc1");
        assert!(results[0].score > 0.0);
    }

    #[test]
    fn test_search_returns_empty_for_no_match() {
        let bridge = make_bridge();
        let results = bridge
            .semantic_search("nonexistent", 5, Some(0.99), Some("user1"))
            .unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_remove_document() {
        let bridge = make_bridge();
        bridge.index_document("doc1", "/a.txt", "hello world", "user1").unwrap();
        assert!(bridge.remove_document("doc1"));
        let results = bridge.semantic_search("hello", 5, Some(0.0), Some("user1")).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_is_available() {
        let bridge = make_bridge();
        assert!(bridge.is_available());
    }

    #[test]
    fn test_with_model() {
        let model: Box<dyn EmbeddingModel> = Box::new(MockEmbeddingModel::new(64));
        let bridge = AiSearchBridge::with_model(model, AiSearchConfig::default());
        bridge
            .index_document("1", "/test.txt", "test content", "user1")
            .unwrap();
        let results = bridge
            .semantic_search("test content", 5, Some(0.0), Some("user1"))
            .unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_custom_config() {
        let config = AiSearchConfig {
            embedding_dim: 256,
            min_similarity: 0.5,
        };
        let bridge = AiSearchBridge::new(config);
        bridge.index_document("1", "/a.txt", "some text here", "user1").unwrap();
        let results = bridge
            .semantic_search("some text here", 5, Some(0.99), Some("user1"))
            .unwrap();
        assert_eq!(results.len(), 1);
    }

    /// TV-SEARCH-001 semantic-arm regression: another principal's documents are
    /// invisible; documents without an owner are hidden when scoped (hide-by-default).
    #[test]
    fn test_scoped_semantic_search_hides_other_owners() {
        let bridge = make_bridge();
        bridge
            .index_document("alice-doc", "/alice/secret.pdf", "quarterly financial report", "alice")
            .unwrap();
        bridge
            .index_document("bob-doc", "/bob/notes.txt", "quarterly financial notes", "bob")
            .unwrap();

        let alice_view = bridge
            .semantic_search("quarterly financial", 10, Some(0.0), Some("alice"))
            .unwrap();
        assert_eq!(alice_view.len(), 1);
        assert_eq!(alice_view[0].id, "alice-doc");

        let bob_view = bridge
            .semantic_search("quarterly financial", 10, Some(0.0), Some("bob"))
            .unwrap();
        assert_eq!(bob_view.len(), 1);
        assert_eq!(bob_view[0].id, "bob-doc");

        // Legacy entries indexed without an owner are hidden when scoped.
        bridge
            .semantic_index
            .add(
                "legacy-doc",
                "/legacy.txt",
                "quarterly financial archive",
                serde_json::Value::Null,
            )
            .unwrap();
        let scoped = bridge
            .semantic_search("quarterly financial", 10, Some(0.0), Some("alice"))
            .unwrap();
        assert!(scoped.iter().all(|r| r.id != "legacy-doc"));

        // Unscoped (admin) still sees everything.
        let admin_view = bridge
            .semantic_search("quarterly financial", 10, Some(0.0), None)
            .unwrap();
        assert_eq!(admin_view.len(), 3);
    }
}
