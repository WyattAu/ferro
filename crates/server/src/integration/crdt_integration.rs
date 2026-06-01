//! CRDT co-editing integration.
//!
//! Provides helper functions for real-time document collaboration.

use ferro_crdt::{CrdtDocument, DocumentId, ParticipantId, TextOperation};

pub fn create_collab_doc(id: &str) -> CrdtDocument {
    CrdtDocument::new(DocumentId(id.to_string()))
}

pub fn join_document(doc: &mut CrdtDocument, site_id: u32, name: &str) -> ParticipantId {
    doc.join(ParticipantId(site_id), name)
}

pub fn insert_text(doc: &mut CrdtDocument, participant: ParticipantId, index: usize, text: &str) -> (Vec<TextOperation>, u64) {
    doc.insert_text(participant, index, text)
}

pub fn apply_remote_ops(doc: &mut CrdtDocument, ops: &[TextOperation]) -> u64 {
    doc.apply_ops(ops)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_doc() {
        let doc = create_collab_doc("test-doc");
        assert_eq!(doc.id.0, "test-doc");
        assert_eq!(doc.version, 0);
        assert_eq!(doc.get_text(), "");
    }

    #[test]
    fn test_join_and_insert() {
        let mut doc = create_collab_doc("collab");
        let p = join_document(&mut doc, 1, "Alice");
        let (ops, version) = insert_text(&mut doc, p, 0, "Hello");
        assert!(!ops.is_empty());
        assert_eq!(version, 1);
        assert_eq!(doc.get_text(), "Hello");
    }

    #[test]
    fn test_apply_remote_ops() {
        let mut doc1 = create_collab_doc("shared");
        join_document(&mut doc1, 1, "Alice");
        let (ops, _) = insert_text(&mut doc1, ParticipantId(1), 0, "Hello");

        let mut doc2 = create_collab_doc("shared");
        join_document(&mut doc2, 2, "Bob");
        apply_remote_ops(&mut doc2, &ops);
        assert_eq!(doc2.get_text(), "Hello");
    }
}
