use std::collections::HashMap;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::text::{RgaString, TextOperation};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct DocumentId(pub String);

impl Default for DocumentId {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentId {
    pub fn new() -> Self {
        DocumentId(Uuid::new_v4().to_string())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ParticipantId(pub u32);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantInfo {
    pub site_id: u32,
    pub name: String,
    #[serde(skip, default = "default_instant")]
    pub last_seen: Instant,
}

fn default_instant() -> Instant {
    Instant::now()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrdtDocument {
    pub id: DocumentId,
    content: RgaString,
    pub version: u64,
    pub participants: HashMap<ParticipantId, ParticipantInfo>,
}

impl CrdtDocument {
    pub fn new(id: DocumentId) -> Self {
        CrdtDocument {
            id,
            content: RgaString::new(),
            version: 0,
            participants: HashMap::new(),
        }
    }

    pub fn join(&mut self, participant_id: ParticipantId, name: &str) -> ParticipantId {
        self.participants.insert(
            participant_id,
            ParticipantInfo {
                site_id: participant_id.0,
                name: name.to_string(),
                last_seen: Instant::now(),
            },
        );
        participant_id
    }

    pub fn leave(&mut self, participant_id: &ParticipantId) {
        self.participants.remove(participant_id);
    }

    pub fn insert_text(
        &mut self,
        participant_id: ParticipantId,
        index: usize,
        text: &str,
    ) -> (Vec<TextOperation>, u64) {
        let site_id = self
            .participants
            .get(&participant_id)
            .map(|p| p.site_id)
            .unwrap_or(participant_id.0);

        let ops = self.content.insert(site_id, index, text);
        self.version += 1;
        (ops, self.version)
    }

    pub fn delete_text(
        &mut self,
        participant_id: ParticipantId,
        index: usize,
        len: usize,
    ) -> (Vec<TextOperation>, u64) {
        let site_id = self
            .participants
            .get(&participant_id)
            .map(|p| p.site_id)
            .unwrap_or(participant_id.0);

        let ops = self.content.delete(site_id, index, len);
        self.version += 1;
        (ops, self.version)
    }

    pub fn apply_ops(&mut self, ops: &[TextOperation]) -> u64 {
        for op in ops {
            self.content.apply(op);
        }
        self.version += 1;
        self.version
    }

    pub fn get_text(&self) -> String {
        self.content.text()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create() {
        let doc = CrdtDocument::new(DocumentId("test-doc".to_string()));
        assert_eq!(doc.id.0, "test-doc");
        assert_eq!(doc.version, 0);
        assert!(doc.participants.is_empty());
        assert_eq!(doc.get_text(), "");
    }

    #[test]
    fn test_join_leave() {
        let mut doc = CrdtDocument::new(DocumentId::new());

        let p1 = ParticipantId(1);
        doc.join(p1, "Alice");
        assert!(doc.participants.contains_key(&p1));
        assert_eq!(doc.participants[&p1].name, "Alice");
        assert_eq!(doc.participants[&p1].site_id, 1);

        let p2 = ParticipantId(2);
        doc.join(p2, "Bob");
        assert_eq!(doc.participants.len(), 2);

        doc.leave(&p1);
        assert_eq!(doc.participants.len(), 1);
        assert!(!doc.participants.contains_key(&p1));
        assert!(doc.participants.contains_key(&p2));
    }

    #[test]
    fn test_insert_and_version() {
        let mut doc = CrdtDocument::new(DocumentId::new());
        doc.join(ParticipantId(1), "Alice");

        let (ops, v1) = doc.insert_text(ParticipantId(1), 0, "Hello");
        assert_eq!(v1, 1);
        assert!(!ops.is_empty());
        assert_eq!(doc.get_text(), "Hello");
        assert_eq!(doc.version, 1);

        let (_, v2) = doc.insert_text(ParticipantId(1), 5, " World");
        assert_eq!(v2, 2);
        assert_eq!(doc.get_text(), "Hello World");
        assert_eq!(doc.version, 2);
    }

    #[test]
    fn test_apply_remote_ops() {
        let mut doc1 = CrdtDocument::new(DocumentId("shared".to_string()));
        doc1.join(ParticipantId(1), "Alice");
        let (ops, v1) = doc1.insert_text(ParticipantId(1), 0, "Hello");
        assert_eq!(v1, 1);

        let mut doc2 = CrdtDocument::new(DocumentId("shared".to_string()));
        doc2.join(ParticipantId(2), "Bob");

        let v = doc2.apply_ops(&ops);
        assert_eq!(v, 1);
        assert_eq!(doc2.get_text(), "Hello");
        assert_eq!(doc2.version, 1);
    }

    #[test]
    fn test_multiple_participants() {
        let mut doc = CrdtDocument::new(DocumentId::new());
        doc.join(ParticipantId(1), "Alice");
        doc.join(ParticipantId(2), "Bob");

        let (ops1, v1) = doc.insert_text(ParticipantId(1), 0, "Hello");
        assert_eq!(v1, 1);

        let (ops2, v2) = doc.insert_text(ParticipantId(2), 5, " World");
        assert_eq!(v2, 2);

        assert_eq!(doc.get_text(), "Hello World");

        let mut doc2 = CrdtDocument::new(DocumentId::new());
        doc2.join(ParticipantId(3), "Charlie");

        let v3 = doc2.apply_ops(&ops1);
        assert_eq!(v3, 1);
        let v4 = doc2.apply_ops(&ops2);
        assert_eq!(v4, 2);
        assert_eq!(doc2.get_text(), "Hello World");
    }

    #[test]
    fn test_serialization() {
        let mut doc = CrdtDocument::new(DocumentId("serde-test".to_string()));
        doc.join(ParticipantId(1), "Alice");
        doc.insert_text(ParticipantId(1), 0, "Hello");

        let json = serde_json::to_string(&doc).unwrap();
        let deserialized: CrdtDocument = serde_json::from_str(&json).unwrap();
        assert_eq!(doc.id, deserialized.id);
        assert_eq!(doc.version, deserialized.version);
        assert_eq!(doc.get_text(), deserialized.get_text());
    }
}
