use std::collections::HashSet;

use dashmap::DashMap;

use crate::fuzzy::prefix_match;

#[derive(Debug, Clone)]
pub struct Posting {
    pub document_id: String,
    pub positions: Vec<usize>,
    pub field: String,
}

pub struct InvertedIndex {
    index: DashMap<String, Vec<Posting>>,
}

impl Default for InvertedIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl InvertedIndex {
    pub fn new() -> Self {
        Self {
            index: DashMap::new(),
        }
    }

    pub fn add_posting(&self, term: &str, posting: Posting) {
        self.index
            .entry(term.to_lowercase())
            .or_default()
            .push(posting);
    }

    pub fn remove_document(&self, document_id: &str, terms: &[String]) {
        let mut terms_to_clean: HashSet<String> = HashSet::new();
        for term in terms {
            let term_lower = term.to_lowercase();
            if let Some(mut postings) = self.index.get_mut(&term_lower) {
                postings.retain(|p| p.document_id != document_id);
                if postings.is_empty() {
                    terms_to_clean.insert(term_lower);
                }
            }
        }
        for term in terms_to_clean {
            self.index.remove(&term);
        }
    }

    pub fn get_postings(&self, term: &str) -> Option<Vec<Posting>> {
        let term_lower = term.to_lowercase();
        self.index.get(&term_lower).map(|r| r.clone())
    }

    pub fn term_count(&self) -> usize {
        self.index.len()
    }

    pub fn terms_with_prefix(&self, prefix: &str) -> Vec<String> {
        let prefix_lower = prefix.to_lowercase();
        let mut terms: Vec<String> = self
            .index
            .iter()
            .filter_map(|entry| {
                if prefix_match(&prefix_lower, entry.key()) {
                    Some(entry.key().clone())
                } else {
                    None
                }
            })
            .collect();
        terms.sort();
        terms.dedup();
        terms
    }

    pub fn all_terms(&self) -> Vec<String> {
        self.index.iter().map(|e| e.key().clone()).collect()
    }
}
