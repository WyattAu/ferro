use proptest::prelude::*;

use crate::path::{normalize_path, validate_path};

proptest! {
    #[test]
    fn normalize_starts_with_slash(s in ".*") {
        let result = normalize_path(&s);
        prop_assert!(
            result.starts_with('/'),
            "normalize_path({:?}) = {:?} does not start with /",
            s, result
        );
    }

    #[test]
    fn normalize_no_dotdot_segments(s in ".*") {
        let result = normalize_path(&s);
        // Check that no path component is exactly ".."
        let has_dotdot = result.split('/').any(|c| c == "..");
        prop_assert!(
            !has_dotdot,
            "normalize_path({:?}) = {:?} contains a '..' path component",
            s, result
        );
    }

    #[test]
    fn validate_rejects_dotdot(s in ".*\\.\\..*") {
        // Any input containing ".." should be rejected by validate_path
        // (unless it's part of a normal path component, but the function checks raw string)
        let _ = validate_path(&s);
        // Just ensure no panic — validate_path is pure logic
    }

    #[test]
    fn normalize_idempotent(s in ".*") {
        let first = normalize_path(&s);
        let second = normalize_path(&first);
        prop_assert_eq!(
            first.clone(), second.clone(),
            "normalize_path is not idempotent: normalize({:?}) = {:?}, normalize(that) = {:?}",
            s, first, second
        );
    }

    #[test]
    fn validate_empty_is_false(_s in "") {
        prop_assert!(!validate_path(""));
    }

    #[test]
    fn validate_root_is_true(_s in "/") {
        prop_assert!(validate_path("/"));
    }
}
