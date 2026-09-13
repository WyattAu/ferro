//! Path validation security seam (`ferro_common::path::validate_path`).
//!
//! `validate_path` is the gate every WebDAV collection/segment operation
//! walks through before touching storage: a SIMD traversal pre-check
//! followed by delegation to the `validkit` estate crate's `ObjectKey`.
//! These tests lock the contract the handlers rely on — traversal,
//! empty/oversized, and legitimate shapes — so a `validkit` rule change
//! that would reject valid user paths (or admit hostile ones) surfaces
//! here first.
//!
//! CONTRACT: the gate operates on DECODED paths. The framework delivers
//! raw percent-encoded request paths, and every handler decodes exactly
//! once (`decode_percent` in webdav.rs / copy_move.rs) BEFORE calling
//! `validate_path` — so `%2e%2e` never reaches this gate encoded. Handler
//! code that validates before decoding would bypass traversal checks.

use ferro_common::path::{is_collection_path, join_path, normalize_path, validate_path};

#[test]
fn legitimate_paths_are_accepted() {
    for ok in [
        "/",
        "/docs",
        "/docs/report.docx",
        "/collections/calendar/events/",
        "/deeply/nested/folder/tree/file.txt",
        "/files/v2/2026/report-final.pdf",
    ] {
        assert!(validate_path(ok), "{ok} should be valid");
    }
}

#[test]
fn traversal_is_rejected() {
    for bad in [
        "/../etc/passwd",
        "/docs/../../secret",
        "/docs/../secret",
        "a/../b",
        "..",
        "/..",
        "/docs/./traversal",
        ".\\docs",
    ] {
        assert!(!validate_path(bad), "{bad} must be rejected");
    }
}

#[test]
fn decoded_traversal_shapes_reject() {
    // Handlers decode percent-encoding before validating (see module docs),
    // so the decoded forms of encoded traversal are what the gate must stop.
    for bad in ["/docs/../secret", "/docs/./secret", "/docs/..\\secret"] {
        assert!(!validate_path(bad), "{bad} must be rejected");
    }
}

#[test]
fn empty_and_whitespace_only_reject() {
    assert!(!validate_path(""));
    assert!(!validate_path("   "));
}

#[test]
fn oversized_paths_reject() {
    // validkit's ObjectKey ceiling is 1024 bytes.
    let long = format!("/{}", "a".repeat(2000));
    assert!(!validate_path(&long), "oversized path must be rejected");

    let boundary_ok = format!("/{}", "a".repeat(1000));
    assert!(validate_path(&boundary_ok), "1 KB path stays valid");
}

#[test]
fn collection_markers_survive_validation_helpers() {
    assert!(is_collection_path("/docs/"));
    assert!(!is_collection_path("/docs"));

    assert_eq!(normalize_path("/docs//sub/"), "/docs/sub");
    assert_eq!(normalize_path("/a/b/../c"), "/a/c");
}

#[test]
fn join_path_keeps_segments_safe() {
    assert_eq!(join_path("/docs", "report.docx"), "/docs/report.docx");
    assert_eq!(join_path("/docs/", "report.docx"), "/docs/report.docx");
    // A traversal segment joined on top must not escape the base.
    let joined = join_path("/docs", "../secret");
    assert!(
        !validate_path(&joined) || !joined.contains(".."),
        "join + validate must not admit traversal: {joined}"
    );
}
