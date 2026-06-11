use ferro_common::error::{FerroError, Result};
use ferro_common::metadata::FileMetadata;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{DocAddress, Index, IndexReader, IndexWriter, ReloadPolicy, Score};
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRankingConfig {
    pub file_name_boost: f64,
    pub path_boost: f64,
    pub content_boost: f64,
    pub recent_file_boost: f64,
    pub recent_file_threshold_days: u64,
    pub document_type_boost: f64,
}

impl Default for SearchRankingConfig {
    fn default() -> Self {
        Self {
            file_name_boost: 3.0,
            path_boost: 2.0,
            content_boost: 1.0,
            recent_file_boost: 1.2,
            recent_file_threshold_days: 7,
            document_type_boost: 1.1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatchLocation {
    Name,
    Path,
    Content,
}

#[non_exhaustive]
#[derive(Debug, Clone, Default)]
pub struct MatchLocations {
    pub name: bool,
    pub path: bool,
    pub content: bool,
}

impl MatchLocations {
    pub fn to_vec(&self) -> Vec<MatchLocation> {
        let mut v = Vec::new();
        if self.name {
            v.push(MatchLocation::Name);
        }
        if self.path {
            v.push(MatchLocation::Path);
        }
        if self.content {
            v.push(MatchLocation::Content);
        }
        v
    }
}

#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub path: String,
    pub score: f64,
    pub snippet: Option<String>,
    pub normalized_score: f64,
    pub highlights: Vec<String>,
    pub match_locations: MatchLocations,
}

#[non_exhaustive]
pub struct SearchEngine {
    index: Index,
    writer: IndexWriter,
    reader: IndexReader,
    _schema: Schema,
    path_field: Field,
    path_id_field: Field,
    content_field: Field,
    name_field: Field,
    mime_field: Field,
    owner_field: Field,
    modified_field: Field,
}

impl SearchEngine {
    /// Create a new search index at the given directory path.
    pub fn new(index_path: &Path) -> Result<Self> {
        let mut schema_builder = Schema::builder();

        let path_field = schema_builder.add_text_field("path", TEXT | STORED);
        let path_id_field = schema_builder.add_text_field("path_id", STRING | STORED);
        let name_field = schema_builder.add_text_field("name", TEXT | STORED);
        let mime_field = schema_builder.add_text_field("mime_type", TEXT | STORED);
        let owner_field = schema_builder.add_text_field("owner", TEXT | STORED);
        let content_field = schema_builder.add_text_field("content", TEXT | STORED);
        let modified_field = schema_builder.add_date_field("modified_at", STORED | FAST);

        let schema = schema_builder.build();

        let index = Index::create_in_dir(index_path, schema.clone())
            .map_err(|e| FerroError::Internal(format!("Failed to create search index: {}", e)))?;

        let writer = index
            .writer(50_000_000)
            .map_err(|e| FerroError::Internal(format!("Failed to create index writer: {}", e)))?;

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| FerroError::Internal(format!("Failed to create index reader: {}", e)))?;

        info!("Search engine initialized at {:?}", index_path);

        Ok(Self {
            index,
            writer,
            reader,
            _schema: schema,
            path_field,
            path_id_field,
            content_field,
            name_field,
            mime_field,
            owner_field,
            modified_field,
        })
    }

    /// Open an existing search index.
    pub fn open(index_path: &Path) -> Result<Self> {
        let index = Index::open_in_dir(index_path)
            .map_err(|e| FerroError::Internal(format!("Failed to open search index: {}", e)))?;

        let schema = index.schema();
        let path_field = schema
            .get_field("path")
            .map_err(|_| FerroError::Internal("Missing 'path' field in schema".to_string()))?;
        let path_id_field = schema
            .get_field("path_id")
            .map_err(|_| FerroError::Internal("Missing 'path_id' field in schema".to_string()))?;
        let content_field = schema
            .get_field("content")
            .map_err(|_| FerroError::Internal("Missing 'content' field in schema".to_string()))?;
        let name_field = schema
            .get_field("name")
            .map_err(|_| FerroError::Internal("Missing 'name' field in schema".to_string()))?;
        let mime_field = schema
            .get_field("mime_type")
            .map_err(|_| FerroError::Internal("Missing 'mime_type' field in schema".to_string()))?;
        let owner_field = schema
            .get_field("owner")
            .map_err(|_| FerroError::Internal("Missing 'owner' field in schema".to_string()))?;
        let modified_field = schema.get_field("modified_at").map_err(|_| {
            FerroError::Internal("Missing 'modified_at' field in schema".to_string())
        })?;

        let writer = index
            .writer(50_000_000)
            .map_err(|e| FerroError::Internal(format!("Failed to create index writer: {}", e)))?;

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| FerroError::Internal(format!("Failed to create index reader: {}", e)))?;

        Ok(Self {
            index,
            writer,
            reader,
            _schema: schema,
            path_field,
            path_id_field,
            content_field,
            name_field,
            mime_field,
            owner_field,
            modified_field,
        })
    }

    /// Index a file's metadata (path, name, mime type, owner).
    pub fn index_metadata(&mut self, metadata: &FileMetadata) -> Result<()> {
        let mut doc = TantivyDocument::new();
        let name = metadata.path.rsplit('/').next().unwrap_or("");

        doc.add_text(self.path_field, &metadata.path);
        doc.add_text(self.path_id_field, &metadata.path);
        doc.add_text(self.name_field, name);
        doc.add_text(self.mime_field, &metadata.mime_type);
        doc.add_text(self.owner_field, &metadata.owner);
        doc.add_date(
            self.modified_field,
            tantivy::DateTime::from_timestamp_micros(metadata.modified_at.timestamp_micros()),
        );

        self.writer
            .add_document(doc)
            .map_err(|e| FerroError::Internal(format!("Failed to index document: {}", e)))?;

        debug!("Indexed metadata for: {}", metadata.path);
        Ok(())
    }

    /// Index a file's metadata and full text content.
    pub fn index_content(&mut self, metadata: &FileMetadata, content: &str) -> Result<()> {
        let mut doc = TantivyDocument::new();
        let name = metadata.path.rsplit('/').next().unwrap_or("");

        doc.add_text(self.path_field, &metadata.path);
        doc.add_text(self.path_id_field, &metadata.path);
        doc.add_text(self.name_field, name);
        doc.add_text(self.mime_field, &metadata.mime_type);
        doc.add_text(self.owner_field, &metadata.owner);
        doc.add_text(self.content_field, content);
        doc.add_date(
            self.modified_field,
            tantivy::DateTime::from_timestamp_micros(metadata.modified_at.timestamp_micros()),
        );

        self.writer
            .add_document(doc)
            .map_err(|e| FerroError::Internal(format!("Failed to index content: {}", e)))?;

        debug!(
            "Indexed content for: {} ({} bytes)",
            metadata.path,
            content.len()
        );
        Ok(())
    }

    /// Remove a document from the index by path.
    pub fn remove(&mut self, path: &str) -> Result<()> {
        let term = Term::from_field_text(self.path_id_field, path);
        let _opstamp = self.writer.delete_term(term);
        debug!("Removed from index: {}", path);
        Ok(())
    }

    /// Commit pending index changes and reload the reader.
    pub fn commit(&mut self) -> Result<()> {
        self.writer
            .commit()
            .map_err(|e| FerroError::Internal(format!("Failed to commit index: {}", e)))?;
        self.reader
            .reload()
            .map_err(|e| FerroError::Internal(format!("Failed to reload reader: {}", e)))?;
        Ok(())
    }

    /// Search the index for matching documents.
    pub fn search(&self, query_str: &str, limit: usize) -> Result<Vec<SearchResult>> {
        self.search_with_config(query_str, limit, &SearchRankingConfig::default())
    }

    pub fn search_with_config(
        &self,
        query_str: &str,
        limit: usize,
        config: &SearchRankingConfig,
    ) -> Result<Vec<SearchResult>> {
        let searcher = self.reader.searcher();

        let query_parser = QueryParser::for_index(
            &self.index,
            vec![self.content_field, self.name_field, self.path_field],
        );
        let query = query_parser
            .parse_query(query_str)
            .map_err(|e| FerroError::Internal(format!("Query parse error: {}", e)))?;

        let top_docs: Vec<(Score, DocAddress)> = searcher
            .search(&query, &TopDocs::with_limit(limit * 3).order_by_score())
            .map_err(|e| FerroError::Internal(format!("Search failed: {}", e)))?;

        let now_epoch = chrono::Utc::now().timestamp();
        let recent_threshold_secs = (config.recent_file_threshold_days as i64) * 86400;

        let query_lower = query_str.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        let mut results: Vec<SearchResult> = top_docs
            .into_iter()
            .map(|(score, doc_addr)| {
                let doc: TantivyDocument = searcher.doc(doc_addr).unwrap_or_default();
                let path = doc
                    .get_first(self.path_field)
                    .and_then(|v| v.as_str())
                    .unwrap_or("<unknown>")
                    .to_string();
                let name = doc
                    .get_first(self.name_field)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_lowercase();
                let path_lower = path.to_lowercase();
                let mime = doc
                    .get_first(self.mime_field)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let mut match_locs = MatchLocations::default();
                let mut boost = 1.0f64;

                let name_matches = query_terms.iter().all(|t| name.contains(t));
                let path_matches = query_terms.iter().any(|t| path_lower.contains(t));
                let content_text = doc
                    .get_first(self.content_field)
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let content_matches = query_terms
                    .iter()
                    .any(|t| content_text.to_lowercase().contains(t));

                if name_matches {
                    match_locs.name = true;
                    boost *= config.file_name_boost;
                    if name == query_lower {
                        boost *= 5.0;
                    } else if name.starts_with(&query_lower) || name.ends_with(&query_lower) {
                        boost *= 3.0;
                    }
                }
                if path_matches {
                    match_locs.path = true;
                    if !name_matches {
                        boost *= config.path_boost;
                    }
                }
                if content_matches {
                    match_locs.content = true;
                    if !name_matches && !path_matches {
                        boost *= config.content_boost;
                    }
                }

                let is_document = Self::is_document_type(&mime);
                if is_document {
                    boost *= config.document_type_boost;
                }

                let modified_ts = doc
                    .get_first(self.modified_field)
                    .and_then(|v| v.as_datetime())
                    .map(|d| d.into_timestamp_micros() / 1_000_000)
                    .unwrap_or(0);
                if now_epoch - modified_ts < recent_threshold_secs {
                    boost *= config.recent_file_boost;
                }

                let final_score = (score as f64) * boost;

                let snippet: Option<String> = if content_matches && !content_text.is_empty() {
                    Some(content_text.chars().take(200).collect())
                } else {
                    None
                };

                let highlights = Self::extract_highlights(content_text, &query_terms);

                SearchResult {
                    path,
                    score: final_score,
                    snippet,
                    normalized_score: 0.0,
                    highlights,
                    match_locations: match_locs,
                }
            })
            .collect();

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(limit);

        let max_score = results.first().map(|r| r.score).unwrap_or(1.0).max(1.0);
        for r in &mut results {
            r.normalized_score = ((r.score / max_score) * 100.0).min(100.0);
        }

        debug!(
            "Search '{}' returned {} results (config: name_boost={}, path_boost={}, content_boost={})",
            query_str,
            results.len(),
            config.file_name_boost,
            config.path_boost,
            config.content_boost
        );
        Ok(results)
    }

    fn is_document_type(mime: &str) -> bool {
        let document_prefixes = [
            "text/",
            "application/pdf",
            "application/msword",
            "application/vnd.",
            "application/rtf",
            "application/json",
            "application/xml",
            "application/javascript",
        ];
        document_prefixes.iter().any(|p| mime.starts_with(p))
    }

    fn extract_highlights(content: &str, query_terms: &[&str]) -> Vec<String> {
        if content.is_empty() || query_terms.is_empty() {
            return Vec::new();
        }
        let content_lower = content.to_lowercase();
        let mut highlights = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for term in query_terms {
            if let Some(pos) = content_lower.find(term) {
                let start = pos.saturating_sub(40);
                let end = (pos + term.len() + 60).min(content.len());
                let fragment = &content[start..end];
                if seen.insert(fragment.to_string()) {
                    highlights.push(fragment.to_string());
                }
            }
        }
        highlights.truncate(3);
        highlights
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferro_common::metadata::ContentHash;
    use tempfile::TempDir;

    fn make_search_engine() -> (SearchEngine, TempDir) {
        let tmp = TempDir::new().unwrap();
        let engine = SearchEngine::new(tmp.path()).unwrap();
        (engine, tmp)
    }

    fn make_metadata(path: &str) -> FileMetadata {
        let hash = ContentHash::new("a".repeat(64)).expect("valid hardcoded hash");
        FileMetadata::new(path.to_string(), hash, 100, "user1".to_string())
    }

    fn make_metadata_with_modified(
        path: &str,
        modified_at: chrono::DateTime<chrono::Utc>,
    ) -> FileMetadata {
        let hash = ContentHash::new("a".repeat(64)).expect("valid hardcoded hash");
        let mut meta = FileMetadata::new(path.to_string(), hash, 100, "user1".to_string());
        meta.modified_at = modified_at;
        meta
    }

    #[test]
    fn test_index_and_search() {
        let (mut engine, _tmp) = make_search_engine();

        let meta1 = make_metadata("/documents/report.pdf");
        let meta2 = make_metadata("/documents/budget.xlsx");
        let meta3 = make_metadata("/photos/vacation.jpg");

        engine.index_metadata(&meta1).unwrap();
        engine.index_metadata(&meta2).unwrap();
        engine.index_metadata(&meta3).unwrap();
        engine.commit().unwrap();

        let results = engine.search("report", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].path.contains("report"));
    }

    #[test]
    fn test_search_content() {
        let (mut engine, _tmp) = make_search_engine();

        let meta = make_metadata("/docs/readme.txt");
        engine
            .index_content(&meta, "Ferro is a high-performance storage orchestrator")
            .unwrap();
        engine.commit().unwrap();

        let results = engine.search("storage orchestrator", 10).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_remove_document() {
        let (mut engine, _tmp) = make_search_engine();

        let meta = make_metadata("/temp/file.txt");
        engine.index_metadata(&meta).unwrap();
        engine.commit().unwrap();

        assert_eq!(engine.search("file", 10).unwrap().len(), 1);

        engine.remove("/temp/file.txt").unwrap();
        engine.commit().unwrap();

        assert_eq!(engine.search("file", 10).unwrap().len(), 0);
    }

    #[test]
    fn test_search_no_results() {
        let (mut engine, _tmp) = make_search_engine();

        let meta = make_metadata("/docs/readme.txt");
        engine.index_metadata(&meta).unwrap();
        engine.commit().unwrap();

        let results = engine.search("nonexistent_query_xyz", 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_file_name_match_boosted_over_content_match() {
        let (mut engine, _tmp) = make_search_engine();

        let report_meta = make_metadata("/docs/Annual_Report.pdf");
        engine
            .index_content(&report_meta, "some general information here")
            .unwrap();

        let status_meta = make_metadata("/docs/status.txt");
        engine
            .index_content(&status_meta, "this is the quarterly report for the team")
            .unwrap();

        engine.commit().unwrap();

        let results = engine.search("report", 10).unwrap();
        assert!(results.len() >= 2);
        assert!(
            results[0].path.contains("Report"),
            "Annual_Report.pdf should rank highest, got: {}",
            results[0].path
        );
        assert!(results[0].match_locations.name);
    }

    #[test]
    fn test_recent_file_boosted_over_old() {
        let (mut engine, _tmp) = make_search_engine();

        let recent = chrono::Utc::now();
        let old = chrono::Utc::now() - chrono::Duration::days(30);

        let recent_meta = make_metadata_with_modified("/docs/recent_report.txt", recent);
        engine
            .index_content(&recent_meta, "project report data analysis")
            .unwrap();

        let old_meta = make_metadata_with_modified("/docs/old_report.txt", old);
        engine
            .index_content(&old_meta, "project report data analysis")
            .unwrap();

        engine.commit().unwrap();

        let results = engine.search("report", 10).unwrap();
        assert!(results.len() >= 2);
        assert!(
            results[0].path.contains("recent_report"),
            "recent file should rank higher, got: {}",
            results[0].path
        );
    }

    #[test]
    fn test_exact_name_match_highest() {
        let (mut engine, _tmp) = make_search_engine();

        let exact_meta = make_metadata("/docs/report.pdf");
        engine
            .index_content(&exact_meta, "general content about various things")
            .unwrap();

        let partial_meta = make_metadata("/docs/my_report_draft.txt");
        engine
            .index_content(
                &partial_meta,
                "report content with many details about reporting",
            )
            .unwrap();

        engine.commit().unwrap();

        let results = engine.search("report", 10).unwrap();
        assert!(
            results[0].path.ends_with("report.pdf"),
            "exact name match should rank highest, got: {}",
            results[0].path
        );
    }

    #[test]
    fn test_multi_term_both_terms_higher() {
        let (mut engine, _tmp) = make_search_engine();

        let both_meta = make_metadata("/docs/project_report.txt");
        engine
            .index_content(&both_meta, "the project report is complete")
            .unwrap();

        let single_meta = make_metadata("/docs/project.txt");
        engine
            .index_content(&single_meta, "the project is ongoing without any report")
            .unwrap();

        engine.commit().unwrap();

        let results = engine.search("project report", 10).unwrap();
        assert!(
            results[0].path.contains("project_report"),
            "file with both terms in name should rank highest, got: {}",
            results[0].path
        );
    }

    #[test]
    fn test_path_match_boosted_for_query() {
        let (mut engine, _tmp) = make_search_engine();

        let path_meta = make_metadata("/documents/report/summary.txt");
        engine
            .index_content(&path_meta, "quarterly summary of financial data")
            .unwrap();

        let other_meta = make_metadata("/misc/data.txt");
        engine
            .index_content(&other_meta, "this contains important data about reports")
            .unwrap();

        engine.commit().unwrap();

        let results = engine.search("report", 10).unwrap();
        assert!(
            !results.is_empty(),
            "should have results for 'report' query"
        );
        assert!(
            results[0].path.contains("report"),
            "path containing 'report' should rank highest, got: {}",
            results[0].path
        );
    }

    #[test]
    fn test_normalized_score_range() {
        let (mut engine, _tmp) = make_search_engine();

        let meta = make_metadata("/docs/report.txt");
        engine.index_content(&meta, "report content here").unwrap();
        engine.commit().unwrap();

        let results = engine.search("report", 10).unwrap();
        assert!(!results.is_empty());
        assert!(
            results[0].normalized_score > 0.0 && results[0].normalized_score <= 100.0,
            "normalized score should be in 0-100 range, got: {}",
            results[0].normalized_score
        );
    }

    #[test]
    fn test_highlights_extracted() {
        let (mut engine, _tmp) = make_search_engine();

        let meta = make_metadata("/docs/readme.txt");
        engine
            .index_content(
                &meta,
                "the quarterly financial report shows growth in revenue",
            )
            .unwrap();
        engine.commit().unwrap();

        let results = engine.search("financial report", 10).unwrap();
        assert!(!results.is_empty());
        assert!(
            !results[0].highlights.is_empty(),
            "should extract highlight fragments"
        );
    }

    #[test]
    fn test_match_locations_populated() {
        let (mut engine, _tmp) = make_search_engine();

        let name_meta = make_metadata("/docs/report.txt");
        engine
            .index_content(&name_meta, "general information")
            .unwrap();

        let content_meta = make_metadata("/docs/notes.txt");
        engine
            .index_content(&content_meta, "the report contains important data")
            .unwrap();

        engine.commit().unwrap();

        let results = engine.search("report", 10).unwrap();
        let name_result = results
            .iter()
            .find(|r| r.path.contains("report.txt"))
            .unwrap();
        assert!(name_result.match_locations.name);

        let content_result = results
            .iter()
            .find(|r| r.path.contains("notes.txt"))
            .unwrap();
        assert!(content_result.match_locations.content);
    }

    #[test]
    fn test_ranking_config_custom() {
        let (mut engine, _tmp) = make_search_engine();

        let report_meta = make_metadata("/docs/Annual_Report.pdf");
        engine.index_content(&report_meta, "some info").unwrap();

        let status_meta = make_metadata("/docs/status.txt");
        engine
            .index_content(&status_meta, "report data here")
            .unwrap();

        engine.commit().unwrap();

        let config = SearchRankingConfig {
            file_name_boost: 10.0,
            path_boost: 1.0,
            content_boost: 0.1,
            ..Default::default()
        };
        let results = engine.search_with_config("report", 10, &config).unwrap();
        assert!(
            results[0].path.contains("Report"),
            "with high name boost, name match should win: {}",
            results[0].path
        );
    }
}
