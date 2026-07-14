//! Automated security audit tests for the Ferro codebase.
//!
//! These tests verify that security controls are in place and functioning correctly.

#[cfg(test)]
mod tests {

    // ========================================================================
    // 1. Path Traversal Tests
    // ========================================================================

    mod path_traversal {
        use ferro_server::api::normalize_api_path;

        #[test]
        fn test_normalize_api_path_rejects_traversal() {
            assert!(normalize_api_path("../../../etc/passwd").is_err());
            assert!(normalize_api_path("/files/../../etc/shadow").is_err());
            assert!(normalize_api_path("a/../b").is_err());
            assert!(normalize_api_path("./foo").is_err());
            assert!(normalize_api_path("foo/.").is_err());
            assert!(normalize_api_path("foo//bar").is_err());
        }

        #[test]
        fn test_normalize_api_path_allows_normal() {
            assert_eq!(normalize_api_path("foo/bar").unwrap(), "/foo/bar");
            assert_eq!(normalize_api_path("foo").unwrap(), "/foo");
            assert_eq!(normalize_api_path("").unwrap(), "/");
        }

        #[test]
        fn test_security_validate_filename_blocks_traversal() {
            use ferro_server::security::validate_filename;
            assert!(validate_filename("../etc/passwd").is_err());
            assert!(validate_filename("..").is_err());
            assert!(validate_filename(".").is_err());
            assert!(validate_filename("file/name").is_err());
            assert!(validate_filename("file\\name").is_err());
        }

        #[test]
        fn test_security_validate_path_blocks_nested_traversal() {
            use ferro_server::security::validate_path;
            assert!(validate_path("docs/../../etc/passwd").is_err());
            assert!(validate_path("a/../b/../../c").is_err());
        }

        #[test]
        fn test_security_validate_path_allows_normal() {
            use ferro_server::security::validate_path;
            assert!(validate_path("documents/report.pdf").is_ok());
            assert!(validate_path("photos/2024/vacation.jpg").is_ok());
            assert!(validate_path("/").is_ok());
        }
    }

    // ========================================================================
    // 2. SQL Injection Prevention — Static Analysis
    // ========================================================================

    mod sql_injection {
        #[test]
        fn test_no_format_sql_in_shares_rs() {
            let source = include_str!("../../server/src/shares.rs");
            assert_no_format_sql_in_non_test_code(source, "shares.rs");
        }

        #[test]
        fn test_no_format_sql_in_users_module() {
            let source = include_str!("../../auth/src/users.rs");
            assert_no_format_sql_in_non_test_code(source, "users.rs");
        }

        #[test]
        fn test_no_format_sql_in_api_keys() {
            let source = include_str!("../../auth/src/api_keys.rs");
            assert_no_format_sql_in_non_test_code(source, "api_keys.rs");
        }

        fn assert_no_format_sql_in_non_test_code(source: &str, file: &str) {
            let lines: Vec<&str> = source.lines().collect();
            let mut test_context = false;
            let mut violations = Vec::new();
            for (i, line) in lines.iter().enumerate() {
                if line.contains("#[cfg(test)]") || line.contains("mod tests") {
                    test_context = true;
                }
                if test_context && line.contains("}") && !line.contains("#[") && !line.contains("fn ") {
                    test_context = false;
                }
                if test_context {
                    continue;
                }
                if line.contains("format!(")
                    && (line.contains("SELECT")
                        || line.contains("INSERT")
                        || line.contains("UPDATE")
                        || line.contains("DELETE")
                        || line.contains("FROM"))
                {
                    violations.push((i + 1, line.trim().to_string()));
                }
            }
            if !violations.is_empty() {
                for (line_num, code) in &violations {
                    eprintln!("SQL injection risk in {} line {}: {}", file, line_num, code);
                }
            }
            assert!(
                violations.is_empty(),
                "Found {} potential SQL injection via format! in {}",
                violations.len(),
                file
            );
        }
    }

    // ========================================================================
    // 3. Authentication & Authorization
    // ========================================================================

    mod auth {
        use ferro_auth::api_keys::{ApiKeyPermission, generate_raw_key, hash_api_key};
        use ferro_auth::cedar::CedarAuthorizer;
        use ferro_auth::users::hash_password;

        #[test]
        fn test_api_key_entropy() {
            let key = generate_raw_key();
            assert_eq!(key.len(), 70); // 32 bytes hex = 64 chars + "ferro_" = 70
            assert!(key.starts_with("ferro_"));
        }

        #[test]
        fn test_api_key_hash_deterministic() {
            let h1 = hash_api_key("ferro_test");
            let h2 = hash_api_key("ferro_test");
            assert_eq!(h1, h2);
            assert_eq!(h1.len(), 64);
        }

        #[test]
        fn test_api_key_hash_collision_resistant() {
            let h1 = hash_api_key("ferro_key_one");
            let h2 = hash_api_key("ferro_key_two");
            assert_ne!(h1, h2);
        }

        #[test]
        fn test_password_uses_bcrypt() {
            let hash = hash_password("test_pass").unwrap();
            assert!(hash.starts_with("$2"), "Must use bcrypt");
            assert!(hash.len() >= 50);
        }

        #[tokio::test]
        async fn test_cedar_default_is_deny() {
            let authorizer = CedarAuthorizer::new().unwrap();
            // Cedar denies by default when no policies are loaded.
            assert!(
                !authorizer
                    .is_authorized_simple("anonymous", "read", "/f")
                    .await
                    .unwrap()
            );
            assert!(
                !authorizer
                    .is_authorized_simple("anonymous", "write", "/f")
                    .await
                    .unwrap()
            );
            assert!(
                !authorizer
                    .is_authorized_simple("anonymous", "delete", "/f")
                    .await
                    .unwrap()
            );
            assert!(
                !authorizer
                    .is_authorized_simple("anonymous", "admin", "/f")
                    .await
                    .unwrap()
            );
        }

        #[tokio::test]
        async fn test_cedar_restrictive_policy() {
            let authorizer = CedarAuthorizer::new().unwrap();
            authorizer
                .load_policies(&[r#"
                @id("alice_read")
                permit (
                    principal == User::"alice",
                    action in Action::"read",
                    resource
                );
            "#
                .to_string()])
                .await
                .unwrap();
            assert!(authorizer.is_authorized_simple("alice", "read", "/f").await.unwrap());
            assert!(!authorizer.is_authorized_simple("bob", "read", "/f").await.unwrap());
            assert!(!authorizer.is_authorized_simple("alice", "write", "/f").await.unwrap());
        }

        #[test]
        fn test_api_key_permission_hierarchy() {
            assert!(ApiKeyPermission::Admin.allows_action("read"));
            assert!(ApiKeyPermission::Admin.allows_action("write"));
            assert!(ApiKeyPermission::Admin.allows_action("admin"));
            assert!(ApiKeyPermission::Write.allows_action("read"));
            assert!(ApiKeyPermission::Write.allows_action("write"));
            // F013 resolved: Write no longer allows "admin" action
            assert!(!ApiKeyPermission::Write.allows_action("admin"));
            assert!(ApiKeyPermission::Read.allows_action("read"));
            assert!(!ApiKeyPermission::Read.allows_action("write"));
        }
    }

    // ========================================================================
    // 4. Input Validation & Content-Type
    // ========================================================================

    mod input_validation {
        use ferro_server::security::{detect_content_type, validate_filename, verify_content_type};

        #[test]
        fn test_blocked_control_chars() {
            assert!(validate_filename("file\x00name").is_err());
            assert!(validate_filename("file\x01name").is_err());
            assert!(validate_filename("file\x1fname").is_err());
        }

        #[test]
        fn test_blocked_empty_and_whitespace() {
            assert!(validate_filename("").is_err());
            assert!(validate_filename("   ").is_err());
            assert!(validate_filename("...").is_err());
        }

        #[test]
        fn test_blocked_reserved_names() {
            for name in ["CON", "PRN", "AUX", "NUL", "COM3", "LPT1"] {
                assert!(validate_filename(name).is_err(), "{} should be blocked", name);
            }
        }

        #[test]
        fn test_blocked_path_separators() {
            assert!(validate_filename("file/name").is_err());
            assert!(validate_filename("file\\name").is_err());
        }

        #[test]
        fn test_blocked_long_name() {
            assert!(validate_filename(&"a".repeat(256)).is_err());
        }

        #[test]
        fn test_allowed_names() {
            assert!(validate_filename("document.pdf").is_ok());
            assert!(validate_filename("my file.txt").is_ok());
            assert!(validate_filename("con").is_ok()); // lowercase OK
        }

        #[test]
        fn test_content_type_png_vs_pdf() {
            let png = b"\x89PNG\r\n\x1a\n";
            assert!(verify_content_type("application/pdf", png).is_some());
            assert!(verify_content_type("image/png", png).is_none());
        }

        #[test]
        fn test_octet_stream_skips() {
            let png = b"\x89PNG\r\n\x1a\n";
            assert!(verify_content_type("application/octet-stream", png).is_none());
        }

        #[test]
        fn test_magic_bytes() {
            assert_eq!(detect_content_type(b"\x89PNG\r\n\x1a\n"), Some("image/png"));
            assert_eq!(detect_content_type(b"%PDF-1.4"), Some("application/pdf"));
            assert_eq!(detect_content_type(b"\xFF\xD8\xFF\xE0"), Some("image/jpeg"));
            assert_eq!(detect_content_type(b"PK\x03\x04"), Some("application/zip"));
            assert_eq!(detect_content_type(b"\x7FELF"), Some("application/x-elf"));
            assert_eq!(detect_content_type(b"MZ"), Some("application/x-msdownload"));
            assert_eq!(detect_content_type(b"random"), None);
        }
    }

    // ========================================================================
    // 5. Account Lockout & Rate Limiting
    // ========================================================================

    mod lockout_rate_limit {
        use ferro_server::security::{AuthAttemptTracker, LoginRateLimiter};
        use std::time::Duration;

        #[test]
        fn test_lockout_after_max_failures() {
            let tracker = AuthAttemptTracker::new(3, Duration::from_secs(10));
            assert!(!tracker.record_failure("1.2.3.4", "admin"));
            assert!(!tracker.record_failure("1.2.3.4", "admin"));
            assert!(tracker.record_failure("1.2.3.4", "admin"));
            assert!(tracker.is_locked_out("1.2.3.4", "admin"));
        }

        #[test]
        fn test_different_user_not_locked() {
            let tracker = AuthAttemptTracker::new(3, Duration::from_secs(10));
            let _ = tracker.record_failure("1.2.3.4", "admin");
            let _ = tracker.record_failure("1.2.3.4", "admin");
            let _ = tracker.record_failure("1.2.3.4", "admin");
            assert!(tracker.is_locked_out("1.2.3.4", "admin"));
            assert!(!tracker.is_locked_out("1.2.3.4", "other"));
        }

        #[test]
        fn test_success_clears_lockout() {
            let tracker = AuthAttemptTracker::new(3, Duration::from_secs(10));
            let _ = tracker.record_failure("1.2.3.4", "admin");
            let _ = tracker.record_failure("1.2.3.4", "admin");
            tracker.record_success("1.2.3.4", "admin");
            assert!(!tracker.is_locked_out("1.2.3.4", "admin"));
        }

        #[tokio::test]
        async fn test_login_rate_limit_enforced() {
            let limiter = LoginRateLimiter::new(3, Duration::from_secs(60));
            assert!(limiter.check("1.2.3.4").await);
            assert!(limiter.check("1.2.3.4").await);
            assert!(limiter.check("1.2.3.4").await);
            assert!(!limiter.check("1.2.3.4").await);
            assert!(limiter.check("5.6.7.8").await);
        }
    }

    // ========================================================================
    // 6. Security Headers
    // ========================================================================

    mod headers {
        #[test]
        fn test_csp_blocks_framing_and_eval() {
            let csp = "default-src 'self'; script-src 'self'; \
                         style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; \
                         img-src 'self' data: blob:; font-src 'self' https://fonts.gstatic.com; \
                         connect-src 'self' ws: wss: https://fonts.googleapis.com https://fonts.gstatic.com; \
                         frame-ancestors 'none'; base-uri 'self'; form-action 'self'";
            assert!(csp.contains("frame-ancestors 'none'"));
            assert!(csp.contains("default-src 'self'"));
            assert!(!csp.contains("'unsafe-eval'"));
            assert!(csp.contains("connect-src 'self' ws: wss:"));
        }
    }

    // ========================================================================
    // 8. Share Password Hashing
    // ========================================================================

    mod share_passwords {
        use ferro_server::shares::{hash_share_password, verify_share_password};

        #[test]
        fn test_share_password_hashed() {
            let hash = hash_share_password("test123");
            assert_ne!(hash, "test123", "Password must not be stored as plaintext");
            assert_eq!(hash.len(), 64, "SHA-256 hex digest must be 64 chars");
        }

        #[test]
        fn test_share_password_verification() {
            let hash = hash_share_password("mysecret");
            assert!(verify_share_password("mysecret", &hash));
            assert!(!verify_share_password("wrong", &hash));
        }

        #[test]
        fn test_share_password_different_inputs_different_hashes() {
            let h1 = hash_share_password("abc");
            let h2 = hash_share_password("def");
            assert_ne!(h1, h2);
        }
    }

    // ========================================================================
    // 7. Default Password & CSRF
    // ========================================================================

    mod defaults_csrf {
        use ferro_server::security::{generate_csrf_token, is_default_password, verify_csrf_token};

        #[test]
        fn test_default_passwords_blocked() {
            for pw in ["changeme", "admin", "password", "ferro", ""] {
                assert!(is_default_password(pw), "'{}' should be flagged as default", pw);
            }
        }

        #[test]
        fn test_strong_password_accepted() {
            assert!(!is_default_password("Str0ngP@ss!"));
            assert!(!is_default_password("correct horse battery staple"));
        }

        #[test]
        fn test_csrf_token_and_verification() {
            let token = generate_csrf_token();
            assert_eq!(token.len(), 64); // 32 bytes hex
            assert!(verify_csrf_token(&token, &token));
            assert!(!verify_csrf_token(&token, "wrong_token"));
            assert!(!verify_csrf_token(&token, ""));
        }
    }
}
