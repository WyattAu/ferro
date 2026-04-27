use common::auth::Claims;

/// Resolve a path to a user-specific home directory when multi-user mode is enabled.
pub fn resolve_user_path(path: &str, claims: Option<&Claims>) -> String {
    match claims {
        Some(c) if c.sub != "anonymous" => {
            let user_root = format!("/users/{}", c.sub);
            if path == "/" || path.is_empty() {
                return user_root;
            }
            format!("{}{}", user_root, path)
        }
        _ => path.to_string(),
    }
}

/// Check whether a user has access to the given path in multi-user mode.
pub fn can_access_path(path: &str, claims: Option<&Claims>) -> bool {
    match claims {
        Some(c) if c.sub != "anonymous" => {
            let prefix = format!("/users/{}/", c.sub);
            path == format!("/users/{}", c.sub) || path.starts_with(&prefix)
        }
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::auth::Claims;

    fn alice_claims() -> Claims {
        Claims {
            sub: "alice".to_string(),
            aud: "ferro".to_string(),
            iss: "ferro".to_string(),
            exp: 0,
            iat: 0,
            nonce: None,
            email: None,
            name: None,
            groups: Some(vec!["users".to_string()]),
        }
    }

    fn anonymous_claims() -> Claims {
        Claims::anonymous()
    }

    #[test]
    fn test_resolve_anonymous() {
        assert_eq!(resolve_user_path("/docs/file.txt", None), "/docs/file.txt");
        assert_eq!(resolve_user_path("/docs/file.txt", Some(&anonymous_claims())), "/docs/file.txt");
    }

    #[test]
    fn test_resolve_authenticated() {
        assert_eq!(
            resolve_user_path("/docs/file.txt", Some(&alice_claims())),
            "/users/alice/docs/file.txt"
        );
        assert_eq!(
            resolve_user_path("/", Some(&alice_claims())),
            "/users/alice"
        );
    }

    #[test]
    fn test_can_access_authenticated() {
        assert!(can_access_path("/users/alice/docs/file.txt", Some(&alice_claims())));
        assert!(can_access_path("/users/alice", Some(&alice_claims())));
        assert!(!can_access_path("/users/bob/docs/file.txt", Some(&alice_claims())));
    }

    #[test]
    fn test_can_access_anonymous() {
        assert!(can_access_path("/docs/file.txt", None));
        assert!(can_access_path("/users/alice/docs/file.txt", Some(&anonymous_claims())));
    }
}
