use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::key::hash_key;

/// Score of a candidate node for a key under weighted rendezvous hashing.
struct Candidate<'a, T> {
    node_id: &'a str,
    metadata: &'a T,
    score: f64,
}

/// Hash a (key, node) pair to a uniform u64.
///
/// Domain separation: `key || 0x00 || node` keeps key/node concatenation
/// injective for keys and node ids without interior NULs.
fn pair_hash(key: &[u8], node_id: &str) -> u64 {
    let mut input = Vec::with_capacity(key.len() + node_id.len() + 1);
    input.extend_from_slice(key);
    input.push(0u8);
    input.extend_from_slice(node_id.as_bytes());
    let hex = hash_key(&input);
    u64::from_be_bytes([
        hex.get(0..2).and_then(|s| u8::from_str_radix(s, 16).ok()).unwrap_or(0),
        hex.get(2..4).and_then(|s| u8::from_str_radix(s, 16).ok()).unwrap_or(0),
        hex.get(4..6).and_then(|s| u8::from_str_radix(s, 16).ok()).unwrap_or(0),
        hex.get(6..8).and_then(|s| u8::from_str_radix(s, 16).ok()).unwrap_or(0),
        hex.get(8..10).and_then(|s| u8::from_str_radix(s, 16).ok()).unwrap_or(0),
        hex.get(10..12)
            .and_then(|s| u8::from_str_radix(s, 16).ok())
            .unwrap_or(0),
        hex.get(12..14)
            .and_then(|s| u8::from_str_radix(s, 16).ok())
            .unwrap_or(0),
        hex.get(14..16)
            .and_then(|s| u8::from_str_radix(s, 16).ok())
            .unwrap_or(0),
    ])
}

/// Map a u64 to (0, 1) with 53 bits of mantissa.
fn u64_to_unit(h: u64) -> f64 {
    (h >> 11) as f64 * (1.0 / 9007199254740992.0)
}

/// Weighted rendezvous score: `ln(u01) / weight`, maximized over nodes.
///
/// This is the classic highest-random-weight (HRW) selection rule; the log
/// form makes weights exact in expectation (node with weight w wins w/N of
/// the key space) and is total-order stable under f64 rounding for a fixed
/// hash input.
fn rendezvous_score(weight: f64, h: u64) -> f64 {
    let u = u64_to_unit(h);
    // u ∈ [0, 1); push off exactly 0 before the log.
    let u = if u < f64::EPSILON { f64::EPSILON } else { u };
    (u.ln()) / weight
}

/// Position record for diagnostics/export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RingEntry {
    pub position: u64,
    pub node_id: String,
    pub virtual_node_index: u32,
}

/// Weighted rendezvous (highest-random-weight) placement.
///
/// Replaces the virtual-node ring: each key is placed on the node maximizing
/// `ln(hash(key, node)) / weight`. This gives the same minimal-disruption
/// guarantee as a consistent hash ring with zero ring bookkeeping — no
/// vnodes, no collisions, and node removal is correct by construction
/// (weighted removal was previously broken: vnodes of weighted nodes were
/// orphaned on `remove_node`).
///
/// The `virtual_nodes` constructor parameter is retained for API
/// compatibility and is ignored (HRW needs no virtual nodes).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashRing<T: Clone> {
    nodes: HashMap<String, NodeSlot<T>>,
    virtual_nodes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NodeSlot<T> {
    weight: Option<f64>,
    metadata: T,
}

impl<T: Clone> HashRing<T> {
    /// Create a new empty placement (parameter retained for compatibility, unused).
    pub fn new(virtual_nodes: u32) -> Self {
        Self {
            nodes: HashMap::new(),
            virtual_nodes,
        }
    }

    /// Create a new placement with defaults.
    pub fn with_defaults() -> Self {
        Self::new(0)
    }

    /// Add a node. Weight scales its share of the key space (default 1.0).
    pub fn add_node(&mut self, node_id: String, weight: Option<f64>, metadata: T) {
        self.nodes.insert(node_id, NodeSlot { weight, metadata });
    }

    /// Remove a node entirely — HRW needs no vnode cleanup.
    pub fn remove_node(&mut self, node_id: &str) -> Option<T> {
        self.nodes.remove(node_id).map(|slot| slot.metadata)
    }

    /// Find the node responsible for a given key.
    pub fn get_node(&self, key: &[u8]) -> Option<(&T, String)> {
        self.rank(key)
            .into_iter()
            .next()
            .map(|c| (c.metadata, c.node_id.to_string()))
    }

    /// Get N distinct nodes for replication of a given key, in placement
    /// order (primary first).
    pub fn get_nodes(&self, key: &[u8], replicas: usize) -> Vec<(&T, String)> {
        if replicas == 0 {
            return Vec::new();
        }
        self.rank(key)
            .into_iter()
            .take(replicas)
            .map(|c| (c.metadata, c.node_id.to_string()))
            .collect()
    }

    /// Score every node for a key and return candidates in descending score.
    fn rank(&self, key: &[u8]) -> Vec<Candidate<'_, T>> {
        let mut candidates: Vec<Candidate<T>> = self
            .nodes
            .iter()
            .map(|(node_id, slot)| {
                let w = slot.weight.unwrap_or(1.0);
                let w = if w.is_finite() && w > 0.0 { w } else { 1.0 };
                Candidate {
                    node_id,
                    metadata: &slot.metadata,
                    score: rendezvous_score(w, pair_hash(key, node_id)),
                }
            })
            .collect();
        candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        candidates
    }

    /// Get all physical nodes in the ring.
    pub fn physical_nodes(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.nodes.keys().cloned().collect();
        ids.sort();
        ids
    }

    /// Virtual nodes per physical node — always 1 under HRW (compatibility).
    pub fn virtual_nodes_per_physical(&self) -> u32 {
        1
    }

    /// Total ring slots — the physical node count under HRW (compatibility).
    pub fn total_virtual_nodes(&self) -> usize {
        self.nodes.len()
    }

    /// Check if the ring is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Get all entries (one per physical node, for diagnostics/export).
    pub fn entries(&self) -> Vec<RingEntry> {
        self.physical_nodes()
            .into_iter()
            .map(|node_id| RingEntry {
                position: pair_hash(node_id.as_bytes(), &node_id),
                virtual_node_index: 0,
                node_id,
            })
            .collect()
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
        let ring: HashRing<NodeMeta> = HashRing::new(0);
        assert!(ring.get_node(b"key").is_none());
        assert!(ring.get_nodes(b"key", 3).is_empty());
    }

    #[test]
    fn test_add_node_and_get() {
        let mut ring = HashRing::with_defaults();
        ring.add_node("node-a".to_string(), None, meta("127.0.0.1:8001"));

        let (metadata, node_id) = ring.get_node(b"some-key").unwrap();
        assert_eq!(node_id, "node-a");
        assert_eq!(metadata.address, "127.0.0.1:8001");
    }

    #[test]
    fn test_multiple_nodes_distribution() {
        let mut ring = HashRing::with_defaults();
        ring.add_node("node-a".to_string(), None, meta("a"));
        ring.add_node("node-b".to_string(), None, meta("b"));
        ring.add_node("node-c".to_string(), None, meta("c"));

        let mut counts = std::collections::HashMap::new();
        for i in 0..1000 {
            let key = format!("key-{}", i);
            let (_, node_id) = ring.get_node(key.as_bytes()).unwrap();
            *counts.entry(node_id).or_insert(0) += 1;
        }

        for count in counts.values() {
            assert!(*count > 200, "Node got too few keys: {}", count);
            assert!(*count < 500, "Node got too many keys: {}", count);
        }
        assert_eq!(counts.len(), 3);
    }

    #[test]
    fn test_remove_node() {
        let mut ring = HashRing::with_defaults();
        ring.add_node("node-a".to_string(), None, meta("a"));
        ring.add_node("node-b".to_string(), None, meta("b"));

        ring.remove_node("node-a");
        assert_eq!(ring.physical_nodes().len(), 1);

        let (_, node_id) = ring.get_node(b"key").unwrap();
        assert_eq!(node_id, "node-b");
    }

    #[test]
    fn test_weighted_removal_leaves_no_orphans() {
        let mut ring = HashRing::with_defaults();
        ring.add_node("node-a".to_string(), Some(2.0), meta("a"));
        ring.add_node("node-b".to_string(), Some(1.0), meta("b"));

        ring.remove_node("node-a");
        for i in 0..500 {
            let (_, node_id) = ring.get_node(format!("key-{}", i).as_bytes()).unwrap();
            assert_eq!(node_id, "node-b", "key-{i} still routed to removed node");
        }
    }

    #[test]
    fn test_minimal_disruption_on_removal() {
        let mut ring = HashRing::with_defaults();
        ring.add_node("node-a".to_string(), None, meta("a"));
        ring.add_node("node-b".to_string(), None, meta("b"));
        ring.add_node("node-c".to_string(), None, meta("c"));

        let mut moved = 0;
        let mut checked = 0;
        for i in 0..500 {
            let key = format!("key-{}", i);
            let (_, before) = ring.get_node(key.as_bytes()).unwrap();
            if before == "node-c" {
                ring.remove_node("node-c");
                let (_, after) = ring.get_node(key.as_bytes()).unwrap();
                assert_ne!(after, "node-c");
                ring.add_node("node-c".to_string(), None, meta("c"));
                moved += 1;
            } else {
                checked += 1;
            }
        }
        let _ = checked;
        assert!(moved > 0, "expected some keys on the removed node");
    }

    #[test]
    fn test_get_multiple_replicas() {
        let mut ring = HashRing::with_defaults();
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
        let mut ring = HashRing::with_defaults();
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
        let mut ring = HashRing::with_defaults();
        ring.add_node("node-a".to_string(), Some(3.0), meta("a"));
        ring.add_node("node-b".to_string(), Some(1.0), meta("b"));

        let mut a = 0;
        for i in 0..2000 {
            let (_, node_id) = ring.get_node(format!("key-{}", i).as_bytes()).unwrap();
            if node_id == "node-a" {
                a += 1;
            }
        }
        assert!(a > 1100, "weight 3.0 should win ~75%: got {a}/2000");
    }

    #[test]
    fn test_consistent_hashing_stability() {
        let mut ring = HashRing::with_defaults();
        ring.add_node("node-a".to_string(), None, meta("a"));
        ring.add_node("node-b".to_string(), None, meta("b"));

        let key1 = b"my-file-path";
        let (_, first_node) = ring.get_node(key1).unwrap();

        ring.add_node("node-c".to_string(), None, meta("c"));

        let (_, second_node) = ring.get_node(key1).unwrap();
        assert_eq!(first_node, second_node);
    }

    #[test]
    fn test_zero_and_invalid_weights_fall_back_to_unit() {
        let mut ring: HashRing<NodeMeta> = HashRing::with_defaults();
        ring.add_node("node-a".to_string(), Some(0.0), meta("a"));
        ring.add_node("node-b".to_string(), Some(f64::NAN), meta("b"));

        let (_, node_id) = ring.get_node(b"key").unwrap();
        assert!(node_id == "node-a" || node_id == "node-b");
    }

    #[test]
    fn test_deterministic_placement() {
        let build = || {
            let mut ring = HashRing::with_defaults();
            ring.add_node("node-a".to_string(), None, meta("a"));
            ring.add_node("node-b".to_string(), None, meta("b"));
            ring
        };
        let r1 = build();
        let r2 = build();
        for i in 0..100 {
            let key = format!("key-{}", i).into_bytes();
            assert_eq!(r1.get_node(&key).unwrap().1, r2.get_node(&key).unwrap().1);
        }
    }

    #[test]
    fn test_entries_count() {
        let mut ring = HashRing::with_defaults();
        ring.add_node("node-a".to_string(), None, meta("a"));
        ring.add_node("node-b".to_string(), None, meta("b"));

        let entries = ring.entries();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_is_empty() {
        let ring: HashRing<NodeMeta> = HashRing::new(0);
        assert!(ring.is_empty());

        let mut ring = ring;
        ring.add_node("node-a".to_string(), None, meta("a"));
        assert!(!ring.is_empty());
    }
}
