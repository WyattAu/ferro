use std::collections::HashMap;

use ferro_consistent_hash::HashRing;

use crate::policy::BackendId;

/// A backend that uses consistent hashing to distribute keys across nodes.
///
/// Each backend node is assigned a weight and mapped onto the hash ring.
/// Keys are deterministically placed based on their hash position.
#[derive(Debug, Clone)]
pub struct ConsistentHashBackend {
    ring: HashRing<BackendId>,
    node_addresses: HashMap<String, BackendId>,
}

impl ConsistentHashBackend {
    /// Create a new consistent hash backend with default virtual nodes (150).
    pub fn new() -> Self {
        Self {
            ring: HashRing::with_defaults(),
            node_addresses: HashMap::new(),
        }
    }

    /// Create a new consistent hash backend with custom virtual node count.
    pub fn with_virtual_nodes(virtual_nodes: u32) -> Self {
        Self {
            ring: HashRing::new(virtual_nodes),
            node_addresses: HashMap::new(),
        }
    }

    /// Add a backend node with an optional weight.
    ///
    /// Higher weight means more virtual nodes on the ring, receiving
    /// proportionally more traffic.
    pub fn add_node(&mut self, node_id: String, backend_id: BackendId, weight: Option<f64>) {
        self.ring.add_node(node_id.clone(), weight, backend_id.clone());
        self.node_addresses.insert(node_id, backend_id);
    }

    /// Remove a backend node from the ring.
    pub fn remove_node(&mut self, node_id: &str) -> Option<BackendId> {
        self.node_addresses.remove(node_id);
        self.ring.remove_node(node_id)
    }

    /// Route a key to its assigned backend.
    pub fn route(&self, key: &[u8]) -> Option<&BackendId> {
        self.ring.get_node(key).map(|(id, _)| id)
    }

    /// Get N distinct backends for replication of a key.
    pub fn route_replicated(&self, key: &[u8], replicas: usize) -> Vec<&BackendId> {
        self.ring
            .get_nodes(key, replicas)
            .into_iter()
            .map(|(id, _)| id)
            .collect()
    }

    /// Get the node ID that a key maps to.
    pub fn get_node_id(&self, key: &[u8]) -> Option<String> {
        self.ring.get_node(key).map(|(_, id)| id)
    }

    /// Get the number of physical nodes.
    pub fn node_count(&self) -> usize {
        self.ring.physical_nodes().len()
    }

    /// Check if the backend is empty.
    pub fn is_empty(&self) -> bool {
        self.ring.is_empty()
    }
}

impl Default for ConsistentHashBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consistent_hash_backend_basic() {
        let mut backend = ConsistentHashBackend::new();
        backend.add_node("s3-1".to_string(), BackendId::S3, None);
        backend.add_node("gcs-1".to_string(), BackendId::Gcs, None);

        let id = backend.route(b"my-file.txt").unwrap();
        assert!(*id == BackendId::S3 || *id == BackendId::Gcs);
    }

    #[test]
    fn test_consistent_hash_backend_replication() {
        let mut backend = ConsistentHashBackend::new();
        backend.add_node("s3-1".to_string(), BackendId::S3, None);
        backend.add_node("gcs-1".to_string(), BackendId::Gcs, None);
        backend.add_node("azure-1".to_string(), BackendId::AzureBlob, None);

        let replicas = backend.route_replicated(b"important-file", 2);
        assert_eq!(replicas.len(), 2);
    }

    #[test]
    fn test_consistent_hash_backend_weighted() {
        let mut backend = ConsistentHashBackend::new();
        backend.add_node("s3-1".to_string(), BackendId::S3, Some(3.0));
        backend.add_node("local-1".to_string(), BackendId::Local, Some(1.0));

        // Run many keys, S3 should get more due to higher weight
        let mut counts = HashMap::new();
        for i in 0..1000 {
            let key = format!("file-{}", i);
            let id = backend.route(key.as_bytes()).unwrap();
            *counts.entry(id.clone()).or_insert(0) += 1;
        }

        let s3_count = counts.get(&BackendId::S3).unwrap_or(&0);
        let local_count = counts.get(&BackendId::Local).unwrap_or(&0);
        assert!(s3_count > local_count, "S3 with weight 3.0 should get more traffic");
    }

    #[test]
    fn test_consistent_hash_backend_remove_node() {
        let mut backend = ConsistentHashBackend::new();
        backend.add_node("s3-1".to_string(), BackendId::S3, None);
        backend.add_node("gcs-1".to_string(), BackendId::Gcs, None);

        let before = backend.route(b"key").unwrap().clone();
        backend.remove_node("gcs-1");

        let after = backend.route(b"key").unwrap().clone();
        assert_eq!(before, after);
        assert_eq!(backend.node_count(), 1);
    }

    #[test]
    fn test_consistent_hash_backend_empty() {
        let backend = ConsistentHashBackend::new();
        assert!(backend.is_empty());
        assert!(backend.route(b"key").is_none());
    }

    #[test]
    fn test_consistent_hash_backend_node_id() {
        let mut backend = ConsistentHashBackend::new();
        backend.add_node("s3-1".to_string(), BackendId::S3, None);

        let node_id = backend.get_node_id(b"file.txt").unwrap();
        assert_eq!(node_id, "s3-1");
    }
}
