use std::path::{Component, Path, PathBuf};

/// Normalize a path by resolving `.` and `..` components and ensuring a leading `/`.
pub fn normalize_path(path: &str) -> String {
    let path = Path::new(path);
    let mut result = Vec::new();
    for component in path.components() {
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
        format!("/{}", s)
    } else {
        s
    }
}

/// Return the parent directory path, or `None` if already at root.
pub fn parent_path(path: &str) -> Option<String> {
    let normalized = normalize_path(path);
    if normalized == "/" {
        return None;
    }
    let parent = Path::new(&normalized).parent()?;
    Some(normalize_path(&parent.to_string_lossy()))
}

/// Return the final path component (file or directory name).
pub fn base_name(path: &str) -> &str {
    let trimmed = path.trim_end_matches('/');
    Path::new(trimmed)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
}

/// Check whether a path represents a collection (ends with `/`).
pub fn is_collection_path(path: &str) -> bool {
    path.ends_with('/')
}

/// Validate that a path is non-empty and does not contain traversal components.
pub fn validate_path(path: &str) -> bool {
    !path.trim().is_empty() && !normalize_path(path).contains("..")
}

/// Join a base path and a segment, normalizing slashes.
pub fn join_path(base: &str, segment: &str) -> String {
    let base = base.trim_end_matches('/');
    let segment = segment.trim_start_matches('/');
    format!("{}/{}", base, segment)
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
    fn test_parent_path() {
        assert_eq!(parent_path("/foo/bar"), Some("/foo".to_string()));
        assert_eq!(parent_path("/foo"), Some("/".to_string()));
        assert_eq!(parent_path("/"), None);
    }

    #[test]
    fn test_base_name() {
        assert_eq!(base_name("/foo/bar.txt"), "bar.txt");
        assert_eq!(base_name("/foo/"), "foo");
        assert_eq!(base_name("/"), "");
    }

    #[test]
    fn test_validate_path() {
        assert!(validate_path("/foo/bar"));
        assert!(validate_path("/"));
        assert!(!validate_path(""));
    }

    #[test]
    fn test_join_path() {
        assert_eq!(join_path("/foo", "bar"), "/foo/bar");
        assert_eq!(join_path("/foo/", "/bar"), "/foo/bar");
    }
}
