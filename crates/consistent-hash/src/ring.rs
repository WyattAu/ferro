use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::key::{hash_key, hash_to_position};

/// Default number of virtual nodes per physical node.
const DEFAULT_VIRTUAL_NODES: u32 = 150;

/// Position of a node on the hash ring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RingEntry {
    pub position: u64,
    pub node_id: String,
    pub virtual_node_index: u32,
}

/// A consistent hash ring with virtual nodes support.
///
/// Each physical node is mapped to multiple virtual nodes on the ring
/// to ensure even distribution. Keys are mapped to the nearest node
/// in clockwise order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashRing<T: Clone> {
    nodes: BTreeMap<u64, T>,
    virtual_nodes: u32,
    physical_nodes: Vec<String>,
}

impl<T: Clone> HashRing<T> {
    /// Create a new empty hash ring.
    pub fn new(virtual_nodes: u32) -> Self {
        Self {
            nodes: BTreeMap::new(),
            virtual_nodes,
            physical_nodes: Vec::new(),
        }
    }

    /// Create a new hash ring with default virtual node count (150).
    pub fn with_defaults() -> Self {
        Self::new(DEFAULT_VIRTUAL_NODES)
    }

    /// Add a node to the ring.
    ///
    /// The node is mapped to `virtual_nodes` positions on the ring.
    /// If a weight is provided, it affects the number of virtual nodes (weight * virtual_nodes).
    pub fn add_node(&mut self, node_id: String, weight: Option<f64>, metadata: T) {
        let count = if let Some(w) = weight {
            (self.virtual_nodes as f64 * w).max(1.0) as u32
        } else {
            self.virtual_nodes
        };

        for i in 0..count {
            let virtual_key = format!("{}#{}", node_id, i);
            let hash = hash_key(virtual_key.as_bytes());
            let position = hash_to_position(&hash);
            self.nodes.insert(position, metadata.clone());
        }

        if !self.physical_nodes.contains(&node_id) {
            self.physical_nodes.push(node_id);
        }
    }

    /// Remove a node and all its virtual nodes from the ring.
    pub fn remove_node(&mut self, node_id: &str) -> Option<T> {
        let count = self.virtual_nodes;
        let mut removed = None;

        for i in 0..count {
            let virtual_key = format!("{}#{}", node_id, i);
            let hash = hash_key(virtual_key.as_bytes());
            let position = hash_to_position(&hash);
            if let Some(removed_value) = self.nodes.remove(&position)
                && removed.is_none()
            {
                removed = Some(removed_value);
            }
        }

        self.physical_nodes.retain(|id| id != node_id);
        removed
    }

    /// Find the node responsible for a given key.
    ///
    /// Returns the metadata of the node whose virtual node position is
    /// closest to the key's position in clockwise order.
    pub fn get_node(&self, key: &[u8]) -> Option<(&T, String)> {
        if self.nodes.is_empty() {
            return None;
        }

        let hash = hash_key(key);
        let position = hash_to_position(&hash);

        // Find the first node with position >= key position (clockwise)
        let result = self
            .nodes
            .range(position..)
            .next()
            .or_else(|| self.nodes.iter().next());

        result.map(|(_pos, metadata)| (metadata, self.find_node_id_for_position(*_pos)))
    }

    /// Get N distinct nodes for replication of a given key.
    ///
    /// Returns up to `replicas` distinct physical nodes, skipping virtual
    /// nodes of the same physical node.
    pub fn get_nodes(&self, key: &[u8], replicas: usize) -> Vec<(&T, String)> {
        if self.nodes.is_empty() || replicas == 0 {
            return Vec::new();
        }

        let hash = hash_key(key);
        let position = hash_to_position(&hash);
        let mut result = Vec::new();
        let mut seen_nodes = std::collections::HashSet::new();

        // Walk clockwise from the key's position
        for (_pos, metadata) in self.nodes.range(position..) {
            let node_id = self.find_node_id_for_position(*_pos);
            if seen_nodes.insert(node_id.clone()) {
                result.push((metadata, node_id));
                if result.len() >= replicas {
                    break;
                }
            }
        }

        // Wrap around if we haven't found enough
        if result.len() < replicas {
            for (_pos, metadata) in self.nodes.iter() {
                let node_id = self.find_node_id_for_position(*_pos);
                if seen_nodes.insert(node_id.clone()) {
                    result.push((metadata, node_id));
                    if result.len() >= replicas {
                        break;
                    }
                }
            }
        }

        result
    }

    /// Get all physical nodes in the ring.
    pub fn physical_nodes(&self) -> &[String] {
        &self.physical_nodes
    }

    /// Get the number of virtual nodes per physical node.
    pub fn virtual_nodes_per_physical(&self) -> u32 {
        self.virtual_nodes
    }

    /// Get the total number of virtual nodes on the ring.
    pub fn total_virtual_nodes(&self) -> usize {
        self.nodes.len()
    }

    /// Check if the ring is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Get all ring entries (for diagnostics/export).
    pub fn entries(&self) -> Vec<RingEntry> {
        self.nodes
            .iter()
            .enumerate()
            .map(|(i, (position, _))| RingEntry {
                position: *position,
                node_id: self.find_node_id_for_position(*position),
                virtual_node_index: i as u32 % self.virtual_nodes,
            })
            .collect()
    }

    fn find_node_id_for_position(&self, position: u64) -> String {
        for i in 0..self.virtual_nodes {
            let candidates: Vec<&String> = self.physical_nodes.iter().collect();
            for physical_id in &candidates {
                let virtual_key = format!("{}#{}", physical_id, i);
                let hash = hash_key(virtual_key.as_bytes());
                let pos = hash_to_position(&hash);
                if pos == position {
                    return (*physical_id).clone();
                }
            }
        }
        "unknown".to_string()
    }
}

impl<T: Clone> Default for HashRing<T> {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct NodeMeta {
        address: String,
    }

    fn meta(addr: &str) -> NodeMeta {
        NodeMeta {
            address: addr.to_string(),
        }
    }

    #[test]
    fn test_empty_ring_returns_none() {
        let ring: HashRing<NodeMeta> = HashRing::new(100);
        assert!(ring.get_node(b"key").is_none());
        assert!(ring.get_nodes(b"key", 3).is_empty());
    }

    #[test]
    fn test_add_node_and_get() {
        let mut ring = HashRing::new(150);
        ring.add_node("node-a".to_string(), None, meta("127.0.0.1:8001"));

        let (metadata, node_id) = ring.get_node(b"some-key").unwrap();
        assert_eq!(node_id, "node-a");
        assert_eq!(metadata.address, "127.0.0.1:8001");
    }

    #[test]
    fn test_multiple_nodes_distribution() {
        let mut ring = HashRing::new(150);
        ring.add_node("node-a".to_string(), None, meta("a"));
        ring.add_node("node-b".to_string(), None, meta("b"));
        ring.add_node("node-c".to_string(), None, meta("c"));

        let mut counts = std::collections::HashMap::new();
        for i in 0..1000 {
            let key = format!("key-{}", i);
            let (_, node_id) = ring.get_node(key.as_bytes()).unwrap();
            *counts.entry(node_id).or_insert(0) += 1;
        }

        // With 3 nodes and 1000 keys, each node should get roughly 333 keys
        for count in counts.values() {
            assert!(*count > 200, "Node got too few keys: {}", count);
            assert!(*count < 500, "Node got too many keys: {}", count);
        }
        assert_eq!(counts.len(), 3);
    }

    #[test]
    fn test_remove_node() {
        let mut ring = HashRing::new(150);
        ring.add_node("node-a".to_string(), None, meta("a"));
        ring.add_node("node-b".to_string(), None, meta("b"));

        ring.remove_node("node-a");
        assert_eq!(ring.physical_nodes().len(), 1);

        let (_, node_id) = ring.get_node(b"key").unwrap();
        assert_eq!(node_id, "node-b");
    }

    #[test]
    fn test_get_multiple_replicas() {
        let mut ring = HashRing::new(150);
        ring.add_node("node-a".to_string(), None, meta("a"));
        ring.add_node("node-b".to_string(), None, meta("b"));
        ring.add_node("node-c".to_string(), None, meta("c"));

        let replicas = ring.get_nodes(b"key", 2);
        assert_eq!(replicas.len(), 2);

        let ids: Vec<&str> = replicas.iter().map(|(_, id)| id.as_str()).collect();
        assert!(ids.contains(&"node-a") || ids.contains(&"node-b") || ids.contains(&"node-c"));
    }

    #[test]
    fn test_replicas_are_distinct_physical_nodes() {
        let mut ring = HashRing::new(150);
        ring.add_node("node-a".to_string(), None, meta("a"));
        ring.add_node("node-b".to_string(), None, meta("b"));
        ring.add_node("node-c".to_string(), None, meta("c"));

        let replicas = ring.get_nodes(b"test-key", 3);
        assert_eq!(replicas.len(), 3);

        let mut ids: Vec<&str> = replicas.iter().map(|(_, id)| id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 3, "Replicas should be distinct physical nodes");
    }

    #[test]
    fn test_add_node_with_weight() {
        let mut ring = HashRing::new(100);
        ring.add_node("node-a".to_string(), Some(2.0), meta("a"));
        ring.add_node("node-b".to_string(), Some(1.0), meta("b"));

        assert!(ring.total_virtual_nodes() > 200);
    }

    #[test]
    fn test_consistent_hashing_stability() {
        let mut ring = HashRing::new(150);
        ring.add_node("node-a".to_string(), None, meta("a"));
        ring.add_node("node-b".to_string(), None, meta("b"));

        let key1 = b"my-file-path";
        let (_, first_node) = ring.get_node(key1).unwrap();

        // Add node-c without removing existing
        ring.add_node("node-c".to_string(), None, meta("c"));

        let (_, second_node) = ring.get_node(key1).unwrap();
        assert_eq!(first_node, second_node);
    }

    #[test]
    fn test_entries_count() {
        let mut ring = HashRing::new(100);
        ring.add_node("node-a".to_string(), None, meta("a"));
        ring.add_node("node-b".to_string(), None, meta("b"));

        let entries = ring.entries();
        assert_eq!(entries.len(), 200);
    }

    #[test]
    fn test_is_empty() {
        let ring: HashRing<NodeMeta> = HashRing::new(100);
        assert!(ring.is_empty());

        let mut ring = ring;
        ring.add_node("node-a".to_string(), None, meta("a"));
        assert!(!ring.is_empty());
    }
}
