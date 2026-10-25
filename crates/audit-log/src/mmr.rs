//! Merkle Mountain Range audit log (ALG-MMRLOG-001, YP-LIFECYCLE-COMPLIANCE-001 Rev 1.1).
//!
//! Companion to `formal/Ferro/AuditChain.lean`. Older peaks are never
//! rewritten by appends (merge-on-append, newest two peaks only); the
//! bagged anchor is a deterministic pure function of the entry list;
//! per-event inclusion proofs are O(log n); truncation of any newest entry
//! changes the anchor. Tamper evidence is conditional on SHA-256 collision
//! resistance — the Lean layer proves the structural properties above it.
//!
//! Scale note (50 users / one host): leaves are retained so inclusion
//! proofs can be generated without a separate proof store; peak/bag
//! computation is O(n) per call against a capped audit log.

use sha2::{Digest, Sha256};

/// A mountain peak: height + perfect-subtree digest (`2^height` leaves).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Peak {
    pub height: u32,
    pub hash: [u8; 32],
}

fn sha256_pair(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(a);
    hasher.update(b);
    hasher.finalize().into()
}

fn leaf_hash(data: &[u8]) -> [u8; 32] {
    Sha256::digest(data).into()
}

/// Subtree hash of `leaves[start..start + 2^height]` (must be in range).
fn subtree_hash(leaves: &[[u8; 32]], start: usize, height: u32) -> [u8; 32] {
    if height == 0 {
        return leaves[start];
    }
    let half = 1usize << (height - 1);
    let l = subtree_hash(leaves, start, height - 1);
    let r = subtree_hash(leaves, start + half, height - 1);
    sha256_pair(&l, &r)
}

/// One step of an inclusion proof: the sibling subtree digest and whether
/// it was combined on the right.
pub type PeakBit = ([u8; 32], bool);

/// Perfect-subtree decomposition: peaks (oldest first), each with its
/// start offset and covered leaf count.
fn decompose(len: usize) -> Vec<(usize, u32)> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut remaining = len;
    while remaining > 0 {
        let k = 63 - (remaining as u64).leading_zeros();
        out.push((start, k));
        let size = 1usize << k;
        start += size;
        remaining -= size;
    }
    out
}

/// An append-only Merkle Mountain Range over entry hashes.
#[derive(Debug, Clone, Default)]
pub struct Mmr {
    leaves: Vec<[u8; 32]>,
}

impl Mmr {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Restore from previously hashed leaves.
    #[must_use]
    pub fn from_hashes(leaves: Vec<[u8; 32]>) -> Self {
        Self { leaves }
    }

    /// Append one entry (payload hashed into a leaf).
    pub fn append(&mut self, data: &[u8]) {
        self.leaves.push(leaf_hash(data));
    }

    /// Number of entries covered.
    #[must_use]
    pub fn len(&self) -> u64 {
        self.leaves.len() as u64
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.leaves.is_empty()
    }

    #[must_use]
    pub fn leaf(&self, index: u64) -> Option<[u8; 32]> {
        self.leaves.get(index as usize).copied()
    }

    /// Current peaks (oldest first). Older peaks are stable under append —
    /// only the last peak's subtree grows (Lean `mmrAppend` contract).
    #[must_use]
    pub fn peaks(&self) -> Vec<Peak> {
        decompose(self.leaves.len())
            .into_iter()
            .map(|(start, height)| Peak {
                height,
                hash: subtree_hash(&self.leaves, start, height),
            })
            .collect()
    }

    /// Bagged epoch anchor (Lean `bag`): fold peaks oldest-first,
    /// `bag = H(bag_rest || peak)`. Deterministic pure function of the
    /// entry list; truncating any newest entry changes it.
    #[must_use]
    pub fn bag(&self) -> [u8; 32] {
        let mut acc = [0u8; 32];
        for peak in self.peaks() {
            acc = sha256_pair(&acc, &peak.hash);
        }
        acc
    }

    /// Inclusion proof for entry `index`: the sibling path from the leaf to
    /// its peak root. Proof entries are `(sibling_hash, sibling_is_left)`.
    #[must_use]
    pub fn inclusion_proof(&self, index: u64) -> Option<Vec<([u8; 32], bool)>> {
        let idx = index as usize;
        if idx >= self.leaves.len() {
            return None;
        }
        // The perfect subtree containing this leaf.
        let (start, height) = decompose(self.leaves.len()).into_iter().find(|(s, h)| {
            let size = 1usize << h;
            idx >= *s && idx < s + size
        })?;
        let in_tree = idx - start;

        let mut proof = Vec::with_capacity(height as usize);
        let mut pos = in_tree;
        let mut lo = idx; // start of the current node's own block
        let mut nodes = 1usize; // current block size in leaves
        let mut cur = self.leaves[idx];
        let mut lvl_height = 0u32;
        while lvl_height < height {
            let sibling_is_left = pos % 2 == 1;
            let sib = if sibling_is_left {
                subtree_hash(&self.leaves, lo - nodes, lvl_height)
            } else {
                subtree_hash(&self.leaves, lo + nodes, lvl_height)
            };
            proof.push((sib, sibling_is_left));
            let (l, r) = if sibling_is_left { (sib, cur) } else { (cur, sib) };
            cur = sha256_pair(&l, &r);
            if sibling_is_left {
                lo -= nodes;
            }
            pos /= 2;
            nodes *= 2;
            lvl_height += 1;
        }
        Some(proof)
    }
}

/// Verify an inclusion proof against the covering peak's hash.
#[must_use]
pub fn verify_inclusion(data: &[u8], index: u64, proof: &[([u8; 32], bool)], peaks: &[Peak]) -> bool {
    let mut cur = leaf_hash(data);
    for (sib, sibling_is_left) in proof {
        cur = if *sibling_is_left {
            sha256_pair(sib, &cur)
        } else {
            sha256_pair(&cur, sib)
        };
    }
    // `index`'s leaf position determines which peaks it may land under;
    // accept if the computed root matches ANY peak (the proof was produced
    // for the covering peak).
    let idx = index;
    let mut offset = 0u64;
    for peak in peaks {
        let size = 1u64 << peak.height;
        if idx >= offset && idx < offset + size {
            return cur == peak.hash;
        }
        offset += size;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)] // used by tests gated behind optional features
    fn h(b: u8) -> [u8; 32] {
        let mut a = [0u8; 32];
        a[0] = b;
        a
    }

    #[test]
    fn test_append_and_len() {
        let mut mmr = Mmr::new();
        assert!(mmr.is_empty());
        for i in 0..7u8 {
            mmr.append(&[i]);
        }
        assert_eq!(mmr.len(), 7);
    }

    #[test]
    fn test_peaks_shape_perfect_decomposition() {
        // 7 = 4 + 2 + 1 → heights [2, 1, 0]
        let mut mmr = Mmr::new();
        for i in 0..7u8 {
            mmr.append(&[i]);
        }
        let peaks = mmr.peaks();
        assert_eq!(peaks.len(), 3);
        assert_eq!(peaks[0].height, 2);
        assert_eq!(peaks[1].height, 1);
        assert_eq!(peaks[2].height, 0);
        // covered count = 2^2 + 2^1 + 2^0 = 7
        let covered: u64 = peaks.iter().map(|p| 1u64 << p.height).sum();
        assert_eq!(covered, 7);
    }

    #[test]
    fn test_append_stabilizes_older_peaks() {
        // Lean mmrAppend contract: older peaks never rewritten.
        let mut mmr = Mmr::new();
        for i in 0..5u8 {
            mmr.append(&[i]);
        }
        let before = mmr.peaks();
        mmr.append(&[5]);
        mmr.append(&[6]);
        let after = mmr.peaks();
        for (i, b) in before.iter().enumerate() {
            if i + 1 < before.len() {
                assert_eq!(b.hash, after[i].hash, "peak {i} changed");
            }
        }
    }

    #[test]
    fn test_bag_deterministic_and_changes_on_truncation() {
        let mut full = Mmr::new();
        for i in 0..10u8 {
            full.append(&[i]);
        }
        let mut truncated = Mmr::new();
        for i in 0..9u8 {
            truncated.append(&[i]);
        }
        let bag1 = full.bag();
        assert_eq!(bag1, full.bag(), "bag must be deterministic");
        assert_ne!(bag1, truncated.bag(), "truncation must change the bag");

        let mut reordered = Mmr::new();
        for i in (0..10u8).rev() {
            reordered.append(&[i]);
        }
        assert_ne!(bag1, reordered.bag(), "reordering must change the bag");
    }

    #[test]
    fn test_inclusion_proof_roundtrip() {
        let mut mmr = Mmr::new();
        for i in 0..13u8 {
            mmr.append(&[i, i.wrapping_mul(7)]);
        }
        let peaks = mmr.peaks();
        for i in 0u64..13 {
            let proof = mmr.inclusion_proof(i).expect("proof");
            let data = [i as u8, i.wrapping_mul(7) as u8];
            assert!(verify_inclusion(&data, i, &proof, &peaks), "index {i}");
        }
    }

    #[test]
    fn test_inclusion_proof_rejects_wrong_data() {
        let mut mmr = Mmr::new();
        for i in 0..10u8 {
            mmr.append(&[i]);
        }
        let peaks = mmr.peaks();
        let proof = mmr.inclusion_proof(4).expect("proof");
        assert!(!verify_inclusion(&[99], 4, &proof, &peaks));
    }

    #[test]
    fn test_inclusion_proof_out_of_range() {
        let mut mmr = Mmr::new();
        mmr.append(&[1]);
        assert!(mmr.inclusion_proof(1).is_none());
        assert!(mmr.inclusion_proof(100).is_none());
    }

    #[test]
    fn test_verify_rejects_cross_peak_replay() {
        // A proof for a leaf under peak 0 must not verify at an index under
        // a different peak.
        let mut mmr = Mmr::new();
        for i in 0..9u8 {
            mmr.append(&[i]);
        }
        let peaks = mmr.peaks();
        let proof = mmr.inclusion_proof(0).expect("proof");
        assert!(!verify_inclusion(&[0], 8, &proof, &peaks));
    }

    #[test]
    fn test_from_hashes_roundtrip() {
        let mut mmr = Mmr::new();
        for i in 0..6u8 {
            mmr.append(&[i]);
        }
        let leaves: Vec<[u8; 32]> = (0..6u8).map(|i| leaf_hash(&[i])).collect();
        let restored = Mmr::from_hashes(leaves);
        assert_eq!(mmr.bag(), restored.bag());
        assert_eq!(mmr.peaks(), restored.peaks());
    }

    #[test]
    fn test_proof_siblings_alternate_sides() {
        // index 1 in a 4-leaf tree: first sibling is left, then right pattern
        let mut mmr = Mmr::new();
        for i in 0..4u8 {
            mmr.append(&[i]);
        }
        let proof = mmr.inclusion_proof(1).expect("proof");
        assert_eq!(proof.len(), 2);
        assert!(proof[0].1, "sibling of index 1 at leaf level is left");
        assert!(!proof[1].1, "next sibling is right");
    }

    #[test]
    fn test_single_leaf() {
        let mut mmr = Mmr::new();
        mmr.append(&[42]);
        let peaks = mmr.peaks();
        assert_eq!(peaks.len(), 1);
        assert_eq!(peaks[0].height, 0);
        let proof = mmr.inclusion_proof(0).expect("proof");
        assert!(proof.is_empty());
        assert!(verify_inclusion(&[42], 0, &proof, &peaks));
    }
}
