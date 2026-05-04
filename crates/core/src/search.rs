use ferro_common::error::{FerroError, Result};
use ferro_common::metadata::FileMetadata;
use std::path::Path;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{DocAddress, Index, IndexReader, IndexWriter, ReloadPolicy, Score};
use tracing::{debug, info};

/// Full-text search engine backed by Tantivy.
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
}

/// A single search result with relevance score and optional snippet.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub path: String,
    pub score: f64,
    pub snippet: Option<String>,
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
        let searcher = self.reader.searcher();

        let query_parser = QueryParser::for_index(
            &self.index,
            vec![self.content_field, self.name_field, self.path_field],
        );
        let query = query_parser
            .parse_query(query_str)
            .map_err(|e| FerroError::Internal(format!("Query parse error: {}", e)))?;

        let top_docs: Vec<(Score, DocAddress)> = searcher
            .search(&query, &TopDocs::with_limit(limit).order_by_score())
            .map_err(|e| FerroError::Internal(format!("Search failed: {}", e)))?;

        let results: Vec<SearchResult> = top_docs
            .into_iter()
            .map(|(score, doc_addr)| {
                let doc: TantivyDocument = searcher.doc(doc_addr).unwrap_or_default();
                let path = doc
                    .get_first(self.path_field)
                    .and_then(|v| v.as_str())
                    .unwrap_or("<unknown>")
                    .to_string();

                SearchResult {
                    path,
                    score: score as f64,
                    snippet: doc
                        .get_first(self.content_field)
                        .and_then(|v| v.as_str())
                        .map(|s| s.chars().take(200).collect()),
                }
            })
            .collect();

        debug!("Search '{}' returned {} results", query_str, results.len());
        Ok(results)
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
        let hash = ContentHash::new("a".repeat(64));
        FileMetadata::new(path.to_string(), hash, 100, "user1".to_string())
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
}
