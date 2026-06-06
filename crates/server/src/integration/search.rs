use std::collections::HashMap;

use ferro_search_index::{Document, SearchFilter, SearchIndex};

pub fn create_file_search_index() -> SearchIndex {
    let fields = vec![
        "name".to_string(),
        "path".to_string(),
        "content_type".to_string(),
        "extension".to_string(),
    ];
    let mut boosts = HashMap::new();
    boosts.insert("name".to_string(), 2.0);
    boosts.insert("path".to_string(), 1.0);
    boosts.insert("content_type".to_string(), 0.5);
    boosts.insert("extension".to_string(), 1.5);
    SearchIndex::with_boosts(fields, boosts)
}

pub fn index_file(
    index: &SearchIndex,
    file_id: &str,
    name: &str,
    path: &str,
    content_type: &str,
    size: u64,
) {
    let mut fields = HashMap::new();
    fields.insert("name".to_string(), name.to_string());
    fields.insert("path".to_string(), path.to_string());
    fields.insert("content_type".to_string(), content_type.to_string());
    if let Some(ext) = std::path::Path::new(name).extension() {
        fields.insert("extension".to_string(), ext.to_string_lossy().to_string());
    }

    let mut metadata = HashMap::new();
    metadata.insert("size".to_string(), size.to_string());

    let doc = Document {
        id: file_id.to_string(),
        fields,
        metadata,
    };
    let _ = index.add_document(doc);
}

pub fn search_files(
    index: &SearchIndex,
    query: &str,
    limit: usize,
) -> Vec<ferro_search_index::SearchResult> {
    let filter = SearchFilter {
        field_filters: HashMap::new(),
        min_score: Some(0.1),
        limit,
        offset: 0,
    };
    let (results, _metrics) = index.search_with_filter(query, filter);
    results
}

pub fn deindex_file(index: &SearchIndex, file_id: &str) {
    let _ = index.remove_document(file_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_index() {
        let index = create_file_search_index();
        assert_eq!(index.document_count(), 0);
    }

    #[test]
    fn test_index_and_search() {
        let index = create_file_search_index();
        index_file(
            &index,
            "f1",
            "report.pdf",
            "/docs/report.pdf",
            "application/pdf",
            2048,
        );
        index_file(
            &index,
            "f2",
            "budget.xlsx",
            "/finance/budget.xlsx",
            "application/vnd.ms-excel",
            1024,
        );

        let results = search_files(&index, "report", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document_id, "f1");
    }

    #[test]
    fn test_deindex() {
        let index = create_file_search_index();
        index_file(
            &index,
            "f1",
            "readme.md",
            "/readme.md",
            "text/markdown",
            100,
        );
        assert_eq!(index.document_count(), 1);
        deindex_file(&index, "f1");
        assert_eq!(index.document_count(), 0);
    }

    #[test]
    fn test_extension_indexing() {
        let index = create_file_search_index();
        index_file(
            &index,
            "f1",
            "photo.jpg",
            "/photos/photo.jpg",
            "image/jpeg",
            500,
        );
        let results = search_files(&index, "jpg", 10);
        assert_eq!(results.len(), 1);
    }
}
