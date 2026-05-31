use crate::error::AiError;
use sha2::{Digest, Sha256};

pub trait EmbeddingModel: Send + Sync {
    fn embed_text(&self, text: &str) -> Result<Vec<f32>, AiError>;
    fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, AiError>;
    fn dimension(&self) -> usize;
    fn model_name(&self) -> &str;
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
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

pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return f32::INFINITY;
    }
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f32>()
        .sqrt()
}

pub struct MockEmbeddingModel {
    dim: usize,
}

impl MockEmbeddingModel {
    pub fn new(dim: usize) -> Self {
        Self { dim }
    }
}

impl EmbeddingModel for MockEmbeddingModel {
    fn embed_text(&self, text: &str) -> Result<Vec<f32>, AiError> {
        let hash = Sha256::digest(text.as_bytes());
        let words: Vec<&str> = text.split_whitespace().collect();
        let word_count = words.len().max(1);

        let mut embedding = Vec::with_capacity(self.dim);
        for i in 0..self.dim {
            let hash_byte = hash[i % 32] as f32;
            let base = (hash_byte / 127.5) - 1.0;

            let word_idx = i % word_count;
            let word = words[word_idx];
            let word_sum: f32 = word.bytes().map(|b| b as f32).sum::<f32>();
            let word_norm = (word_sum / 255.0) * 0.1;

            embedding.push(base * 0.5 + word_norm * 0.5);
        }
        Ok(embedding)
    }

    fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, AiError> {
        texts.iter().map(|t| self.embed_text(t)).collect()
    }

    fn dimension(&self) -> usize {
        self.dim
    }

    fn model_name(&self) -> &str {
        "mock-embedding"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_embedding_deterministic() {
        let model = MockEmbeddingModel::new(64);
        let e1 = model.embed_text("hello world").unwrap();
        let e2 = model.embed_text("hello world").unwrap();
        assert_eq!(e1, e2);
        assert_eq!(e1.len(), 64);
    }

    #[test]
    fn test_mock_embedding_different_inputs_differ() {
        let model = MockEmbeddingModel::new(64);
        let e1 = model.embed_text("hello").unwrap();
        let e2 = model.embed_text("goodbye").unwrap();
        assert_ne!(e1, e2);
    }

    #[test]
    fn test_cosine_similarity_identical_vectors() {
        let v = vec![1.0, 2.0, 3.0, 4.0];
        let sim = cosine_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal_vectors() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_opposite_vectors() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![-1.0, -2.0, -3.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn test_batch_embedding() {
        let model = MockEmbeddingModel::new(64);
        let texts = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let results = model.embed_batch(&texts).unwrap();
        assert_eq!(results.len(), 3);
        for r in &results {
            assert_eq!(r.len(), 64);
        }
    }

    #[test]
    fn test_dimension_mismatch_cosine() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &b);
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn test_euclidean_distance_same_vector() {
        let v = vec![1.0, 2.0, 3.0];
        let d = euclidean_distance(&v, &v);
        assert!((d - 0.0).abs() < 1e-6);
    }
}
