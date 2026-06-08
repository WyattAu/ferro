pub mod key;
pub mod ring;

pub use key::{hash_key, hash_key_with_salt};
pub use ring::{HashRing, RingEntry};

#[cfg(test)]
mod tests {
    use super::*;
    use ring::HashRing;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TestMeta {
        name: String,
    }

    #[test]
    fn test_full_workflow() {
        let mut ring = HashRing::new(150);
        ring.add_node("node-1".to_string(), None, TestMeta { name: "n1".into() });
        ring.add_node("node-2".to_string(), None, TestMeta { name: "n2".into() });
        ring.add_node("node-3".to_string(), None, TestMeta { name: "n3".into() });

        let (meta, _) = ring.get_node(b"user:12345:file.txt").unwrap();
        assert!(["n1", "n2", "n3"].contains(&meta.name.as_str()));

        let replicas = ring.get_nodes(b"user:12345:file.txt", 2);
        assert_eq!(replicas.len(), 2);

        let mut names: Vec<&str> = replicas.iter().map(|(m, _)| m.name.as_str()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn test_hash_consistency() {
        let h1 = hash_key(b"test");
        let h2 = hash_key(b"test");
        assert_eq!(h1, h2);

        let h3 = hash_key_with_salt(b"test", b"salt");
        assert_ne!(h1, h3);
    }
}
