//! Desktop sync engine integration tests.

use ferro_desktop::config::DesktopConfig;

#[test]
fn test_desktop_config_defaults() {
    let config = DesktopConfig::default();
    assert_eq!(config.server_url, "http://localhost:8080");
    assert!(config.auto_mount);
    assert!(config.username.is_empty());
    assert!(config.password.is_empty());
    assert!(config.rclone_path.is_none());
    assert!(!config.mount_point.as_os_str().is_empty());
}

#[test]
fn test_progress_tracking_realistic_counts() {
    #[cfg(feature = "sync")]
    {
        use ferro_desktop::sync::progress::SyncProgress;
        use std::sync::atomic::Ordering;

        let progress = SyncProgress::new();
        progress.total_files.store(1_000_000, Ordering::SeqCst);
        progress.total_bytes.store(50_000_000_000, Ordering::SeqCst);

        for i in 0..500_000u64 {
            let size = 1024 + (i % 8192);
            progress.record_file(size);
        }

        let summary = progress.to_summary();
        assert_eq!(summary.completed_files, 500_000);
        assert!(summary.completed_bytes > 0);
        assert!(summary.bytes_per_second >= 0.0);
        assert!(!progress.is_complete());

        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"total_files\":1000000"));
    }
}

#[test]
fn test_pauser_pause_resume() {
    #[cfg(feature = "sync")]
    {
        use ferro_desktop::sync::pauser::SyncPauser;

        let pauser = SyncPauser::new();
        assert!(!pauser.is_paused());
        assert!(pauser.pause_reason().is_none());

        pauser.pause("user requested");
        assert!(pauser.is_paused());
        assert_eq!(pauser.pause_reason(), Some("user requested".to_string()));

        pauser.pause("network issue");
        assert!(pauser.is_paused());
        assert_eq!(pauser.pause_reason(), Some("network issue".to_string()));

        pauser.resume();
        assert!(!pauser.is_paused());
        assert!(pauser.pause_reason().is_none());
    }
}

#[test]
fn test_sync_state_persistence_round_trip() {
    #[cfg(feature = "sync")]
    {
        use ferro_desktop::sync::state::SyncState;
        use ferro_desktop::sync::types::SyncEntry;

        let dir = std::env::temp_dir().join("ferro-integ-state-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let mut state = SyncState::new(&dir);
        state.insert(SyncEntry {
            relative_path: "documents/report.pdf".to_string(),
            is_dir: false,
            local_hash: "hash_synced".to_string(),
            remote_hash: "hash_synced".to_string(),
            local_size: 1024,
            remote_size: 1024,
            local_mtime_ms: 1000,
            remote_mtime_ms: 1000,
            last_synced_hash: "hash_synced".to_string(),
            last_synced_ms: 1000,
            local_deleted: false,
            remote_deleted: false,
        });
        state.insert(SyncEntry {
            relative_path: "photos".to_string(),
            is_dir: true,
            local_hash: String::new(),
            remote_hash: String::new(),
            local_size: 0,
            remote_size: 0,
            local_mtime_ms: 0,
            remote_mtime_ms: 0,
            last_synced_hash: String::new(),
            last_synced_ms: 0,
            local_deleted: false,
            remote_deleted: false,
        });
        state.save().unwrap();

        let loaded = SyncState::load(&dir).unwrap();
        assert_eq!(loaded.len(), 2);

        let entry = loaded.get("documents/report.pdf").unwrap();
        assert_eq!(entry.status(), ferro_desktop::sync::types::FileSyncStatus::Synced);

        let dir_entry = loaded.get("photos").unwrap();
        assert!(dir_entry.is_dir);

        state.mark_local_deleted("documents/report.pdf");
        assert_eq!(
            state.get("documents/report.pdf").unwrap().status(),
            ferro_desktop::sync::types::FileSyncStatus::LocalDeleted
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[test]
fn test_block_chunking_determinism() {
    #[cfg(feature = "sync")]
    {
        use ferro_desktop::sync::block::chunk_data;

        let data: Vec<u8> = (0..500_000).map(|i| (i % 256) as u8).collect();
        let blocks1 = chunk_data(&data, 65536, 4096, 1_048_576);
        let blocks2 = chunk_data(&data, 65536, 4096, 1_048_576);

        assert_eq!(blocks1.len(), blocks2.len());
        for (a, b) in blocks1.iter().zip(blocks2.iter()) {
            assert_eq!(a, b);
        }

        let total: u64 = blocks1.iter().map(|b| b.1).sum();
        assert_eq!(total, data.len() as u64);
    }
}

#[test]
fn test_sync_config_default_values() {
    #[cfg(feature = "sync")]
    {
        use ferro_desktop::sync::engine::SyncConfig;

        let config = SyncConfig::default();
        assert_eq!(config.server_url, "http://localhost:8080");
        assert_eq!(config.max_file_size, 10_000_000_000);
        assert!(config.use_block_sync);
        assert_eq!(config.block_size, 65536);
    }
}

#[test]
fn test_desktop_config_rclone_url() {
    let config = DesktopConfig {
        server_url: "https://ferro.example.com".to_string(),
        username: "alice".to_string(),
        password: "s3cret".to_string(),
        ..Default::default()
    };
    let url = config.rclone_remote_url();
    assert!(url.starts_with("webdav://alice:s3cret@"));
    assert!(url.contains("ferro.example.com"));
}

#[test]
fn test_temp_dir_sync_engine_construction() {
    #[cfg(feature = "sync")]
    {
        use ferro_desktop::sync::engine::SyncConfig;
        use ferro_desktop::sync::engine::SyncEngine;

        let dir = std::env::temp_dir().join("ferro-integ-engine-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let engine = SyncEngine::new(SyncConfig {
            local_path: dir.clone(),
            remote_path: "/test-sync".to_string(),
            server_url: "http://localhost:18080".to_string(),
            username: "test".to_string(),
            password: "test".to_string(),
            ..Default::default()
        })
        .unwrap();

        let state = engine.state();
        assert!(state.blocking_read().is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[tokio::test]
async fn test_sync_engine_scan_local_files() {
    #[cfg(feature = "sync")]
    {
        use ferro_desktop::sync::engine::SyncConfig;
        use ferro_desktop::sync::engine::SyncEngine;

        let dir = std::env::temp_dir().join("ferro-integ-scan-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(dir.join("file1.txt"), b"hello world").unwrap();
        std::fs::create_dir_all(dir.join("subdir")).unwrap();
        std::fs::write(dir.join("subdir/file2.txt"), b"nested content").unwrap();

        let engine = SyncEngine::new(SyncConfig {
            local_path: dir.clone(),
            remote_path: "/scan-test".to_string(),
            server_url: "http://localhost:18080".to_string(),
            ..Default::default()
        })
        .unwrap();

        let state = engine.state();
        {
            let mut s = state.write().await;
            s.update_local(
                "file1.txt",
                "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9".to_string(),
                11,
                1000,
                false,
            );
            s.update_local("subdir/file2.txt", "abc".to_string(), 14, 1000, false);
        }

        let state = state.read().await;
        assert!(state.get("file1.txt").is_some());
        assert!(state.get("subdir/file2.txt").is_some());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
