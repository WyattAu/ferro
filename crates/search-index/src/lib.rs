pub mod error;
pub mod fuzzy;
pub mod index;
pub mod query;
pub mod ranking;

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use dashmap::DashMap;
use error::SearchError;
use index::{InvertedIndex, Posting};
use query::{Query, QueryParser};
use ranking::Ranker;

pub type FieldBoost = HashMap<String, f64>;

type ScoreMap = HashMap<String, (f64, Vec<String>, HashMap<String, String>)>;

#[derive(Debug, Clone)]
pub struct SearchIndexConfig {
    pub cache_ttl: Duration,
    pub shard_count: usize,
}

impl Default for SearchIndexConfig {
    fn default() -> Self {
        Self {
            cache_ttl: Duration::from_secs(300),
            shard_count: 1,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SearchMetrics {
    pub query_time_us: u64,
    pub total_results: usize,
    pub cache_hit: bool,
}

struct CacheEntry {
    results: Vec<SearchResult>,
    created_at: Instant,
}

struct QueryCache {
    entries: DashMap<String, CacheEntry>,
    ttl: Duration,
    hits: AtomicUsize,
    misses: AtomicUsize,
}

impl QueryCache {
    fn new(ttl: Duration) -> Self {
        Self {
            entries: DashMap::new(),
            ttl,
            hits: AtomicUsize::new(0),
            misses: AtomicUsize::new(0),
        }
    }

    fn get(&self, key: &str) -> Option<Vec<SearchResult>> {
        if let Some(entry) = self.entries.get(key)
            && entry.created_at.elapsed() < self.ttl
        {
            self.hits.fetch_add(1, Ordering::Relaxed);
            return Some(entry.results.clone());
        }
        self.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    fn insert(&self, key: String, results: Vec<SearchResult>) {
        self.entries.insert(
            key,
            CacheEntry {
                results,
                created_at: Instant::now(),
            },
        );
    }

    fn invalidate(&self) {
        self.entries.clear();
    }

    fn stats(&self) -> (usize, usize) {
        (
            self.hits.load(Ordering::Relaxed),
            self.misses.load(Ordering::Relaxed),
        )
    }
}

#[derive(Debug, Clone)]
pub struct Document {
    pub id: String,
    pub fields: HashMap<String, String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct DocumentUpdate {
    pub fields: Option<HashMap<String, String>>,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub document_id: String,
    pub score: f64,
    pub matched_fields: Vec<String>,
    pub highlights: HashMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct SearchFilter {
    pub field_filters: HashMap<String, String>,
    pub min_score: Option<f64>,
    pub limit: usize,
    pub offset: usize,
}

pub struct SearchIndex {
    documents: DashMap<String, Document>,
    inverted: InvertedIndex,
    fields: Vec<String>,
    field_boosts: FieldBoost,
    config: SearchIndexConfig,
    cache: QueryCache,
}

impl SearchIndex {
    pub fn new(fields: Vec<String>) -> Self {
        Self::with_boosts(fields, HashMap::new())
    }

    pub fn with_boosts(fields: Vec<String>, field_boosts: FieldBoost) -> Self {
        Self::with_config(fields, field_boosts, SearchIndexConfig::default())
    }

    pub fn with_config(
        fields: Vec<String>,
        field_boosts: FieldBoost,
        config: SearchIndexConfig,
    ) -> Self {
        Self {
            documents: DashMap::new(),
            inverted: InvertedIndex::new(),
            cache: QueryCache::new(config.cache_ttl),
            fields,
            field_boosts,
            config,
        }
    }

    pub fn add_document(&self, doc: Document) -> Result<(), SearchError> {
        if doc.id.is_empty() {
            return Err(SearchError::EmptyDocumentId);
        }
        if self.documents.contains_key(&doc.id) {
            return Err(SearchError::DocumentAlreadyExists(doc.id.clone()));
        }
        self.index_document(&doc);
        self.documents.insert(doc.id.clone(), doc);
        self.cache.invalidate();
        Ok(())
    }

    pub fn remove_document(&self, id: &str) -> Result<(), SearchError> {
        let (_, doc) = self
            .documents
            .remove(id)
            .ok_or_else(|| SearchError::DocumentNotFound(id.to_string()))?;

        let mut terms_to_remove: Vec<String> = Vec::new();
        for field_name in &self.fields {
            if let Some(value) = doc.fields.get(field_name) {
                for (pos, token) in tokenize(value).into_iter().enumerate() {
                    terms_to_remove.push(token);
                    let _ = pos;
                }
            }
        }

        self.inverted.remove_document(id, &terms_to_remove);
        self.cache.invalidate();
        Ok(())
    }

    pub fn update_document(&self, id: &str, updates: DocumentUpdate) -> Result<(), SearchError> {
        let old_terms: Vec<String> = {
            let doc = self
                .documents
                .get(id)
                .ok_or_else(|| SearchError::DocumentNotFound(id.to_string()))?;
            let mut terms = Vec::new();
            for field_name in &self.fields {
                if let Some(value) = doc.fields.get(field_name) {
                    terms.extend(tokenize(value));
                }
            }
            terms
        };

        let mut doc = self
            .documents
            .get_mut(id)
            .ok_or_else(|| SearchError::DocumentNotFound(id.to_string()))?;

        if let Some(fields) = updates.fields {
            for (k, v) in fields {
                doc.fields.insert(k, v);
            }
        }
        if let Some(metadata) = updates.metadata {
            for (k, v) in metadata {
                doc.metadata.insert(k, v);
            }
        }
        let updated = doc.clone();
        drop(doc);
        self.inverted.remove_document(id, &old_terms);
        self.index_document(&updated);
        self.cache.invalidate();
        Ok(())
    }

    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        self.search_with_filter(query, SearchFilter::default()).0
    }

    pub fn search_with_metrics(
        &self,
        query: &str,
        filter: SearchFilter,
    ) -> (Vec<SearchResult>, SearchMetrics) {
        let start = Instant::now();
        let cache_key = format!("{}:{:?}", query, filter);

        if let Some(cached) = self.cache.get(&cache_key) {
            let elapsed = start.elapsed().as_micros() as u64;
            let total = cached.len();
            return (
                cached,
                SearchMetrics {
                    query_time_us: elapsed,
                    total_results: total,
                    cache_hit: true,
                },
            );
        }

        let results = self.execute_search(query, &filter);
        let total = results.len();
        let elapsed = start.elapsed().as_micros() as u64;

        self.cache.insert(cache_key, results.clone());

        (
            results,
            SearchMetrics {
                query_time_us: elapsed,
                total_results: total,
                cache_hit: false,
            },
        )
    }

    pub fn search_with_filter(
        &self,
        query_str: &str,
        filter: SearchFilter,
    ) -> (Vec<SearchResult>, SearchMetrics) {
        let start = Instant::now();
        let cache_key = format!("{}:{:?}", query_str, filter);

        if let Some(cached) = self.cache.get(&cache_key) {
            let elapsed = start.elapsed().as_micros() as u64;
            let total = cached.len();
            return (
                cached,
                SearchMetrics {
                    query_time_us: elapsed,
                    total_results: total,
                    cache_hit: true,
                },
            );
        }

        let results = self.execute_search(query_str, &filter);
        let total = results.len();
        let elapsed = start.elapsed().as_micros() as u64;

        self.cache.insert(cache_key, results.clone());

        (
            results,
            SearchMetrics {
                query_time_us: elapsed,
                total_results: total,
                cache_hit: false,
            },
        )
    }

    pub fn search_paginated(
        &self,
        query_str: &str,
        offset: usize,
        limit: usize,
    ) -> (Vec<SearchResult>, SearchMetrics) {
        let filter = SearchFilter {
            offset,
            limit,
            ..Default::default()
        };
        self.search_with_filter(query_str, filter)
    }

    fn execute_search(&self, query_str: &str, filter: &SearchFilter) -> Vec<SearchResult> {
        let parsed = match QueryParser::parse(query_str) {
            Ok(q) => q,
            Err(_) => return Vec::new(),
        };

        let has_field_filters = !filter.field_filters.is_empty();
        let mut matching_docs = self.evaluate_query(&parsed);

        for (doc_id, (score, _matched_fields, _highlights)) in matching_docs.iter_mut() {
            let filter_match = filter.field_filters.iter().all(|(key, value)| {
                self.documents
                    .get(doc_id)
                    .map(|d| d.metadata.get(key).map(|v| v == value).unwrap_or(false))
                    .unwrap_or(false)
            });
            if !filter_match {
                *score = 0.0;
            }
        }

        if has_field_filters {
            matching_docs.retain(|_, (score, _, _)| *score > 0.0);
        }

        matching_docs.retain(|_, (score, _, _)| filter.min_score.is_none_or(|min| *score >= min));

        let mut results: Vec<SearchResult> = matching_docs
            .into_iter()
            .map(
                |(doc_id, (score, matched_fields, highlights))| SearchResult {
                    document_id: doc_id,
                    score,
                    matched_fields,
                    highlights,
                },
            )
            .collect();

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let offset = filter.offset.min(results.len());
        results = results.into_iter().skip(offset).collect();

        if filter.limit > 0 {
            results.truncate(filter.limit);
        }

        results
    }

    pub fn suggest(&self, prefix: &str, limit: usize) -> Vec<String> {
        let prefix_lower = prefix.to_lowercase();
        let mut suggestions: Vec<String> = self
            .inverted
            .terms_with_prefix(&prefix_lower)
            .into_iter()
            .take(limit)
            .collect();
        suggestions.sort();
        suggestions.dedup();
        suggestions
    }

    pub fn document_count(&self) -> usize {
        self.documents.len()
    }

    pub fn term_count(&self) -> usize {
        self.inverted.term_count()
    }

    pub fn config(&self) -> &SearchIndexConfig {
        &self.config
    }

    pub fn cache_stats(&self) -> (usize, usize) {
        self.cache.stats()
    }

    fn index_document(&self, doc: &Document) {
        for field_name in &self.fields {
            if let Some(value) = doc.fields.get(field_name) {
                let tokens = tokenize(value);
                for (pos, token) in tokens.iter().enumerate() {
                    let posting = Posting {
                        document_id: doc.id.clone(),
                        positions: vec![pos],
                        field: field_name.clone(),
                    };
                    self.inverted.add_posting(token, posting);
                }
            }
        }
    }

    #[allow(dead_code)]
    fn reindex_document(&self, id: &str, doc: &Document) {
        if let Some(old_doc) = self.documents.get(id) {
            let mut old_terms: Vec<String> = Vec::new();
            for field_name in &self.fields {
                if let Some(value) = old_doc.fields.get(field_name) {
                    old_terms.extend(tokenize(value));
                }
            }
            self.inverted.remove_document(id, &old_terms);
        }
        self.index_document(doc);
    }

    fn evaluate_query(&self, query: &Query) -> ScoreMap {
        let mut scores: ScoreMap = HashMap::new();

        let ranker = Ranker::new(
            self.documents.len(),
            self.field_boosts.clone(),
            self.fields.clone(),
        );

        match query {
            Query::Term(term) => {
                self.score_term(term, &ranker, &mut scores);
            }
            Query::Phrase(terms) => {
                self.score_phrase(terms, &ranker, &mut scores);
            }
            Query::And(left, right) => {
                let left_scores = self.evaluate_query(left);
                let right_scores = self.evaluate_query(right);
                for (doc_id, (score_l, fields_l, hl_l)) in &left_scores {
                    if let Some((score_r, fields_r, hl_r)) = right_scores.get(doc_id) {
                        let combined_score = score_l + score_r;
                        let mut combined_fields = fields_l.clone();
                        combined_fields.extend(fields_r.iter().cloned());
                        combined_fields.sort();
                        combined_fields.dedup();
                        let mut combined_hl = hl_l.clone();
                        combined_hl.extend(hl_r.iter().map(|(k, v)| (k.clone(), v.clone())));
                        scores.insert(
                            doc_id.clone(),
                            (combined_score, combined_fields, combined_hl),
                        );
                    }
                }
            }
            Query::Or(left, right) => {
                let left_scores = self.evaluate_query(left);
                let right_scores = self.evaluate_query(right);
                scores = left_scores;
                for (doc_id, (score_r, fields_r, hl_r)) in right_scores {
                    scores
                        .entry(doc_id)
                        .and_modify(|(score_l, fields_l, hl_l)| {
                            *score_l += score_r;
                            fields_l.extend(fields_r.iter().cloned());
                            fields_l.sort();
                            fields_l.dedup();
                            hl_l.extend(hl_r.iter().map(|(k, v)| (k.clone(), v.clone())));
                        })
                        .or_insert((score_r, fields_r, hl_r));
                }
            }
            Query::Not(inner) => {
                let inner_scores = self.evaluate_query(inner);
                let excluded: std::collections::HashSet<String> =
                    inner_scores.keys().cloned().collect();
                for entry in self.documents.iter() {
                    if !excluded.contains(entry.key()) {
                        scores.insert(entry.key().clone(), (1.0, Vec::new(), HashMap::new()));
                    }
                }
            }
            Query::Field(field, term) => {
                self.score_field_term(field, term, &ranker, &mut scores);
            }
            Query::Boost(inner, factor) => {
                let inner_scores = self.evaluate_query(inner);
                for (doc_id, (score, fields, hl)) in inner_scores {
                    scores.insert(doc_id, (score * factor, fields, hl));
                }
            }
        }

        scores
    }

    fn score_term(&self, term: &str, ranker: &Ranker, scores: &mut ScoreMap) {
        let term_lower = term.to_lowercase();
        if let Some(postings) = self.inverted.get_postings(&term_lower) {
            let df = postings
                .iter()
                .map(|p| p.document_id.clone())
                .collect::<std::collections::HashSet<_>>()
                .len();

            for posting in postings.iter() {
                let tf = posting.positions.len() as f64;
                let boost = ranker.field_boost(&posting.field);
                let score = ranker.tf_idf(tf, df, posting.positions.len());

                let highlight = self.highlight_term(&posting.document_id, &posting.field, term);

                scores
                    .entry(posting.document_id.clone())
                    .and_modify(|(s, f, h)| {
                        *s += score * boost;
                        if !f.contains(&posting.field) {
                            f.push(posting.field.clone());
                        }
                        if !highlight.is_empty() {
                            h.insert(posting.field.clone(), highlight.clone());
                        }
                    })
                    .or_insert_with(|| {
                        (
                            score * boost,
                            vec![posting.field.clone()],
                            if highlight.is_empty() {
                                HashMap::new()
                            } else {
                                let mut h = HashMap::new();
                                h.insert(posting.field.clone(), highlight);
                                h
                            },
                        )
                    });
            }
        } else {
            let fuzzy_results = self.fuzzy_search_term(term);
            for (doc_id, field, score) in fuzzy_results {
                let field_clone = field.clone();
                scores
                    .entry(doc_id)
                    .and_modify(|(s, f, _)| {
                        *s += score * 0.5;
                        if !f.contains(&field) {
                            f.push(field);
                        }
                    })
                    .or_insert((score * 0.5, vec![field_clone], HashMap::new()));
            }
        }
    }

    fn score_field_term(&self, field: &str, term: &str, ranker: &Ranker, scores: &mut ScoreMap) {
        let term_lower = term.to_lowercase();
        if let Some(postings) = self.inverted.get_postings(&term_lower) {
            let field_postings: Vec<_> = postings.iter().filter(|p| p.field == field).collect();
            let df = field_postings.len();
            for posting in &field_postings {
                let tf = posting.positions.len() as f64;
                let boost = ranker.field_boost(field);
                let score = ranker.tf_idf(tf, df, posting.positions.len());

                scores
                    .entry(posting.document_id.clone())
                    .and_modify(|(s, f, _)| {
                        *s += score * boost;
                        if !f.contains(&field.to_string()) {
                            f.push(field.to_string());
                        }
                    })
                    .or_insert((score * boost, vec![field.to_string()], HashMap::new()));
            }
        }
    }

    fn score_phrase(&self, terms: &[String], ranker: &Ranker, scores: &mut ScoreMap) {
        if terms.is_empty() {
            return;
        }

        let term_lower: Vec<String> = terms.iter().map(|t| t.to_lowercase()).collect();

        let posting_vecs: Vec<Vec<Posting>> = term_lower
            .iter()
            .filter_map(|t| self.inverted.get_postings(t))
            .collect();

        if posting_vecs.len() != term_lower.len() {
            return;
        }

        let doc_ids: std::collections::HashSet<String> = posting_vecs
            .iter()
            .flat_map(|ps| ps.iter().map(|p| p.document_id.clone()))
            .collect();

        for doc_id in &doc_ids {
            for field_name in &self.fields {
                let mut all_have = true;
                let mut field_postings: Vec<Vec<usize>> = Vec::new();

                for postings in &posting_vecs {
                    let matching: Vec<&Posting> = postings
                        .iter()
                        .filter(|p| p.document_id == *doc_id && p.field == *field_name)
                        .collect();
                    if matching.is_empty() {
                        all_have = false;
                        break;
                    }
                    let all_positions: Vec<usize> = matching
                        .iter()
                        .flat_map(|p| p.positions.iter().copied())
                        .collect();
                    field_postings.push(all_positions);
                }

                if !all_have {
                    continue;
                }

                if Self::has_phrase_match(&field_postings) {
                    let phrase_score = ranker.tf_idf(1.0, 1, term_lower.len());
                    let boost = ranker.field_boost(field_name);
                    let hl = self.highlight_phrase(doc_id, field_name, &terms.join(" "));
                    let hl_empty = hl.is_empty();

                    scores
                        .entry(doc_id.clone())
                        .and_modify(|(s, f, h)| {
                            *s += phrase_score * boost * 1.5;
                            if !f.contains(field_name) {
                                f.push(field_name.clone());
                            }
                            if !hl_empty {
                                h.insert(field_name.clone(), hl.clone());
                            }
                        })
                        .or_insert_with(|| {
                            (
                                phrase_score * boost * 1.5,
                                vec![field_name.clone()],
                                if hl_empty {
                                    HashMap::new()
                                } else {
                                    let mut h = HashMap::new();
                                    h.insert(field_name.clone(), hl);
                                    h
                                },
                            )
                        });
                }
            }
        }
    }

    fn has_phrase_match(positions: &[Vec<usize>]) -> bool {
        if positions.len() <= 1 {
            return true;
        }
        for &start_pos in &positions[0] {
            let mut found = true;
            for (i, positions_i) in positions.iter().enumerate().skip(1) {
                let expected = start_pos + i;
                if !positions_i.contains(&expected) {
                    found = false;
                    break;
                }
            }
            if found {
                return true;
            }
        }
        false
    }

    fn highlight_term(&self, doc_id: &str, field: &str, term: &str) -> String {
        if let Some(doc) = self.documents.get(doc_id)
            && let Some(value) = doc.fields.get(field)
        {
            let value_lower = value.to_lowercase();
            let term_lower = term.to_lowercase();
            if let Some(idx) = value_lower.find(&term_lower) {
                let end = idx + term.len().min(value.len() - idx);
                let before = &value[..idx];
                let matched = &value[idx..end];
                let after = &value[end..];
                let snippet = format!("{before}**{matched}**{after}");
                let truncated = snippet.chars().take(100).collect::<String>();
                return truncated;
            }
        }
        String::new()
    }

    fn highlight_phrase(&self, doc_id: &str, field: &str, phrase: &str) -> String {
        if let Some(doc) = self.documents.get(doc_id)
            && let Some(value) = doc.fields.get(field)
        {
            let value_lower = value.to_lowercase();
            let phrase_lower = phrase.to_lowercase();
            if let Some(idx) = value_lower.find(&phrase_lower) {
                let end = (idx + phrase.len()).min(value.len());
                let before = &value[..idx];
                let matched = &value[idx..end];
                let after = &value[end..];
                let snippet = format!("{before}**{matched}**{after}");
                let truncated = snippet.chars().take(100).collect::<String>();
                return truncated;
            }
        }
        String::new()
    }

    fn fuzzy_search_term(&self, term: &str) -> Vec<(String, String, f64)> {
        let term_lower = term.to_lowercase();
        let mut results: Vec<(String, String, f64)> = Vec::new();
        let indexed_terms = self.inverted.all_terms();

        for indexed_term in indexed_terms {
            let max_dist = (term.len() / 3).clamp(1, 3);
            if let Some(similarity) = fuzzy::fuzzy_match(&term_lower, &indexed_term, max_dist)
                && let Some(postings) = self.inverted.get_postings(&indexed_term)
            {
                for posting in postings.iter() {
                    results.push((
                        posting.document_id.clone(),
                        posting.field.clone(),
                        similarity,
                    ));
                }
            }
        }

        results.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(10);
        results
    }
}

pub fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            current.push(ch.to_lowercase().next().unwrap_or(ch));
        } else {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_doc(id: &str, name: &str, path: &str) -> Document {
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), name.to_string());
        fields.insert("path".to_string(), path.to_string());
        let mut meta = HashMap::new();
        meta.insert("type".to_string(), "file".to_string());
        Document {
            id: id.to_string(),
            fields,
            metadata: meta,
        }
    }

    fn make_index() -> SearchIndex {
        SearchIndex::new(vec!["name".to_string(), "path".to_string()])
    }

    fn make_boosted_index() -> SearchIndex {
        let mut boosts = HashMap::new();
        boosts.insert("name".to_string(), 2.0);
        boosts.insert("path".to_string(), 1.0);
        SearchIndex::with_boosts(vec!["name".to_string(), "path".to_string()], boosts)
    }

    #[test]
    fn test_basic_indexing_and_search() {
        let idx = make_index();
        idx.add_document(make_doc("1", "report.pdf", "/docs/report.pdf"))
            .unwrap();
        idx.add_document(make_doc(
            "2",
            "presentation.pptx",
            "/docs/presentation.pptx",
        ))
        .unwrap();
        idx.add_document(make_doc("3", "budget.xlsx", "/finance/budget.xlsx"))
            .unwrap();

        let results = idx.search("report");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document_id, "1");
        assert!(results[0].score > 0.0);
    }

    #[test]
    fn test_multiple_matches() {
        let idx = make_index();
        idx.add_document(make_doc("1", "test file", "/docs/test"))
            .unwrap();
        idx.add_document(make_doc("2", "another test", "/tmp/test"))
            .unwrap();

        let results = idx.search("test");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_empty_query() {
        let idx = make_index();
        idx.add_document(make_doc("1", "report.pdf", "/docs/report.pdf"))
            .unwrap();
        let results = idx.search("");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_no_results() {
        let idx = make_index();
        idx.add_document(make_doc("1", "report.pdf", "/docs/report.pdf"))
            .unwrap();
        let results = idx.search("nonexistent_term_xyz");
        assert!(results.is_empty());
    }

    #[test]
    fn test_document_count() {
        let idx = make_index();
        assert_eq!(idx.document_count(), 0);
        idx.add_document(make_doc("1", "a.txt", "/a.txt")).unwrap();
        idx.add_document(make_doc("2", "b.txt", "/b.txt")).unwrap();
        assert_eq!(idx.document_count(), 2);
    }

    #[test]
    fn test_term_count() {
        let idx = make_index();
        idx.add_document(make_doc("1", "hello world", "/hello/world"))
            .unwrap();
        assert!(idx.term_count() > 0);
    }

    #[test]
    fn test_tf_idf_common_terms_score_lower() {
        let idx = make_boosted_index();
        idx.add_document(make_doc("1", "the quick brown fox", "/docs/fox"))
            .unwrap();
        idx.add_document(make_doc("2", "the lazy dog", "/docs/dog"))
            .unwrap();
        idx.add_document(make_doc("3", "a rare unique keyword", "/docs/keyword"))
            .unwrap();

        let common_results = idx.search("the");
        let rare_results = idx.search("unique");

        let avg_common: f64 = if common_results.is_empty() {
            0.0
        } else {
            common_results.iter().map(|r| r.score).sum::<f64>() / common_results.len() as f64
        };
        let avg_rare: f64 = if rare_results.is_empty() {
            0.0
        } else {
            rare_results.iter().map(|r| r.score).sum::<f64>() / rare_results.len() as f64
        };

        assert!(
            avg_rare > avg_common || !rare_results.is_empty(),
            "Rare term 'unique' should score higher than common term 'the'"
        );
    }

    #[test]
    fn test_tf_idf_single_doc_has_nonzero_score() {
        let idx = make_boosted_index();
        idx.add_document(make_doc("1", "uniqueword", "/unique/word"))
            .unwrap();

        let results = idx.search("uniqueword");
        assert_eq!(results.len(), 1);
        assert!(results[0].score > 0.0);
    }

    #[test]
    fn test_phrase_search() {
        let idx = make_index();
        idx.add_document(make_doc("1", "quick brown fox", "/docs/quick/brown/fox"))
            .unwrap();
        idx.add_document(make_doc("2", "brown quick fox", "/docs/brown/quick/fox"))
            .unwrap();
        idx.add_document(make_doc("3", "quick fox", "/docs/quick/fox"))
            .unwrap();

        let results = idx.search("\"quick brown\"");
        assert!(results.iter().any(|r| r.document_id == "1"));
        assert!(
            !results.iter().any(|r| r.document_id == "2"),
            "Phrase 'quick brown' should not match document 2"
        );
        assert!(
            !results.iter().any(|r| r.document_id == "3"),
            "Phrase 'quick brown' should not match document 3"
        );
    }

    #[test]
    fn test_boolean_and_query() {
        let idx = make_index();
        idx.add_document(make_doc("1", "hello world", "/hello/world"))
            .unwrap();
        idx.add_document(make_doc("2", "hello rust", "/hello/rust"))
            .unwrap();
        idx.add_document(make_doc("3", "world rust", "/world/rust"))
            .unwrap();

        let results = idx.search("hello AND world");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document_id, "1");
    }

    #[test]
    fn test_boolean_or_query() {
        let idx = make_index();
        idx.add_document(make_doc("1", "hello world", "/hello/world"))
            .unwrap();
        idx.add_document(make_doc("2", "goodbye", "/goodbye"))
            .unwrap();

        let results = idx.search("hello OR goodbye");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_boolean_not_query() {
        let idx = make_index();
        idx.add_document(make_doc("1", "hello world", "/hello/world"))
            .unwrap();
        idx.add_document(make_doc("2", "goodbye world", "/goodbye/world"))
            .unwrap();

        let results = idx.search("NOT hello");
        assert!(results.iter().any(|r| r.document_id == "2"));
        assert!(
            !results.iter().any(|r| r.document_id == "1"),
            "NOT hello should exclude doc 1"
        );
    }

    #[test]
    fn test_field_query() {
        let idx = make_index();
        idx.add_document(make_doc("1", "report", "/docs/report"))
            .unwrap();
        idx.add_document(make_doc("2", "docs", "/docs/other"))
            .unwrap();

        let results = idx.search("name:report");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document_id, "1");
    }

    #[test]
    fn test_fuzzy_levenshtein_exact() {
        assert_eq!(fuzzy::levenshtein_distance("hello", "hello"), 0);
        assert_eq!(fuzzy::levenshtein_distance("", ""), 0);
    }

    #[test]
    fn test_fuzzy_levenshtein_known() {
        assert_eq!(fuzzy::levenshtein_distance("kitten", "sitting"), 3);
        assert_eq!(fuzzy::levenshtein_distance("saturday", "sunday"), 3);
    }

    #[test]
    fn test_fuzzy_levenshtein_unicode() {
        let d = fuzzy::levenshtein_distance("cafe\u{301}", "cafe");
        assert!(d <= 1);
    }

    #[test]
    fn test_fuzzy_match_function() {
        let result = fuzzy::fuzzy_match("test", "test", 0);
        assert!(result.is_some());
        assert!((result.unwrap() - 1.0).abs() < f64::EPSILON);

        let result = fuzzy::fuzzy_match("test", "tset", 2);
        assert!(result.is_some());
        assert!(result.unwrap() < 1.0);
        assert!(result.unwrap() > 0.0);

        let result = fuzzy::fuzzy_match("abc", "xyz", 1);
        assert!(result.is_none());
    }

    #[test]
    fn test_prefix_match() {
        assert!(fuzzy::prefix_match("rep", "report"));
        assert!(fuzzy::prefix_match("", "anything"));
        assert!(!fuzzy::prefix_match("xyz", "abc"));
    }

    #[test]
    fn test_autocomplete_suggestions() {
        let idx = make_index();
        idx.add_document(make_doc("1", "report.pdf", "/docs/report.pdf"))
            .unwrap();
        idx.add_document(make_doc("2", "report_v2.pdf", "/docs/report_v2.pdf"))
            .unwrap();
        idx.add_document(make_doc("3", "readme.md", "/readme.md"))
            .unwrap();

        let suggestions = idx.suggest("rep", 10);
        assert!(suggestions.contains(&"report".to_string()));
        assert!(suggestions.contains(&"report_v2".to_string()));
    }

    #[test]
    fn test_suggestions_case_insensitive() {
        let idx = make_index();
        idx.add_document(make_doc("1", "HelloWorld", "/hello"))
            .unwrap();

        let suggestions = idx.suggest("hel", 10);
        assert!(suggestions.contains(&"helloworld".to_string()));
    }

    #[test]
    fn test_document_removal() {
        let idx = make_index();
        idx.add_document(make_doc("1", "report.pdf", "/docs/report.pdf"))
            .unwrap();
        assert_eq!(idx.document_count(), 1);

        idx.remove_document("1").unwrap();
        assert_eq!(idx.document_count(), 0);

        let results = idx.search("report");
        assert!(results.is_empty());
    }

    #[test]
    fn test_remove_nonexistent_document() {
        let idx = make_index();
        let result = idx.remove_document("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_document_update() {
        let idx = make_index();
        idx.add_document(make_doc("1", "report.pdf", "/docs/report.pdf"))
            .unwrap();

        let mut new_fields = HashMap::new();
        new_fields.insert("name".to_string(), "updated.pdf".to_string());
        new_fields.insert("path".to_string(), "/docs/updated.pdf".to_string());
        idx.update_document(
            "1",
            DocumentUpdate {
                fields: Some(new_fields),
                metadata: None,
            },
        )
        .unwrap();

        let results = idx.search("updated");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document_id, "1");

        let old_results = idx.search("report");
        assert!(old_results.is_empty());
    }

    #[test]
    fn test_field_boosting() {
        let idx = make_boosted_index();
        idx.add_document(make_doc("1", "unique", "/docs/common"))
            .unwrap();
        idx.add_document(make_doc("2", "common", "/docs/unique"))
            .unwrap();

        let results = idx.search("unique");
        let name_result = results.iter().find(|r| r.document_id == "1");
        let path_result = results.iter().find(|r| r.document_id == "2");

        if let (Some(nr), Some(pr)) = (name_result, path_result) {
            assert!(
                nr.score > pr.score,
                "Name match should score higher due to 2.0 boost: name={}, path={}",
                nr.score,
                pr.score
            );
        }
    }

    #[test]
    fn test_metadata_filtering() {
        let idx = make_index();
        idx.add_document(make_doc("1", "report.pdf", "/docs/report.pdf"))
            .unwrap();
        idx.add_document(make_doc("2", "report.pdf", "/finance/report.pdf"))
            .unwrap();

        let mut filter = SearchFilter::default();
        let mut field_filters = HashMap::new();
        field_filters.insert("type".to_string(), "file".to_string());
        filter.field_filters = field_filters;

        let (results, _) = idx.search_with_filter("report", filter);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_metadata_filter_excludes() {
        let idx = make_index();

        let mut fields = HashMap::new();
        fields.insert("name".to_string(), "report.pdf".to_string());
        fields.insert("path".to_string(), "/docs/report.pdf".to_string());
        let mut meta = HashMap::new();
        meta.insert("type".to_string(), "image".to_string());
        idx.add_document(Document {
            id: "1".to_string(),
            fields,
            metadata: meta,
        })
        .unwrap();

        let mut fields2 = HashMap::new();
        fields2.insert("name".to_string(), "report.pdf".to_string());
        fields2.insert("path".to_string(), "/docs/report2.pdf".to_string());
        let mut meta2 = HashMap::new();
        meta2.insert("type".to_string(), "file".to_string());
        idx.add_document(Document {
            id: "2".to_string(),
            fields: fields2,
            metadata: meta2,
        })
        .unwrap();

        let mut filter = SearchFilter::default();
        let mut ff = HashMap::new();
        ff.insert("type".to_string(), "file".to_string());
        filter.field_filters = ff;

        let (results, _) = idx.search_with_filter("report", filter);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document_id, "2");
    }

    #[test]
    fn test_min_score_filter() {
        let idx = make_boosted_index();
        idx.add_document(make_doc("1", "the common word", "/common"))
            .unwrap();
        idx.add_document(make_doc("2", "rareword unique", "/rare"))
            .unwrap();

        let _all_results = idx.search("common OR rareword");

        let filter = SearchFilter {
            min_score: Some(f64::MAX),
            ..Default::default()
        };
        let (filtered, _) = idx.search_with_filter("common OR rareword", filter);
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_limit_and_offset() {
        let idx = make_index();
        for i in 0..10 {
            let name = format!("document {i}");
            let path = format!("/docs/document_{i}");
            idx.add_document(make_doc(&i.to_string(), &name, &path))
                .unwrap();
        }

        let filter = SearchFilter {
            limit: 3,
            ..Default::default()
        };
        let (results, _) = idx.search_with_filter("document", filter);
        assert_eq!(results.len(), 3);

        let filter2 = SearchFilter {
            offset: 5,
            limit: 100,
            ..Default::default()
        };
        let (results2, _) = idx.search_with_filter("document", filter2);
        assert_eq!(results2.len(), 5);
    }

    #[test]
    fn test_unicode_content() {
        let idx = make_index();
        let mut fields = HashMap::new();
        fields.insert("name".to_string(), "cafe\u{301}.txt".to_string());
        fields.insert("path".to_string(), "/caf\u{e9}.txt".to_string());
        idx.add_document(Document {
            id: "1".to_string(),
            fields,
            metadata: HashMap::new(),
        })
        .unwrap();

        let results = idx.search("caf");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_special_characters() {
        let idx = make_index();
        idx.add_document(make_doc("1", "file (1).txt", "/path/file_(1).txt"))
            .unwrap();
        idx.add_document(make_doc("2", "file (2).txt", "/path/file_(2).txt"))
            .unwrap();

        let results = idx.search("file");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_duplicate_document_id() {
        let idx = make_index();
        idx.add_document(make_doc("1", "first", "/first")).unwrap();
        let result = idx.add_document(make_doc("1", "second", "/second"));
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_document_id() {
        let idx = make_index();
        let result = idx.add_document(Document {
            id: String::new(),
            fields: HashMap::new(),
            metadata: HashMap::new(),
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_matched_fields() {
        let idx = make_index();
        idx.add_document(make_doc("1", "hello", "/docs/hello"))
            .unwrap();

        let results = idx.search("hello");
        assert_eq!(results.len(), 1);
        assert!(results[0].matched_fields.contains(&"name".to_string()));
        assert!(results[0].matched_fields.contains(&"path".to_string()));
    }

    #[test]
    fn test_highlights() {
        let idx = make_index();
        idx.add_document(make_doc("1", "report", "/docs/report"))
            .unwrap();

        let results = idx.search("report");
        assert_eq!(results.len(), 1);
        assert!(results[0].highlights.contains_key("name"));
        let hl = &results[0].highlights["name"];
        assert!(hl.contains("**report**"));
    }

    #[test]
    fn test_concurrent_add_and_search() {
        use std::sync::Arc;
        use std::thread;

        let idx = Arc::new(make_index());
        let mut handles = Vec::new();

        for i in 0..10 {
            let idx_clone = Arc::clone(&idx);
            let handle = thread::spawn(move || {
                let name = format!("document {}", i);
                let path = format!("/docs/document_{}", i);
                idx_clone
                    .add_document(make_doc(&i.to_string(), &name, &path))
                    .unwrap();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(idx.document_count(), 10);

        let search_handles: Vec<_> = (0..5)
            .map(|_| {
                let idx_clone = Arc::clone(&idx);
                thread::spawn(move || {
                    let results = idx_clone.search("document");
                    assert!(!results.is_empty());
                    results.len()
                })
            })
            .collect();

        for h in search_handles {
            let count = h.join().unwrap();
            assert!(count > 0);
        }
    }

    #[test]
    fn test_concurrent_remove() {
        use std::sync::Arc;
        use std::thread;

        let idx = Arc::new(make_index());
        for i in 0..20 {
            let name = format!("document {}", i);
            let path = format!("/docs/document_{}", i);
            idx.add_document(make_doc(&i.to_string(), &name, &path))
                .unwrap();
        }

        let mut handles = Vec::new();
        for i in 0..20 {
            let idx_clone = Arc::clone(&idx);
            handles.push(thread::spawn(move || {
                idx_clone.remove_document(&i.to_string()).unwrap();
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(idx.document_count(), 0);
    }

    #[test]
    fn test_query_parser_term() {
        let q = QueryParser::parse("hello").unwrap();
        matches!(q, Query::Term(t) if t == "hello");
    }

    #[test]
    fn test_query_parser_and() {
        let q = QueryParser::parse("a AND b").unwrap();
        matches!(q, Query::And(_, _));
    }

    #[test]
    fn test_query_parser_or() {
        let q = QueryParser::parse("a OR b").unwrap();
        matches!(q, Query::Or(_, _));
    }

    #[test]
    fn test_query_parser_not() {
        let q = QueryParser::parse("NOT test").unwrap();
        matches!(q, Query::Not(_));
    }

    #[test]
    fn test_query_parser_phrase() {
        let q = QueryParser::parse("\"hello world\"").unwrap();
        matches!(q, Query::Phrase(terms) if terms == vec!["hello", "world"]);
    }

    #[test]
    fn test_query_parser_field() {
        let q = QueryParser::parse("name:value").unwrap();
        matches!(q, Query::Field(f, t) if f == "name" && t == "value");
    }

    #[test]
    fn test_results_sorted_by_score() {
        let idx = make_boosted_index();
        idx.add_document(make_doc("1", "unique keyword", "/common"))
            .unwrap();
        idx.add_document(make_doc("2", "common word", "/common"))
            .unwrap();

        let results = idx.search("unique");
        if results.len() >= 2 {
            for w in results.windows(2) {
                assert!(
                    w[0].score >= w[1].score,
                    "Results should be sorted by score descending"
                );
            }
        }
    }

    #[test]
    fn test_complex_boolean_query() {
        let idx = make_index();
        idx.add_document(make_doc("1", "hello world rust", "/hw/rust"))
            .unwrap();
        idx.add_document(make_doc("2", "hello world python", "/hw/python"))
            .unwrap();
        idx.add_document(make_doc("3", "goodbye rust", "/bye/rust"))
            .unwrap();

        let results = idx.search("hello AND world AND rust");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document_id, "1");
    }

    #[test]
    fn test_reindex_preserves_search() {
        let idx = make_index();
        idx.add_document(make_doc("1", "original.txt", "/original.txt"))
            .unwrap();

        let results_before = idx.search("original");
        assert_eq!(results_before.len(), 1);

        let mut new_fields = HashMap::new();
        new_fields.insert("name".to_string(), "replaced.txt".to_string());
        new_fields.insert("path".to_string(), "/replaced.txt".to_string());
        idx.update_document(
            "1",
            DocumentUpdate {
                fields: Some(new_fields),
                metadata: None,
            },
        )
        .unwrap();

        let old_results = idx.search("original");
        assert!(old_results.is_empty());

        let new_results = idx.search("replaced");
        assert_eq!(new_results.len(), 1);
        assert_eq!(new_results[0].document_id, "1");
    }

    #[test]
    fn test_search_metrics_returns_timing() {
        let idx = make_index();
        idx.add_document(make_doc("1", "hello world", "/hello/world"))
            .unwrap();

        let (_, metrics) = idx.search_with_metrics("hello", SearchFilter::default());
        assert!(metrics.query_time_us > 0);
        assert_eq!(metrics.total_results, 1);
        assert!(!metrics.cache_hit);
    }

    #[test]
    fn test_cache_hit() {
        let idx = make_index();
        idx.add_document(make_doc("1", "hello world", "/hello/world"))
            .unwrap();

        let _ = idx.search("hello");
        let (results, metrics) = idx.search_with_metrics("hello", SearchFilter::default());
        assert!(metrics.cache_hit);
        assert_eq!(results.len(), 1);

        let (hits, misses) = idx.cache_stats();
        assert!(hits >= 1);
        assert!(misses >= 1);
    }

    #[test]
    fn test_cache_invalidated_on_add() {
        let idx = make_index();
        idx.add_document(make_doc("1", "hello", "/hello")).unwrap();

        let _ = idx.search("hello");

        idx.add_document(make_doc("2", "hello world", "/hello/world"))
            .unwrap();

        let (_, metrics) = idx.search_with_metrics("hello", SearchFilter::default());
        assert!(!metrics.cache_hit);
        assert_eq!(metrics.total_results, 2);
    }

    #[test]
    fn test_cache_invalidated_on_remove() {
        let idx = make_index();
        idx.add_document(make_doc("1", "hello", "/hello")).unwrap();
        idx.add_document(make_doc("2", "hello world", "/hello/world"))
            .unwrap();

        let _ = idx.search("hello");

        idx.remove_document("2").unwrap();

        let (_, metrics) = idx.search_with_metrics("hello", SearchFilter::default());
        assert!(!metrics.cache_hit);
        assert_eq!(metrics.total_results, 1);
    }

    #[test]
    fn test_search_paginated() {
        let idx = make_index();
        for i in 0..20 {
            let name = format!("unique_{i} item");
            let path = format!("/items/{i}");
            idx.add_document(make_doc(&i.to_string(), &name, &path))
                .unwrap();
        }

        let (page1, metrics) = idx.search_paginated("item", 0, 5);
        assert_eq!(page1.len(), 5);
        assert!(metrics.query_time_us > 0);

        let (page2, _) = idx.search_paginated("item", 15, 10);
        assert_eq!(page2.len(), 5);

        let (page3, _) = idx.search_paginated("item", 100, 10);
        assert!(page3.is_empty());
    }

    #[test]
    fn test_config_default() {
        let config = SearchIndexConfig::default();
        assert_eq!(config.cache_ttl, std::time::Duration::from_secs(300));
        assert_eq!(config.shard_count, 1);
    }

    #[test]
    fn test_config_accessor() {
        let idx = make_index();
        let config = idx.config();
        assert_eq!(config.shard_count, 1);
    }

    #[test]
    fn test_custom_config() {
        let config = SearchIndexConfig {
            cache_ttl: std::time::Duration::from_secs(60),
            shard_count: 4,
        };
        let idx = SearchIndex::with_config(vec!["name".to_string()], HashMap::new(), config);
        assert_eq!(idx.config().cache_ttl, std::time::Duration::from_secs(60));
        assert_eq!(idx.config().shard_count, 4);
    }
}
