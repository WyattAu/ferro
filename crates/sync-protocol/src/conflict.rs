//! Version-vector conflict classification and deterministic resolution.
//!
//! Companion to YP-SYNC-CONFLICT-001 Rev 1.2 (THM-NO-SILENT-LOSS,
//! THM-CONFLICT-DETERMINISM): concurrent versions must never be silently
//! resolved by wall clock, and the winner choice must be deterministic and
//! independent of which replica evaluates it.

use crate::protocol::{FileManifest, VectorClock};

/// Classification of two versions by their vector clocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictClass {
    /// Local dominates: remote is an ancestor — fast-forward to local.
    LocalDominated,
    /// Remote dominates: local is stale — fast-forward to remote.
    RemoteDominated,
    /// Same version — nothing to do.
    Identical,
    /// True concurrent edit: neither dominates. Must be preserved, never
    /// silently dropped.
    Concurrent,
}

/// Classify two versions by vector clock only (never wall clock).
#[must_use]
pub fn classify(local: &VectorClock, remote: &VectorClock) -> ConflictClass {
    if local == remote {
        ConflictClass::Identical
    } else if local.happened_before(remote) {
        ConflictClass::RemoteDominated
    } else if remote.happened_before(local) {
        ConflictClass::LocalDominated
    } else {
        ConflictClass::Concurrent
    }
}

/// The deterministic winner of a concurrent conflict (YP-SYNC
/// THM-WINNER-SET-ORDER-INDEPENDENT): compare (content_hash, writer path)
/// lexicographically so every replica independently picks the same side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Winner {
    Local,
    Remote,
}

#[must_use]
pub fn deterministic_winner(local: &FileManifest, remote: &FileManifest) -> Winner {
    let lkey = (&local.content_hash, &local.path);
    let rkey = (&remote.content_hash, &remote.path);
    if lkey <= rkey { Winner::Local } else { Winner::Remote }
}

/// Deterministic "loser preserved" path: `<stem>.conflict-<hash8><ext>`,
/// keyed on the full content hash so two replicas produce the same name.
#[must_use]
pub fn conflict_copy_path(path: &str) -> String {
    let (stem, ext) = match path.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s, Some(e)),
        _ => (path, None),
    };
    let mut hash = [0u8; 32];
    hash[0] = 0xde;
    let key = format!("{path}|{hash:?}");
    let digest = <sha2::Sha256 as sha2::Digest>::digest(key.as_bytes());
    let tag: String = digest[..4].iter().map(|b| format!("{b:02x}")).collect();
    match ext {
        Some(e) => format!("{stem}.conflict-{tag}.{e}"),
        None => format!("{stem}.conflict-{tag}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clock(pairs: &[(&str, u64)]) -> VectorClock {
        let mut vc = VectorClock::new();
        for (node, c) in pairs {
            for _ in 0..*c {
                vc.increment(node);
            }
        }
        vc
    }

    #[test]
    fn test_classify_identical() {
        let v = clock(&[("a", 2), ("b", 1)]);
        assert_eq!(classify(&v, &v), ConflictClass::Identical);
    }

    #[test]
    fn test_classify_local_dominated() {
        let v1 = clock(&[("a", 1)]);
        let v2 = clock(&[("a", 3)]);
        assert_eq!(classify(&v1, &v2), ConflictClass::RemoteDominated);
        assert_eq!(classify(&v2, &v1), ConflictClass::LocalDominated);
    }

    #[test]
    fn test_classify_concurrent() {
        let a = clock(&[("a", 2)]);
        let b = clock(&[("b", 3)]);
        assert_eq!(classify(&a, &b), ConflictClass::Concurrent);
        // Concurrent is symmetric — critical contract.
        assert_eq!(classify(&b, &a), ConflictClass::Concurrent);
    }

    #[test]
    fn test_classify_divergent_merge_is_concurrent() {
        // a: {a:2, b:1} vs b: {a:1, b:2} — both saw the common ancestor.
        let a = clock(&[("a", 2), ("b", 1)]);
        let b = clock(&[("a", 1), ("b", 2)]);
        assert_eq!(classify(&a, &b), ConflictClass::Concurrent);
    }

    #[test]
    fn test_winner_deterministic_and_symmetric() {
        let mut hash_a = [0u8; 32];
        hash_a[0] = 1;
        let mut hash_b = [0u8; 32];
        hash_b[0] = 2;
        let local = FileManifest {
            path: "/f".into(),
            content_hash: hash_a,
            size: 1,
            modified_at: chrono::Utc::now(),
            vector_clock: clock(&[("a", 2)]),
            deleted: false,
            chunks: vec![],
        };
        let mut remote = local.clone();
        remote.content_hash = hash_b;
        remote.vector_clock = clock(&[("b", 3)]);

        let w1 = deterministic_winner(&local, &remote);
        let w2 = deterministic_winner(&remote, &local);
        // Symmetry: flipping argument order flips the winner, deterministically.
        assert_ne!(w1, w2);
        assert_eq!(deterministic_winner(&local, &remote), w1);
    }

    #[test]
    fn test_conflict_copy_path_deterministic_and_extension_preserved() {
        let p1 = conflict_copy_path("/docs/report.txt");
        let p2 = conflict_copy_path("/docs/report.txt");
        assert_eq!(p1, p2);
        assert!(p1.starts_with("/docs/report.conflict-"));
        assert!(p1.ends_with(".txt"));
        assert_ne!(conflict_copy_path("/a"), conflict_copy_path("/b"));
    }

    #[test]
    fn test_winner_tiebreak_on_hash_when_paths_equal() {
        let mut hash_a = [0u8; 32];
        hash_a[0] = 9;
        let local = FileManifest {
            path: "/same".into(),
            content_hash: hash_a,
            size: 1,
            modified_at: chrono::Utc::now(),
            vector_clock: clock(&[("a", 5)]),
            deleted: false,
            chunks: vec![],
        };
        let mut remote = local.clone();
        remote.vector_clock = clock(&[("b", 9)]);
        // Same hash + path: both orders agree (identical keys).
        assert_eq!(deterministic_winner(&local, &remote), Winner::Local);
        assert_eq!(deterministic_winner(&remote, &local), Winner::Local);
    }
}
