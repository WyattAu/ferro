use crate::simd::compare::contains_simd;
use std::borrow::Cow;
use std::path::{Component, Path, PathBuf};

/// Normalize a path by resolving `.` and `..` components and ensuring a leading `/`.
/// Returns a borrowed string if no normalization was needed, or an owned string otherwise.
#[must_use]
pub fn normalize_path(path: &str) -> Cow<'_, str> {
    // Fast path: if path already starts with '/' and contains no '.' or '..' components,
    // no double slashes, and doesn't end with '/' (unless it's just "/")
    if path.starts_with('/') && !path.contains('.') && !path.contains("//") && (path == "/" || !path.ends_with('/')) {
        return Cow::Borrowed(path);
    }

    // Slow path: need to normalize
    let path_obj = Path::new(path);
    let mut result = Vec::new();
    for component in path_obj.components() {
        match component {
            Component::ParentDir => {
                result.pop();
            }
            Component::CurDir => {}
            other => {
                result.push(other);
            }
        }
    }
    let normalized: PathBuf = result.into_iter().collect();
    let s = normalized.to_string_lossy().to_string();
    if s.is_empty() || !s.starts_with('/') {
        Cow::Owned(format!("/{s}"))
    } else {
        Cow::Owned(s)
    }
}

/// Return the parent directory path, or `None` if already at root.
#[must_use]
pub fn parent_path(path: &str) -> Option<String> {
    let normalized = normalize_path(path);
    if normalized == "/" {
        return None;
    }
    let parent = Path::new(normalized.as_ref()).parent()?;
    Some(normalize_path(parent.to_string_lossy().as_ref()).into_owned())
}

/// Return the final path component (file or directory name).
#[must_use]
pub fn base_name(path: &str) -> &str {
    let trimmed = path.trim_end_matches('/');
    Path::new(trimmed).file_name().and_then(|n| n.to_str()).unwrap_or("")
}

/// Check whether a path represents a collection (ends with `/`).
#[must_use]
pub fn is_collection_path(path: &str) -> bool {
    path.ends_with('/')
}

/// Validate that a path is non-empty and does not contain traversal components.
#[must_use]
pub fn validate_path(path: &str) -> bool {
    if path.trim().is_empty() {
        return false;
    }

    #[cfg(target_arch = "x86_64")]
    {
        !contains_simd(path, "..")
            && !contains_simd(path, "./")
            && !contains_simd(path, ".\\")
            && !contains_simd(&normalize_path(path), "..")
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        !path.contains("..") && !path.contains("./") && !path.contains(".\\") && !normalize_path(path).contains("..")
    }
}

/// Join a base path and a segment, normalizing slashes.
/// Returns a borrowed string if possible, or an owned string otherwise.
#[must_use]
pub fn join_path<'a>(base: &'a str, segment: &'a str) -> Cow<'a, str> {
    let base = base.trim_end_matches('/');
    let segment = segment.trim_start_matches('/');
    if base.is_empty() {
        Cow::Owned(format!("/{segment}"))
    } else if segment.is_empty() {
        Cow::Borrowed(base)
    } else {
        Cow::Owned(format!("{base}/{segment}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path("/foo/bar/"), "/foo/bar");
        assert_eq!(normalize_path("/foo/../bar"), "/bar");
        assert_eq!(normalize_path("foo"), "/foo");
        assert_eq!(normalize_path("/"), "/");
    }

    #[test]
    fn test_normalize_path_multiple_dots() {
        assert_eq!(normalize_path("/foo/./bar"), "/foo/bar");
        assert_eq!(normalize_path("/foo/../.."), "/");
        assert_eq!(normalize_path("/a/b/../../c"), "/c");
    }

    #[test]
    fn test_normalize_path_empty() {
        assert_eq!(normalize_path(""), "/");
    }

    #[test]
    fn test_normalize_path_borrowed() {
        // Should return Borrowed for already-normalized paths
        assert!(matches!(normalize_path("/foo/bar"), Cow::Borrowed(_)));
        assert!(matches!(normalize_path("/"), Cow::Borrowed(_)));
        // Should return Owned for paths needing normalization
        assert!(matches!(normalize_path("/foo/./bar"), Cow::Owned(_)));
        assert!(matches!(normalize_path("/foo/../bar"), Cow::Owned(_)));
    }

    #[test]
    fn test_parent_path() {
        assert_eq!(parent_path("/foo/bar"), Some("/foo".to_string()));
        assert_eq!(parent_path("/foo"), Some("/".to_string()));
        assert_eq!(parent_path("/"), None);
    }

    #[test]
    fn test_parent_path_deep() {
        assert_eq!(parent_path("/a/b/c/d"), Some("/a/b/c".to_string()));
    }

    #[test]
    fn test_base_name() {
        assert_eq!(base_name("/foo/bar.txt"), "bar.txt");
        assert_eq!(base_name("/foo/"), "foo");
        assert_eq!(base_name("/"), "");
    }

    #[test]
    fn test_base_name_no_extension() {
        assert_eq!(base_name("/foo/README"), "README");
    }

    #[test]
    fn test_validate_path() {
        assert!(validate_path("/foo/bar"));
        assert!(validate_path("/"));
        assert!(!validate_path(""));
    }

    #[test]
    fn test_validate_path_traversal() {
        assert!(!validate_path("/foo/../bar"));
        assert!(!validate_path("/foo/../../bar"));
    }

    #[test]
    fn test_validate_path_whitespace() {
        assert!(!validate_path("   "));
    }

    #[test]
    fn test_join_path() {
        assert_eq!(join_path("/foo", "bar"), "/foo/bar");
        assert_eq!(join_path("/foo/", "/bar"), "/foo/bar");
    }

    #[test]
    fn test_join_path_empty_segments() {
        assert_eq!(join_path("", "bar"), "/bar");
        assert_eq!(join_path("/foo", ""), "/foo");
    }

    #[test]
    fn test_join_path_borrowed() {
        // Should return Borrowed when base is non-empty and segment is empty
        assert!(matches!(join_path("/foo", ""), Cow::Borrowed(_)));
        // Should return Owned when both are non-empty
        assert!(matches!(join_path("/foo", "bar"), Cow::Owned(_)));
    }

    #[test]
    fn test_is_collection_path() {
        assert!(is_collection_path("/foo/"));
        assert!(!is_collection_path("/foo"));
        assert!(is_collection_path("/"));
    }
}
