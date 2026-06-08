//! Integration tests for mobile API contracts.
//!
//! These tests verify the serialization and deserialization of API types,
//! sync checkpoint protocol, push notification payloads, and file provider
//! interface contracts.

use chrono::{DateTime, Utc};
use ferro_mobile_contract::api::*;
use ferro_mobile_contract::notifications::*;
use ferro_mobile_contract::sync::*;

// ============================================================================
// API Contract Type Tests
// ============================================================================

#[test]
fn test_auth_request_roundtrip() {
    let req = MobileAuthRequest {
        username: "testuser".to_string(),
        password: Some("password123".to_string()),
        token: None,
    };
    let json = serde_json::to_string(&req).unwrap();
    let de: MobileAuthRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(de.username, req.username);
    assert_eq!(de.password, req.password);
    assert!(de.token.is_none());
}

#[test]
fn test_auth_request_token_only() {
    let req = MobileAuthRequest {
        username: "testuser".to_string(),
        password: None,
        token: Some("jwt-token-here".to_string()),
    };
    let json = serde_json::to_string(&req).unwrap();
    let de: MobileAuthRequest = serde_json::from_str(&json).unwrap();
    assert!(de.password.is_none());
    assert_eq!(de.token.as_deref(), Some("jwt-token-here"));
}

#[test]
fn test_auth_response_roundtrip() {
    let resp = MobileAuthResponse {
        access_token: "access-123".to_string(),
        refresh_token: "refresh-456".to_string(),
        expires_in: 3600,
        user: MobileUser {
            id: "user-1".to_string(),
            username: "testuser".to_string(),
            display_name: "Test User".to_string(),
            storage_quota_bytes: 10_737_418_240,
            storage_used_bytes: 1_073_741_824,
        },
    };
    let json = serde_json::to_string(&resp).unwrap();
    let de: MobileAuthResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(de.access_token, "access-123");
    assert_eq!(de.refresh_token, "refresh-456");
    assert_eq!(de.expires_in, 3600);
    assert_eq!(de.user.storage_quota_bytes, 10_737_418_240);
}

#[test]
fn test_file_list_request_defaults() {
    let json = r#"{
        "path": "/Documents"
    }"#;
    let req: FileListRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.path, "/Documents");
    assert_eq!(req.depth, 1);
    assert!(req.include_metadata);
    assert_eq!(req.page_size, 100);
    assert!(req.cursor.is_none());
}

#[test]
fn test_file_list_request_custom() {
    let req = FileListRequest {
        path: "/Photos".to_string(),
        depth: 2,
        include_metadata: false,
        page_size: 50,
        cursor: Some("abc123".to_string()),
    };
    let json = serde_json::to_string(&req).unwrap();
    let de: FileListRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(de.depth, 2);
    assert!(!de.include_metadata);
    assert_eq!(de.page_size, 50);
    assert_eq!(de.cursor.as_deref(), Some("abc123"));
}

#[test]
fn test_file_list_response_pagination() {
    let resp = FileListResponse {
        files: vec![MobileFile {
            path: "/Documents/file.txt".to_string(),
            name: "file.txt".to_string(),
            is_dir: false,
            size: 1024,
            modified: Utc::now(),
            created: Utc::now(),
            content_type: Some("text/plain".to_string()),
            etag: "\"abc123\"".to_string(),
            permissions: FilePermissions::default(),
        }],
        next_cursor: Some("next-page-token".to_string()),
    };
    let json = serde_json::to_string(&resp).unwrap();
    let de: FileListResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(de.files.len(), 1);
    assert_eq!(de.next_cursor.as_deref(), Some("next-page-token"));
}

#[test]
fn test_upload_request_roundtrip() {
    let req = UploadRequest {
        path: "/Documents/report.pdf".to_string(),
        content_type: "application/pdf".to_string(),
        size: 2048000,
        overwrite: true,
    };
    let json = serde_json::to_string(&req).unwrap();
    let de: UploadRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(de.path, req.path);
    assert_eq!(de.content_type, req.content_type);
    assert!(de.overwrite);
}

#[test]
fn test_upload_response_roundtrip() {
    let resp = UploadResponse {
        path: "/Documents/report.pdf".to_string(),
        etag: "\"def456\"".to_string(),
        size: 2048000,
    };
    let json = serde_json::to_string(&resp).unwrap();
    let de: UploadResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(de.etag, "\"def456\"");
    assert_eq!(de.size, 2048000);
}

#[test]
fn test_download_request_range() {
    let req = DownloadRequest {
        path: "/Videos/movie.mp4".to_string(),
        range_start: Some(1024),
        range_end: Some(4095),
    };
    let json = serde_json::to_string(&req).unwrap();
    let de: DownloadRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(de.range_start, Some(1024));
    assert_eq!(de.range_end, Some(4095));
}

#[test]
fn test_create_folder_request() {
    let req = CreateFolderRequest {
        path: "/Documents/New Folder".to_string(),
    };
    let json = serde_json::to_string(&req).unwrap();
    let de: CreateFolderRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(de.path, "/Documents/New Folder");
}

#[test]
fn test_delete_request_recursive() {
    let req = DeleteRequest {
        path: "/Photos/Vacation".to_string(),
        recursive: true,
    };
    let json = serde_json::to_string(&req).unwrap();
    let de: DeleteRequest = serde_json::from_str(&json).unwrap();
    assert!(de.recursive);
}

#[test]
fn test_move_request_roundtrip() {
    let req = MoveRequest {
        from: "/Documents/old-name.txt".to_string(),
        to: "/Documents/new-name.txt".to_string(),
        overwrite: false,
    };
    let json = serde_json::to_string(&req).unwrap();
    let de: MoveRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(de.from, "/Documents/old-name.txt");
    assert_eq!(de.to, "/Documents/new-name.txt");
    assert!(!de.overwrite);
}

#[test]
fn test_share_create_request() {
    let req = ShareCreateRequest {
        path: "/Photos".to_string(),
        expiry_hours: Some(72),
        allow_download: true,
        allow_upload: false,
        password: Some("secure123".to_string()),
    };
    let json = serde_json::to_string(&req).unwrap();
    let de: ShareCreateRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(de.expiry_hours, Some(72));
    assert!(de.allow_download);
    assert!(!de.allow_upload);
}

#[test]
fn test_share_info_roundtrip() {
    let resp = ShareInfo {
        id: "share-123".to_string(),
        token: "token-abc".to_string(),
        url: "https://share.ferro.io/s/abc".to_string(),
        expires_at: Some(Utc::now()),
    };
    let json = serde_json::to_string(&resp).unwrap();
    let de: ShareInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(de.id, "share-123");
    assert!(de.url.starts_with("https://"));
}

#[test]
fn test_file_permissions_variants() {
    let read_only = FilePermissions {
        can_read: true,
        can_write: false,
        can_delete: false,
        can_share: false,
    };
    let json = serde_json::to_string(&read_only).unwrap();
    let de: FilePermissions = serde_json::from_str(&json).unwrap();
    assert!(de.can_read);
    assert!(!de.can_write);
    assert!(!de.can_delete);
}

// ============================================================================
// Sync Checkpoint Protocol Tests
// ============================================================================

#[test]
fn test_sync_checkpoint_roundtrip() {
    let checkpoint = SyncCheckpoint {
        cursor: "abc123def456".to_string(),
        last_sync: Utc::now(),
        device_id: "device-iphone-001".to_string(),
    };
    let json = serde_json::to_string(&checkpoint).unwrap();
    let de: SyncCheckpoint = serde_json::from_str(&json).unwrap();
    assert_eq!(de.cursor, "abc123def456");
    assert_eq!(de.device_id, "device-iphone-001");
}

#[test]
fn test_changes_since_request() {
    let req = ChangesSinceRequest {
        cursor: "last-cursor-token".to_string(),
        path_prefix: Some("/Documents".to_string()),
    };
    let json = serde_json::to_string(&req).unwrap();
    let de: ChangesSinceRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(de.cursor, "last-cursor-token");
    assert_eq!(de.path_prefix.as_deref(), Some("/Documents"));
}

#[test]
fn test_changes_since_request_no_prefix() {
    let req = ChangesSinceRequest {
        cursor: "cursor-1".to_string(),
        path_prefix: None,
    };
    let json = serde_json::to_string(&req).unwrap();
    let de: ChangesSinceRequest = serde_json::from_str(&json).unwrap();
    assert!(de.path_prefix.is_none());
}

#[test]
fn test_file_change_created() {
    let change = FileChange {
        path: "/Documents/new-file.txt".to_string(),
        change_type: ChangeType::Created,
        etag: Some("\"new-etag\"".to_string()),
        size: Some(512),
        modified: Utc::now(),
    };
    let json = serde_json::to_string(&change).unwrap();
    let de: FileChange = serde_json::from_str(&json).unwrap();
    assert!(matches!(de.change_type, ChangeType::Created));
    assert_eq!(de.size, Some(512));
}

#[test]
fn test_file_change_modified() {
    let change = FileChange {
        path: "/Documents/existing.txt".to_string(),
        change_type: ChangeType::Modified,
        etag: Some("\"updated-etag\"".to_string()),
        size: Some(1024),
        modified: Utc::now(),
    };
    let json = serde_json::to_string(&change).unwrap();
    let de: FileChange = serde_json::from_str(&json).unwrap();
    assert!(matches!(de.change_type, ChangeType::Modified));
}

#[test]
fn test_file_change_deleted() {
    let change = FileChange {
        path: "/Documents/deleted.txt".to_string(),
        change_type: ChangeType::Deleted,
        etag: None,
        size: None,
        modified: Utc::now(),
    };
    let json = serde_json::to_string(&change).unwrap();
    let de: FileChange = serde_json::from_str(&json).unwrap();
    assert!(matches!(de.change_type, ChangeType::Deleted));
    assert!(de.etag.is_none());
}

#[test]
fn test_file_change_moved() {
    let change = FileChange {
        path: "/Documents/old-name.txt".to_string(),
        change_type: ChangeType::Moved {
            new_path: "/Documents/new-name.txt".to_string(),
        },
        etag: Some("\"moved-etag\"".to_string()),
        size: None,
        modified: Utc::now(),
    };
    let json = serde_json::to_string(&change).unwrap();
    let de: FileChange = serde_json::from_str(&json).unwrap();
    if let ChangeType::Moved { new_path } = de.change_type {
        assert_eq!(new_path, "/Documents/new-name.txt");
    } else {
        panic!("Expected ChangeType::Moved");
    }
}

#[test]
fn test_changes_since_response() {
    let resp = ChangesSinceResponse {
        changes: vec![
            FileChange {
                path: "/file1.txt".to_string(),
                change_type: ChangeType::Created,
                etag: Some("\"etag1\"".to_string()),
                size: Some(100),
                modified: Utc::now(),
            },
            FileChange {
                path: "/file2.txt".to_string(),
                change_type: ChangeType::Deleted,
                etag: None,
                size: None,
                modified: Utc::now(),
            },
        ],
        new_cursor: "new-cursor-token".to_string(),
        has_more: false,
    };
    let json = serde_json::to_string(&resp).unwrap();
    let de: ChangesSinceResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(de.changes.len(), 2);
    assert!(!de.has_more);
    assert_eq!(de.new_cursor, "new-cursor-token");
}

#[test]
fn test_sync_upload_batch() {
    let batch = SyncUploadBatch {
        files: vec![
            SyncFileUpload {
                path: "/file1.txt".to_string(),
                content_type: "text/plain".to_string(),
                size: 100,
                etag: None,
            },
            SyncFileUpload {
                path: "/file2.jpg".to_string(),
                content_type: "image/jpeg".to_string(),
                size: 2048,
                etag: Some("\"existing-etag\"".to_string()),
            },
        ],
    };
    let json = serde_json::to_string(&batch).unwrap();
    let de: SyncUploadBatch = serde_json::from_str(&json).unwrap();
    assert_eq!(de.files.len(), 2);
    assert!(de.files[1].etag.is_some());
}

#[test]
fn test_sync_upload_response() {
    let resp = SyncUploadResponse {
        results: vec![
            SyncUploadResult {
                path: "/file1.txt".to_string(),
                status: SyncUploadStatus::Created,
                etag: "\"new-etag\"".to_string(),
            },
            SyncUploadResult {
                path: "/file2.txt".to_string(),
                status: SyncUploadStatus::Conflict,
                etag: "\"server-etag\"".to_string(),
            },
        ],
    };
    let json = serde_json::to_string(&resp).unwrap();
    let de: SyncUploadResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(de.results.len(), 2);
    assert!(matches!(de.results[0].status, SyncUploadStatus::Created));
    assert!(matches!(de.results[1].status, SyncUploadStatus::Conflict));
}

#[test]
fn test_sync_upload_status_all_variants() {
    let statuses = vec![
        SyncUploadStatus::Created,
        SyncUploadStatus::Updated,
        SyncUploadStatus::Conflict,
        SyncUploadStatus::QuotaExceeded,
    ];
    for status in statuses {
        let json = serde_json::to_string(&status).unwrap();
        let de: SyncUploadStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{:?}", status), format!("{:?}", de));
    }
}

// ============================================================================
// Push Notification Payload Tests
// ============================================================================

#[test]
fn test_push_token_registration() {
    let reg = PushTokenRegistration {
        device_id: "device-001".to_string(),
        platform: MobilePlatform::Ios,
        push_token: "apns-token-xyz".to_string(),
        app_version: "1.2.3".to_string(),
    };
    let json = serde_json::to_string(&reg).unwrap();
    let de: PushTokenRegistration = serde_json::from_str(&json).unwrap();
    assert!(matches!(de.platform, MobilePlatform::Ios));
    assert_eq!(de.push_token, "apns-token-xyz");
}

#[test]
fn test_push_token_registration_android() {
    let reg = PushTokenRegistration {
        device_id: "device-002".to_string(),
        platform: MobilePlatform::Android,
        push_token: "fcm-token-abc".to_string(),
        app_version: "2.0.0".to_string(),
    };
    let json = serde_json::to_string(&reg).unwrap();
    let de: PushTokenRegistration = serde_json::from_str(&json).unwrap();
    assert!(matches!(de.platform, MobilePlatform::Android));
}

#[test]
fn test_notification_payload_file_shared() {
    let payload = NotificationPayload {
        event_type: NotificationEvent::FileShared,
        path: Some("/Documents/report.pdf".to_string()),
        actor: Some("user@example.com".to_string()),
        timestamp: Utc::now(),
    };
    let json = serde_json::to_string(&payload).unwrap();
    let de: NotificationPayload = serde_json::from_str(&json).unwrap();
    assert!(matches!(de.event_type, NotificationEvent::FileShared));
    assert_eq!(de.path.as_deref(), Some("/Documents/report.pdf"));
}

#[test]
fn test_notification_payload_share_received() {
    let payload = NotificationPayload {
        event_type: NotificationEvent::ShareReceived,
        path: Some("/Shared/file.txt".to_string()),
        actor: Some("sender@example.com".to_string()),
        timestamp: Utc::now(),
    };
    let json = serde_json::to_string(&payload).unwrap();
    let de: NotificationPayload = serde_json::from_str(&json).unwrap();
    assert!(matches!(de.event_type, NotificationEvent::ShareReceived));
}

#[test]
fn test_notification_payload_quota_warning() {
    let payload = NotificationPayload {
        event_type: NotificationEvent::QuotaWarning { percent_used: 85 },
        path: None,
        actor: None,
        timestamp: Utc::now(),
    };
    let json = serde_json::to_string(&payload).unwrap();
    let de: NotificationPayload = serde_json::from_str(&json).unwrap();
    if let NotificationEvent::QuotaWarning { percent_used } = de.event_type {
        assert_eq!(percent_used, 85);
    } else {
        panic!("Expected QuotaWarning");
    }
}

#[test]
fn test_notification_payload_sync_conflict() {
    let payload = NotificationPayload {
        event_type: NotificationEvent::SyncConflict {
            path: "/Documents/notes.txt".to_string(),
        },
        path: None,
        actor: None,
        timestamp: Utc::now(),
    };
    let json = serde_json::to_string(&payload).unwrap();
    let de: NotificationPayload = serde_json::from_str(&json).unwrap();
    if let NotificationEvent::SyncConflict { path } = de.event_type {
        assert_eq!(path, "/Documents/notes.txt");
    } else {
        panic!("Expected SyncConflict");
    }
}

#[test]
fn test_notification_payload_comment_added() {
    let payload = NotificationPayload {
        event_type: NotificationEvent::CommentAdded {
            file_path: "/Documents/design.md".to_string(),
        },
        path: None,
        actor: Some("reviewer@example.com".to_string()),
        timestamp: Utc::now(),
    };
    let json = serde_json::to_string(&payload).unwrap();
    let de: NotificationPayload = serde_json::from_str(&json).unwrap();
    if let NotificationEvent::CommentAdded { file_path } = de.event_type {
        assert_eq!(file_path, "/Documents/design.md");
    } else {
        panic!("Expected CommentAdded");
    }
}

#[test]
fn test_notification_event_tagged_enum() {
    let events = vec![
        NotificationEvent::FileShared,
        NotificationEvent::ShareReceived,
        NotificationEvent::QuotaWarning { percent_used: 90 },
        NotificationEvent::SyncConflict {
            path: "/test.txt".to_string(),
        },
        NotificationEvent::CommentAdded {
            file_path: "/doc.md".to_string(),
        },
    ];
    for event in events {
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\""), "Event should have type tag");
        let de: NotificationEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{:?}", event), format!("{:?}", de));
    }
}

// ============================================================================
// File Provider Interface Tests
// ============================================================================

#[test]
fn test_file_provider_listing() {
    let files = vec![
        MobileFile {
            path: "/Documents".to_string(),
            name: "Documents".to_string(),
            is_dir: true,
            size: 0,
            modified: Utc::now(),
            created: Utc::now(),
            content_type: None,
            etag: "\"dir-etag\"".to_string(),
            permissions: FilePermissions::default(),
        },
        MobileFile {
            path: "/Documents/file.txt".to_string(),
            name: "file.txt".to_string(),
            is_dir: false,
            size: 512,
            modified: Utc::now(),
            created: Utc::now(),
            content_type: Some("text/plain".to_string()),
            etag: "\"file-etag\"".to_string(),
            permissions: FilePermissions {
                can_read: true,
                can_write: true,
                can_delete: true,
                can_share: true,
            },
        },
    ];
    let resp = FileListResponse {
        files,
        next_cursor: None,
    };
    let json = serde_json::to_string(&resp).unwrap();
    let de: FileListResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(de.files.len(), 2);
    assert!(de.files[0].is_dir);
    assert!(!de.files[1].is_dir);
    assert!(de.files[1].permissions.can_share);
}

#[test]
fn test_file_provider_metadata() {
    let file = MobileFile {
        path: "/Photos/image.jpg".to_string(),
        name: "image.jpg".to_string(),
        is_dir: false,
        size: 2_097_152,
        modified: DateTime::parse_from_rfc3339("2025-01-15T10:30:00Z")
            .unwrap()
            .to_utc(),
        created: DateTime::parse_from_rfc3339("2025-01-10T08:00:00Z")
            .unwrap()
            .to_utc(),
        content_type: Some("image/jpeg".to_string()),
        etag: "\"etag-jpg\"".to_string(),
        permissions: FilePermissions::default(),
    };
    let json = serde_json::to_string(&file).unwrap();
    let de: MobileFile = serde_json::from_str(&json).unwrap();
    assert_eq!(de.size, 2_097_152);
    assert_eq!(de.content_type.as_deref(), Some("image/jpeg"));
    assert_eq!(de.name, "image.jpg");
}

#[test]
fn test_file_provider_etag_format() {
    let file = MobileFile {
        path: "/test.txt".to_string(),
        name: "test.txt".to_string(),
        is_dir: false,
        size: 100,
        modified: Utc::now(),
        created: Utc::now(),
        content_type: None,
        etag: "\"abc123\"".to_string(),
        permissions: FilePermissions::default(),
    };
    assert!(file.etag.starts_with('"'));
    assert!(file.etag.ends_with('"'));
}

#[test]
fn test_file_provider_pagination() {
    let resp = FileListResponse {
        files: vec![],
        next_cursor: Some("page2-cursor".to_string()),
    };
    let json = serde_json::to_string(&resp).unwrap();
    let de: FileListResponse = serde_json::from_str(&json).unwrap();
    assert!(de.files.is_empty());
    assert_eq!(de.next_cursor.as_deref(), Some("page2-cursor"));
}

#[test]
fn test_file_provider_deep_listing() {
    let req = FileListRequest {
        path: "/".to_string(),
        depth: 3,
        include_metadata: true,
        page_size: 200,
        cursor: None,
    };
    let json = serde_json::to_string(&req).unwrap();
    let de: FileListRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(de.depth, 3);
    assert_eq!(de.page_size, 200);
}

#[test]
fn test_file_provider_conflict_detection() {
    let batch_resp = SyncUploadResponse {
        results: vec![
            SyncUploadResult {
                path: "/file1.txt".to_string(),
                status: SyncUploadStatus::Created,
                etag: "\"new-etag\"".to_string(),
            },
            SyncUploadResult {
                path: "/file2.txt".to_string(),
                status: SyncUploadStatus::Conflict,
                etag: "\"server-etag\"".to_string(),
            },
        ],
    };
    let json = serde_json::to_string(&batch_resp).unwrap();
    let de: SyncUploadResponse = serde_json::from_str(&json).unwrap();

    let conflicts: Vec<_> = de
        .results
        .iter()
        .filter(|r| matches!(r.status, SyncUploadStatus::Conflict))
        .collect();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].path, "/file2.txt");
}

#[test]
fn test_file_provider_quota_exceeded() {
    let result = SyncUploadResult {
        path: "/large-file.bin".to_string(),
        status: SyncUploadStatus::QuotaExceeded,
        etag: String::new(),
    };
    let json = serde_json::to_string(&result).unwrap();
    let de: SyncUploadResult = serde_json::from_str(&json).unwrap();
    assert!(matches!(de.status, SyncUploadStatus::QuotaExceeded));
}

// ============================================================================
// Error Contract Tests
// ============================================================================

#[test]
fn test_mobile_api_error_display() {
    let errors = vec![
        ferro_mobile_contract::error::MobileApiError::Unauthorized,
        ferro_mobile_contract::error::MobileApiError::Forbidden,
        ferro_mobile_contract::error::MobileApiError::NotFound {
            resource: "file.txt".to_string(),
        },
        ferro_mobile_contract::error::MobileApiError::Conflict {
            reason: "version mismatch".to_string(),
        },
        ferro_mobile_contract::error::MobileApiError::QuotaExceeded,
        ferro_mobile_contract::error::MobileApiError::ServerError {
            code: 500,
            message: "internal error".to_string(),
        },
        ferro_mobile_contract::error::MobileApiError::NetworkError {
            reason: "connection refused".to_string(),
        },
        ferro_mobile_contract::error::MobileApiError::SyncConflict {
            path: "/test.txt".to_string(),
        },
    ];
    for error in errors {
        let msg = error.to_string();
        assert!(!msg.is_empty(), "Error message should not be empty");
    }
}

#[test]
fn test_mobile_api_error_is_error() {
    let err = ferro_mobile_contract::error::MobileApiError::Unauthorized;
    let boxed: Box<dyn std::error::Error> = Box::new(err);
    assert!(!boxed.to_string().is_empty());
}

// ============================================================================
// Cross-cutting Integration Tests
// ============================================================================

#[test]
fn test_full_sync_workflow() {
    // 1. Create sync checkpoint
    let checkpoint = SyncCheckpoint {
        cursor: "initial".to_string(),
        last_sync: Utc::now(),
        device_id: "test-device".to_string(),
    };

    // 2. Request changes
    let changes_req = ChangesSinceRequest {
        cursor: checkpoint.cursor.clone(),
        path_prefix: None,
    };
    assert_eq!(changes_req.cursor, "initial");

    // 3. Process changes
    let changes_resp = ChangesSinceResponse {
        changes: vec![FileChange {
            path: "/new-file.txt".to_string(),
            change_type: ChangeType::Created,
            etag: Some("\"new-etag\"".to_string()),
            size: Some(256),
            modified: Utc::now(),
        }],
        new_cursor: "cursor-after-sync".to_string(),
        has_more: false,
    };

    // 4. Upload files
    let _upload_batch = SyncUploadBatch {
        files: vec![SyncFileUpload {
            path: "/local-file.txt".to_string(),
            content_type: "text/plain".to_string(),
            size: 128,
            etag: None,
        }],
    };

    // 5. Process upload results
    let upload_resp = SyncUploadResponse {
        results: vec![SyncUploadResult {
            path: "/local-file.txt".to_string(),
            status: SyncUploadStatus::Created,
            etag: "\"uploaded-etag\"".to_string(),
        }],
    };

    // Verify all steps completed
    assert!(!changes_resp.changes.is_empty());
    assert!(!upload_resp.results.is_empty());
    assert!(matches!(
        upload_resp.results[0].status,
        SyncUploadStatus::Created
    ));
}

#[test]
fn test_notification_payload_roundtrip() {
    let payloads = vec![
        NotificationPayload {
            event_type: NotificationEvent::FileShared,
            path: Some("/test.txt".to_string()),
            actor: Some("user1".to_string()),
            timestamp: Utc::now(),
        },
        NotificationPayload {
            event_type: NotificationEvent::QuotaWarning { percent_used: 75 },
            path: None,
            actor: None,
            timestamp: Utc::now(),
        },
    ];

    for payload in payloads {
        let json = serde_json::to_string(&payload).unwrap();
        let de: NotificationPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{:?}", payload.event_type), format!("{:?}", de.event_type));
    }
}
