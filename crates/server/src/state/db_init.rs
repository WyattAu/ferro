use crate::db::DbHandle;
use crate::state::AppState;
use ferro_selective_sync::persistence::ProfileStore as SelectiveSyncProfileStore;
use std::sync::Arc;

pub(super) fn with_db(mut state: AppState, db: DbHandle) -> AppState {
    state.db = Some(db.clone());
    let conn = db.lock().unwrap_or_else(|e| e.into_inner());

    let user_store = crate::users::InMemoryUserStore::new().with_db(db.clone());
    if let Ok(users) = crate::users::InMemoryUserStore::load_all_from_db(&conn) {
        for user in users {
            user_store.load_from_db(user);
        }
    }
    state.user_store = Arc::new(user_store);

    let share_store = crate::shares::ShareStore::new().with_db(db.clone());
    if let Ok(loaded) = crate::shares::ShareStore::load_all_from_db(&conn) {
        share_store.load_links_blocking(loaded);
    }
    state.share_store = Arc::new(share_store);

    let fav_store = crate::favorites::InMemoryFavoriteStore::new().with_db(db.clone());
    if let Ok(paths) = crate::favorites::InMemoryFavoriteStore::load_all_from_db(&conn) {
        for path in paths {
            fav_store.favorites.insert(path);
        }
    }
    state.favorites = Arc::new(fav_store);

    let tags_store = ferro_server_collaboration::tags::TagStore::new().with_db(db.clone());
    if let Err(e) = tags_store.load_all_from_db(&conn) {
        tracing::warn!(error = %e, "failed to load tags from database");
    }
    state.tags = Arc::new(tags_store);

    let comments_store = crate::comments::CommentStore::new().with_db(db.clone());
    state.comments = Arc::new(comments_store);

    let sync_store = crate::sync::ops::SyncStore::new().with_db(db.clone());
    if let Err(e) = sync_store.load_all_from_db(&conn) {
        tracing::warn!(error = %e, "failed to load sync ops from database");
    }
    state.sync_store = Arc::new(sync_store);
    let activity_store = ferro_server_activitypub::store::ActivityStore::new().with_db(db.clone());
    if let Err(e) = activity_store.load_all_from_db(&conn) {
        tracing::warn!(error = %e, "failed to load activity store from database");
    }
    state.activity_store = Arc::new(activity_store);

    if let Ok(entries) = ferro_server_webdav_core::trash::load_trash_from_db(&conn) {
        for entry in entries {
            state.trash.insert(entry.original_path.clone(), entry);
        }
    }
    state.trash_store = state.trash_store.clone().with_db(db.clone());
    // NOTE: load_from_db() internally re-locks the same Mutex as `conn` above.
    // We must drop `conn` first to avoid a deadlock (std::sync::Mutex is not reentrant).
    drop(conn);
    state.trash_store.load_from_db();
    let push_store = ferro_server_integrations::push_notifications::PushNotificationStore::new(db.clone());
    if let Err(e) = push_store.init_table() {
        tracing::warn!(error = %e, "failed to init push_notifications table");
    }
    state.push_notification_store = Some(Arc::new(tokio::sync::RwLock::new(push_store)));

    // Initialize device store
    let device_store = ferro_server_user_mgmt::account_api::DeviceStore::new(db.clone());
    if let Err(e) = device_store.init_table() {
        tracing::warn!(error = %e, "failed to init devices table");
    }

    // Re-acquire conn lock for remaining loads
    let conn = db.lock().unwrap_or_else(|e| e.into_inner());
    let lock_mgr = ferro_server_webdav_core::lock::LockManager::new().with_db(db.clone());
    if let Err(e) = lock_mgr.load_all_from_db(&conn) {
        tracing::warn!(error = %e, "failed to load locks from database");
    }
    state.lock_manager = Arc::new(lock_mgr);
    let remote_mounts = ferro_server_integrations::remote_mount::RemoteMountStore::new().with_db_handle(db.clone());
    if let Err(e) = remote_mounts.load_all_from_db(&conn) {
        tracing::warn!(error = %e, "failed to load remote mounts from database");
    }
    state.remote_mounts = Arc::new(remote_mounts);
    drop(conn);

    state.branding_store = ferro_server_admin_api::branding::BrandingStore::new().with_db(db.clone());
    state.task_store = ferro_server_productivity::tasks::TaskStore::new().with_db(db.clone());
    state.retention_store = ferro_server_compliance::retention::RetentionStore::new().with_db(db.clone());
    state.dlp_store = ferro_server_compliance::dlp_api::DlpStore::new().with_db(db.clone());
    state.watermark_db_store = ferro_server_content::watermark_api::WatermarkDbStore::new().with_db(db.clone());
    state.guest_store = ferro_server_user_mgmt::guests::GuestStore::new().with_db(db.clone());
    state.gdpr_store = ferro_server_admin_api::gdpr::GdprStore::new().with_db(db.clone());
    state.worm_store = ferro_server_compliance::worm::WormPolicyStore::new().with_db(db.clone());
    state.mail_store = ferro_server_integrations::mail_api::MailStore::new().with_db(db.clone());
    state.notification_prefs_store = crate::notification_prefs_api::NotificationPrefsStore::new().with_db(db.clone());
    // Rebuild admin-api adapters with DB-backed stores
    state.admin_audit_adapter = Arc::new(super::adapters::AdminAuditLogAdapter(state.audit_log.clone()));
    state.admin_share_store = Arc::new(super::adapters::AdminShareStoreAdapter(state.share_store.clone()));
    state.admin_favorites_store = Arc::new(super::adapters::AdminFavoriteStoreAdapter(state.favorites.clone()));
    state.admin_tags_store = Arc::new(super::adapters::AdminTagStoreAdapter(state.tags.clone()));
    state.collab_audit_adapter = Arc::new(super::adapters::CollaborationAuditLogAdapter(state.audit_log.clone()));
    state.user_mgmt_audit_adapter = Arc::new(super::adapters::UserMgmtAuditLogAdapter(state.audit_log.clone()));
    if let Err(e) = state.notification_prefs_store.init_table() {
        tracing::warn!(error = %e, "failed to init notification_prefs table");
    }
    state.webhook_delivery_store = ferro_server_api_core::webhooks::WebhookDeliveryStore::new().with_db(db.clone());

    let selective_sync_path = match &state.data_dir {
        Some(dir) => format!("{}/selective_sync.db", dir),
        None => ":memory:".to_string(),
    };
    let selective_sync_conn = match rusqlite::Connection::open(&selective_sync_path) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "failed to open selective sync database");
            return state;
        }
    };
    match SelectiveSyncProfileStore::new(selective_sync_conn) {
        Ok(store) => {
            state.selective_sync_store = Some(Arc::new(store));
        }
        Err(e) => {
            tracing::warn!(error = %e, "failed to initialize selective sync store");
        }
    }

    state
}
