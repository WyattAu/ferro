use ferro_search_index::{SearchIndex, Document};
use std::collections::HashMap;

fn make_doc(id: &str, name: &str, path: &str) -> Document {
    let mut fields = HashMap::new();
    fields.insert("name".to_string(), name.to_string());
    fields.insert("path".to_string(), path.to_string());
    let mut meta = HashMap::new();
    meta.insert("type".to_string(), "image".to_string());
    Document {
        id: id.to_string(),
        fields,
        metadata: meta,
    }
}

fn make_index() -> SearchIndex {
    SearchIndex::new(vec!["name".to_string(), "path".to_string()])
}

#[test]
fn test_index_and_search_file() {
    let idx = make_index();

    let files = [
        ("1", "annual-report-2024.pdf", "/docs/annual-report-2024.pdf"),
        ("2", "meeting-notes.docx", "/docs/meeting-notes.docx"),
        ("3", "budget-spreadsheet.xlsx", "/finance/budget-spreadsheet.xlsx"),
        ("4", "vacation-photo.jpg", "/photos/2024/vacation.jpg"),
        ("5", "project-plan.md", "/projects/project-plan.md"),
        ("6", "api-reference.html", "/docs/api-reference.html"),
        ("7", "database-schema.sql", "/db/database-schema.sql"),
        ("8", "logo.png", "/assets/logo.png"),
        ("9", "readme.txt", "/readme.txt"),
        ("10", "config.json", "/config.json"),
    ];

    for (id, name, path) in &files {
        idx.add_document(make_doc(id, name, path))
            .unwrap();
    }

    assert_eq!(idx.document_count(), 10);

    let results = idx.search("report");
    assert!(
        !results.is_empty(),
        "Search for 'report' should return results"
    );
    assert_eq!(results[0].document_id, "1");
    assert!(results[0].score > 0.0);

    let mut filter = ferro_search_index::SearchFilter::default();
    let mut field_filters = HashMap::new();
    field_filters.insert("type".to_string(), "image".to_string());
    filter.field_filters = field_filters;

    let image_results = idx.search_with_filter("photo", filter);
    assert!(
        !image_results.is_empty(),
        "Search for 'photo' filtered by type=image should return results"
    );

    let all_results = idx.search("file");
    let exact_results: Vec<_> = all_results
        .iter()
        .filter(|r| r.matched_fields.contains(&"name".to_string()))
        .collect();

    assert!(
        all_results.len() >= exact_results.len(),
        "Total results should be >= name-matched results"
    );

    idx.remove_document("1").unwrap();
    assert_eq!(idx.document_count(), 9);

    let after_delete = idx.search("annual report");
    assert!(
        after_delete.is_empty(),
        "Deleted document should not appear in search results"
    );
}

#[test]
fn test_search_ranking_exact_vs_partial() {
    let idx = make_index();

    idx.add_document(make_doc("1", "report", "/report"))
        .unwrap();
    idx.add_document(make_doc("2", "report-final", "/report-final"))
        .unwrap();
    idx.add_document(make_doc("3", "my-report-2024", "/my-report-2024"))
        .unwrap();

    let results = idx.search("report");
    assert!(results.len() >= 2);

    let name_match = results
        .iter()
        .find(|r| r.document_id == "1" && r.matched_fields.contains(&"name".to_string()));
    assert!(
        name_match.is_some(),
        "Exact name match should appear in results"
    );
}

#[test]
fn test_search_phrase_query() {
    let idx = make_index();

    idx.add_document(make_doc("1", "quick brown fox", "/a"))
        .unwrap();
    idx.add_document(make_doc("2", "brown quick fox", "/b"))
        .unwrap();
    idx.add_document(make_doc("3", "the quick fox", "/c"))
        .unwrap();

    let results = idx.search("\"quick brown\"");
    assert!(
        results.iter().any(|r| r.document_id == "1"),
        "Phrase 'quick brown' should match doc 1"
    );
    assert!(
        !results.iter().any(|r| r.document_id == "2"),
        "Phrase 'quick brown' should not match doc 2 (wrong order)"
    );
    assert!(
        !results.iter().any(|r| r.document_id == "3"),
        "Phrase 'quick brown' should not match doc 3 (missing word)"
    );
}

#[test]
fn test_search_boolean_queries() {
    let idx = make_index();

    idx.add_document(make_doc("1", "hello world", "/hw"))
        .unwrap();
    idx.add_document(make_doc("2", "hello rust", "/hr"))
        .unwrap();
    idx.add_document(make_doc("3", "goodbye rust", "/br"))
        .unwrap();

    let and_results = idx.search("hello AND world");
    assert_eq!(and_results.len(), 1);
    assert_eq!(and_results[0].document_id, "1");

    let or_results = idx.search("hello OR goodbye");
    assert_eq!(or_results.len(), 3);

    let not_results = idx.search("NOT goodbye");
    assert!(
        not_results.iter().any(|r| r.document_id == "1"),
        "NOT goodbye should include doc 1"
    );
    assert!(
        !not_results.iter().any(|r| r.document_id == "3"),
        "NOT goodbye should exclude doc 3"
    );
}

#[test]
fn test_search_autocomplete() {
    let idx = make_index();

    idx.add_document(make_doc("1", "report.pdf", "/docs/report.pdf"))
        .unwrap();
    idx.add_document(make_doc("2", "report_v2.pdf", "/docs/report_v2.pdf"))
        .unwrap();
    idx.add_document(make_doc("3", "readme.md", "/readme.md"))
        .unwrap();

    let suggestions = idx.suggest("rep", 10);
    assert!(
        suggestions.contains(&"report".to_string()),
        "Suggestions should contain 'report'"
    );
    assert!(
        suggestions.contains(&"report_v2".to_string()),
        "Suggestions should contain 'report_v2'"
    );
}

#[test]
fn test_search_update_document() {
    let idx = make_index();

    idx.add_document(make_doc("1", "old-name.txt", "/old-name.txt"))
        .unwrap();

    let mut new_fields = HashMap::new();
    new_fields.insert("name".to_string(), "new-name.txt".to_string());
    new_fields.insert("path".to_string(), "/new-name.txt".to_string());
    idx.update_document(
        "1",
        ferro_search_index::DocumentUpdate {
            fields: Some(new_fields),
            metadata: None,
        },
    )
    .unwrap();

    let results = idx.search("old-name");
    assert!(results.is_empty(), "Old name should not match after update");

    let new_results = idx.search("new");
    assert_eq!(new_results.len(), 1, "Updated name should be searchable");
    let name_results = idx.search("name");
    assert_eq!(name_results.len(), 1, "Updated name should be searchable by 'name'");
}

#[test]
fn test_search_pagination() {
    let idx = make_index();

    for i in 0..20 {
        idx.add_document(make_doc(
            &i.to_string(),
            &format!("document-{}", i),
            &format!("/docs/document-{}", i),
        ))
        .unwrap();
    }

    let mut filter = ferro_search_index::SearchFilter::default();
    filter.limit = 5;
    let page1 = idx.search_with_filter("document", filter);
    assert_eq!(page1.len(), 5);

    let mut filter2 = ferro_search_index::SearchFilter::default();
    filter2.offset = 10;
    filter2.limit = 5;
    let page3 = idx.search_with_filter("document", filter2);
    assert!(page3.len() >= 1);
}
