use crate::AppState;

use crate::account_api;
use crate::activity;
use crate::admin_api;
use crate::antivirus_api;
use crate::api;
use crate::api_error;
use crate::api_federation;
use crate::api_keys_routes;
use crate::audit;
use crate::auth;
use crate::backup;
use crate::batch;
use crate::branding;
use crate::bulk;
use crate::calendar_api;
use crate::chat_api;
use crate::collab_ws;
use crate::comments;
use crate::config;
use crate::contacts_api;
use crate::dashboard;
use crate::dav;
use crate::dlp_api;
use crate::e2ee;
use crate::encryption;
use crate::event_triggers;
use crate::favorites;
use crate::federation;
use crate::gdpr;
use crate::guests;
use crate::link_analytics_api;
use crate::mail_api;
use crate::metrics;
use crate::move_copy;
use crate::notes_api;
use crate::notification_prefs_api;
use crate::offline_api;
use crate::openapi;
use crate::photos_api;
use crate::plugin_marketplace_api;
use crate::plugin_permissions;
use crate::policies;
use crate::presigned;
use crate::prometheus_metrics;
use crate::push_notifications;
use crate::quota;
use crate::remote_mount;
use crate::request_id;
use crate::request_logging;
use crate::retention;
use crate::search;
use crate::security;
use crate::security_headers;
use crate::selective_sync_api;
use crate::shares;
use crate::shares_ext;
use crate::simple_auth;
use crate::snapshots;
use crate::storage_health;
use crate::streaming;
use crate::sync;
use crate::tags;
use crate::tasks_api;
use crate::tenant_rate_limit_api;
use crate::thumbnails;
use crate::totp_api;
use crate::trash;
use crate::upload;
use crate::user_api;
use crate::wasm_upload;
use crate::watermark_api;
#[cfg(feature = "webauthn")]
use crate::webauthn_api;
use crate::webdav;
use crate::webhooks;
use crate::whiteboard_api;
use crate::workers;
use crate::worm;
use crate::ws;

use crate::{
    audit_handler, health_check, health_endpoint, liveness, readiness, startup, storage_stats,
};

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use tower::limit::ConcurrencyLimitLayer;
use tower_http::compression::CompressionLayer;

pub fn build_router(state: AppState) -> Router {
    build_router_with_static(state, None, "*", "v1")
}

fn api_routes(
    state: &AppState,
    webrtc_offers: Arc<ferro_server_webrtc::offers::OfferStore>,
) -> Router<AppState> {
    Router::new()
        .route("/auth/info", axum::routing::get(api::auth_info))
        .route("/auth/login", axum::routing::get(api::auth_login))
        .route("/auth/callback", axum::routing::get(api::auth_callback))
        .route(
            "/auth/refresh",
            axum::routing::post(api::auth_refresh_token),
        )
        .route(
            "/auth/change-password",
            axum::routing::post(api::auth_change_password),
        )
        // TOTP two-factor authentication
        .route(
            "/auth/totp/setup",
            axum::routing::post(totp_api::totp_setup::<AppState>),
        )
        .route(
            "/auth/totp/enable",
            axum::routing::post(totp_api::totp_enable::<AppState>),
        )
        .route(
            "/auth/totp/disable",
            axum::routing::post(totp_api::totp_disable::<AppState>),
        )
        .route(
            "/auth/totp/status",
            axum::routing::get(totp_api::totp_status::<AppState>),
        )
        // WebAuthn/FIDO2 authentication (G-04)
        .merge({
            #[cfg(feature = "webauthn")]
            {
                axum::Router::new()
                    .route(
                        "/auth/webauthn/register/begin",
                        axum::routing::post(webauthn_api::webauthn_register_begin::<AppState>),
                    )
                    .route(
                        "/auth/webauthn/register/finish",
                        axum::routing::post(webauthn_api::webauthn_register_finish::<AppState>),
                    )
                    .route(
                        "/auth/webauthn/login/begin",
                        axum::routing::post(webauthn_api::webauthn_login_begin::<AppState>),
                    )
                    .route(
                        "/auth/webauthn/login/finish",
                        axum::routing::post(webauthn_api::webauthn_login_finish::<AppState>),
                    )
            }
            #[cfg(not(feature = "webauthn"))]
            {
                axum::Router::new()
            }
        })
        .route(
            "/search",
            axum::routing::get(search::handle_search::<AppState>),
        )
        .route(
            "/workers",
            axum::routing::get(workers::list_workers::<AppState>)
                .post(workers::register_worker::<AppState>),
        )
        .route(
            "/workers/upload",
            axum::routing::post(wasm_upload::upload_wasm_module::<AppState>),
        )
        .route(
            "/workers/modules/:filename",
            axum::routing::delete(wasm_upload::delete_wasm_module::<AppState>),
        )
        .route(
            "/workers/modules",
            axum::routing::get(wasm_upload::list_wasm_modules::<AppState>),
        )
        .route(
            "/plugins",
            axum::routing::get(plugin_permissions::list_plugins::<AppState>),
        )
        .route(
            "/admin/plugins/marketplace",
            axum::routing::get(plugin_marketplace_api::list_marketplace_plugins::<AppState>),
        )
        .route(
            "/admin/plugins/:id/install",
            axum::routing::post(plugin_marketplace_api::install_plugin::<AppState>),
        )
        .route(
            "/admin/plugins/:id/uninstall",
            axum::routing::post(plugin_marketplace_api::uninstall_plugin::<AppState>),
        )
        .route(
            "/admin/plugins/:id/enable",
            axum::routing::post(plugin_marketplace_api::enable_plugin::<AppState>),
        )
        .route(
            "/admin/plugins/:id/disable",
            axum::routing::post(plugin_marketplace_api::disable_plugin::<AppState>),
        )
        .route(
            "/policies",
            axum::routing::get(policies::list_policies)
                .post(policies::add_policy)
                .delete(policies::delete_policy),
        )
        .route("/config", axum::routing::get(config::get_server_config))
        .route(
            "/branding",
            axum::routing::get(branding::get_public_branding::<AppState>),
        )
        .route("/files", axum::routing::get(api::list_files))
        .route("/files/mkdir", axum::routing::post(api::mkdir))
        .route(
            "/files/move",
            axum::routing::post(move_copy::move_file::<AppState>),
        )
        .route(
            "/files/copy",
            axum::routing::post(move_copy::copy_file::<AppState>),
        )
        .route("/upload-url", axum::routing::get(presigned::get_upload_url))
        .route(
            "/download-url",
            axum::routing::get(presigned::get_download_url),
        )
        .route(
            "/shares",
            axum::routing::get(shares::list_shares).post(shares::create_share),
        )
        .route(
            "/shares/:token",
            axum::routing::delete(shares::delete_share),
        )
        .route("/audit", axum::routing::get(audit_handler))
        .route("/storage/stats", axum::routing::get(storage_stats))
        .route(
            "/snapshots",
            axum::routing::get(snapshots::list_snapshots::<AppState>)
                .post(snapshots::create_snapshot::<AppState>),
        )
        .route(
            "/snapshots/:id",
            axum::routing::delete(snapshots::delete_snapshot_by_id::<AppState>),
        )
        .route(
            "/snapshots/:id/restore",
            axum::routing::post(snapshots::restore_snapshot::<AppState>),
        )
        .route(
            "/favorites",
            axum::routing::get(favorites::list_favorites)
                .put(favorites::add_favorite)
                .delete(favorites::remove_favorite),
        )
        .route("/recent", axum::routing::get(favorites::list_recent))
        .route("/trash", axum::routing::get(trash::list_trash::<AppState>))
        .route(
            "/trash/:path",
            axum::routing::delete(trash::move_to_trash::<AppState>),
        )
        .route(
            "/trash/restore",
            axum::routing::post(trash::restore_trash::<AppState>),
        )
        .route(
            "/trash/purge",
            axum::routing::delete(trash::purge_trash::<AppState>),
        )
        .route(
            "/trash/empty",
            axum::routing::delete(trash::empty_trash::<AppState>),
        )
        .route("/bulk/delete", axum::routing::post(bulk::bulk_delete))
        .route("/batch/copy", axum::routing::post(batch::batch_copy))
        .route("/batch/move", axum::routing::post(batch::batch_move))
        .route("/batch/delete", axum::routing::post(batch::batch_delete))
        .route("/batch/share", axum::routing::post(batch::batch_share))
        .route(
            "/fed/share",
            axum::routing::post(federation::federated_share),
        )
        .merge(api_federation::routes::<AppState>())
        .route(
            "/files/encrypt",
            axum::routing::post(encryption::encrypt_file::<AppState>),
        )
        .route(
            "/files/decrypt",
            axum::routing::post(encryption::decrypt_file::<AppState>),
        )
        .route("/e2ee/encrypt", axum::routing::post(e2ee::e2ee_encrypt))
        .route(
            "/e2ee/key/generate",
            axum::routing::post(e2ee::e2ee_key_generate),
        )
        .route("/quota", axum::routing::get(quota::get_quota))
        .route("/dashboard", axum::routing::get(dashboard::get_dashboard))
        .route(
            "/activity",
            axum::routing::get(activity::get_activity::<AppState>),
        )
        .route("/tags", axum::routing::get(tags::list_tags::<AppState>))
        .route(
            "/tags/:path",
            axum::routing::get(tags::get_tags::<AppState>).post(tags::add_tags::<AppState>),
        )
        .route(
            "/tags/:path/:tag",
            axum::routing::delete(tags::remove_tag::<AppState>),
        )
        .route(
            "/tags/search",
            axum::routing::get(tags::search_by_tag::<AppState>),
        )
        .route(
            "/comments",
            axum::routing::get(comments::list_comments_handler::<AppState>)
                .post(comments::create_comment_handler::<AppState>),
        )
        .route(
            "/comments/:id",
            axum::routing::put(comments::update_comment_handler::<AppState>)
                .delete(comments::delete_comment_handler::<AppState>),
        )
        .route(
            "/comments/:id/resolve",
            axum::routing::post(comments::resolve_comment_handler::<AppState>),
        )
        .route(
            "/health/storage",
            axum::routing::get(storage_health::storage_health_handler::<AppState>),
        )
        .route(
            "/thumbnail/*path",
            axum::routing::get(thumbnails::get_thumbnail::<AppState>),
        )
        .route(
            "/preferences",
            axum::routing::get(search::handle_get_preferences::<AppState>)
                .put(search::handle_update_preferences::<AppState>),
        )
        .route(
            "/locks",
            axum::routing::get(search::handle_list_locks::<AppState>),
        )
        .route(
            "/locks/force-unlock",
            axum::routing::post(search::handle_force_unlock::<AppState>),
        )
        .route(
            "/locks/:token",
            axum::routing::delete(search::handle_unlock_by_token::<AppState>),
        )
        .route(
            "/admin/stats",
            axum::routing::get(admin_api::admin_stats::<AppState>),
        )
        .route(
            "/admin/storage",
            axum::routing::get(admin_api::admin_storage::<AppState>),
        )
        .route(
            "/admin/storage/stats",
            axum::routing::get(admin_api::admin_storage_stats::<AppState>),
        )
        .route(
            "/admin/audit",
            axum::routing::get(admin_api::admin_audit::<AppState>),
        )
        .route(
            "/admin/audit/summary",
            axum::routing::get(admin_api::admin_audit_summary::<AppState>),
        )
        .route(
            "/admin/maintenance",
            axum::routing::get(admin_api::admin_maintenance::<AppState>)
                .post(admin_api::admin_maintenance::<AppState>),
        )
        .route(
            "/admin/backup/:id",
            axum::routing::delete(backup::delete_backup::<AppState>),
        )
        .route(
            "/admin/backup",
            axum::routing::post(backup::create_backup::<AppState>),
        )
        .route(
            "/admin/backup/latest",
            axum::routing::get(backup::get_latest_backup::<AppState>),
        )
        .route(
            "/admin/backup/download",
            axum::routing::get(backup::download_backup::<AppState>),
        )
        .route(
            "/admin/backup/restore",
            axum::routing::post(backup::restore_from_archive::<AppState>),
        )
        .route(
            "/admin/backups",
            axum::routing::get(backup::list_backups::<AppState>),
        )
        .route(
            "/admin/integrity",
            axum::routing::get(backup::audit_integrity::<AppState>),
        )
        .route(
            "/admin/audit-chain",
            axum::routing::get(backup::audit_chain_verify::<AppState>),
        )
        .route(
            "/admin/restore",
            axum::routing::post(backup::restore_backup::<AppState>),
        )
        .route(
            "/admin/webhooks/:id",
            axum::routing::delete(webhooks::delete_webhook::<AppState>),
        )
        .route(
            "/admin/webhooks",
            axum::routing::post(webhooks::create_webhook::<AppState>)
                .get(webhooks::list_webhooks::<AppState>),
        )
        .route(
            "/admin/webhooks/:id/deliveries",
            axum::routing::get(webhooks::list_webhook_deliveries::<AppState>),
        )
        .route(
            "/admin/webhooks/deliveries/dead",
            axum::routing::get(webhooks::list_dead_letters::<AppState>),
        )
        .route(
            "/admin/users",
            axum::routing::post(user_api::create_user::<AppState>)
                .get(admin_api::admin_list_users::<AppState>),
        )
        .route(
            "/admin/users/:id",
            axum::routing::get(admin_api::admin_get_user::<AppState>)
                .put(user_api::update_user::<AppState>)
                .delete(admin_api::admin_delete_user::<AppState>),
        )
        .route(
            "/admin/users/:id/reset-password",
            axum::routing::post(user_api::reset_password::<AppState>),
        )
        .route(
            "/admin/users/:id/role",
            axum::routing::put(admin_api::admin_set_user_role::<AppState>),
        )
        // Branding (G-09)
        .route(
            "/admin/branding",
            axum::routing::get(branding::get_branding::<AppState>)
                .put(branding::update_branding::<AppState>)
                .delete(branding::reset_branding::<AppState>),
        )
        // Guest accounts (G-10)
        .route(
            "/admin/guests",
            axum::routing::post(guests::create_guest::<AppState>)
                .get(guests::list_guests::<AppState>),
        )
        .route(
            "/admin/guests/:id",
            axum::routing::delete(guests::revoke_guest::<AppState>),
        )
        // Data retention policies (G-23)
        .route(
            "/admin/retention/policies",
            axum::routing::get(retention::list_policies::<AppState>)
                .post(retention::create_policy::<AppState>),
        )
        .route(
            "/admin/retention/policies/:id",
            axum::routing::delete(retention::delete_policy::<AppState>),
        )
        .route(
            "/admin/retention/execute",
            axum::routing::post(retention::execute_policies::<AppState>),
        )
        // WORM policies
        .route(
            "/admin/worm/policies",
            axum::routing::get(worm::list_policies::<AppState>)
                .post(worm::create_policy::<AppState>),
        )
        .route(
            "/admin/worm/policies/:id",
            axum::routing::delete(worm::delete_policy::<AppState>),
        )
        // GDPR compliance (G-13)
        .route(
            "/admin/gdpr",
            axum::routing::get(gdpr::list_gdpr_requests::<AppState>),
        )
        .route(
            "/admin/users/:id/export",
            axum::routing::post(gdpr::request_data_export::<AppState>)
                .get(admin_api::admin_export_user_data::<AppState>),
        )
        .route(
            "/admin/users/:id/data",
            axum::routing::delete(admin_api::admin_erase_user_data::<AppState>),
        )
        // Event triggers (G-16)
        .route(
            "/admin/triggers",
            axum::routing::post(event_triggers::create_event_trigger)
                .get(event_triggers::list_event_triggers),
        )
        .route(
            "/admin/triggers/:id",
            axum::routing::delete(event_triggers::delete_event_trigger),
        )
        .route(
            "/admin/triggers/:id/toggle",
            axum::routing::post(event_triggers::toggle_event_trigger),
        )
        // Tenant rate limiting (OP-006)
        .route(
            "/admin/tenants/rate-limits",
            axum::routing::get(tenant_rate_limit_api::list_tenant_rate_limits),
        )
        .route(
            "/admin/tenants/:id/rate-limit",
            axum::routing::get(tenant_rate_limit_api::get_tenant_rate_limit)
                .put(tenant_rate_limit_api::update_tenant_rate_limit)
                .delete(tenant_rate_limit_api::delete_tenant_rate_limit),
        )
        .route(
            "/admin/tenants/:id/rate-limit/status",
            axum::routing::get(tenant_rate_limit_api::get_tenant_rate_limit_status),
        )
        // Account transfer and device management (P3-08)
        .route(
            "/admin/users/:id/transfer",
            axum::routing::post(account_api::transfer_user_data::<AppState>),
        )
        .route(
            "/admin/devices/:user_id/wipe",
            axum::routing::post(account_api::wipe_user_devices::<AppState>),
        )
        .route(
            "/admin/users/:id/devices",
            axum::routing::get(account_api::list_user_devices::<AppState>),
        )
        .route(
            "/admin/users/:id/devices/:device_id/revoke",
            axum::routing::post(account_api::revoke_device::<AppState>),
        )
        // Notification preferences (P4-07)
        .route(
            "/notification-prefs",
            axum::routing::get(notification_prefs_api::get_notification_prefs)
                .put(notification_prefs_api::update_notification_prefs),
        )
        .route(
            "/admin/search/config",
            axum::routing::get(search::handle_get_search_config::<AppState>)
                .put(search::handle_update_search_config::<AppState>),
        )
        .route(
            "/admin/search/reindex",
            axum::routing::post(search::handle_reindex::<AppState>),
        )
        // Extended shares (G-24, G-25)
        .route(
            "/shares/ext",
            axum::routing::post(shares_ext::create_share_ext),
        )
        .route(
            "/users/me",
            axum::routing::get(user_api::get_current_user::<AppState>)
                .put(user_api::update_current_user::<AppState>),
        )
        .nest(
            "",
            ferro_server_versioning::routes().layer(axum::Extension(
                ferro_server_versioning::VersioningState {
                    data_dir: state.data_dir.clone(),
                    admin_user: state.admin_user.clone(),
                    storage: state.storage.clone(),
                    max_file_versions: state.max_file_versions,
                },
            )),
        )
        .nest(
            "/webrtc",
            ferro_server_webrtc::routes(ferro_server_webrtc::WebRtcState {
                offers: webrtc_offers,
            }),
        )
        .route(
            "/graphql",
            axum::routing::get(ferro_graphql::graphql_playground)
                .post(ferro_graphql::graphql_handler),
        )
        .route(
            "/sync/events",
            axum::routing::get(sync::events::sync_events),
        )
        .route("/sync/delta", axum::routing::get(sync::events::sync_delta))
        .route(
            "/sync/status",
            axum::routing::get(sync::events::sync_status),
        )
        // Block sync protocol
        .route(
            "/sync/blocks/manifest",
            axum::routing::get(sync::blocks::get_manifest),
        )
        .route(
            "/sync/blocks/upload",
            axum::routing::post(sync::blocks::upload_blocks),
        )
        .route(
            "/sync/blocks/check",
            axum::routing::get(sync::blocks::check_blocks),
        )
        .route(
            "/sync/blocks/assemble",
            axum::routing::post(sync::blocks::assemble_file),
        )
        .route(
            "/sync/blocks/:hash",
            axum::routing::get(sync::blocks::get_block),
        )
        // Selective sync profiles
        .route(
            "/sync/profiles",
            axum::routing::get(selective_sync_api::list_profiles)
                .post(selective_sync_api::create_profile),
        )
        .route(
            "/sync/profiles/:id",
            axum::routing::put(selective_sync_api::update_profile)
                .delete(selective_sync_api::delete_profile),
        )
        .route(
            "/sync/filter-preview",
            axum::routing::post(selective_sync_api::filter_preview),
        )
        .route("/ws", axum::routing::get(ws::ws_handler))
        .route("/upload/init", axum::routing::post(upload::init_upload))
        .route(
            "/upload/:upload_id/chunk/:chunk_index",
            axum::routing::put(upload::upload_chunk),
        )
        .route(
            "/upload/:upload_id/complete",
            axum::routing::post(upload::complete_upload),
        )
        .route(
            "/upload/:upload_id",
            axum::routing::delete(upload::cancel_upload),
        )
        .route("/uploads", axum::routing::get(upload::list_uploads))
        // API key management
        .route(
            "/api-keys",
            axum::routing::get(api_keys_routes::list_api_keys::<AppState>)
                .post(api_keys_routes::create_api_key::<AppState>),
        )
        .route(
            "/api-keys/:id",
            axum::routing::delete(api_keys_routes::delete_api_key::<AppState>),
        )
        // Calendar REST API bridge
        .route(
            "/calendar/events",
            axum::routing::get(calendar_api::list_events::<AppState>),
        )
        .route(
            "/calendar/events",
            axum::routing::post(calendar_api::create_event::<AppState>),
        )
        .route(
            "/calendar/events/:uid",
            axum::routing::put(calendar_api::update_event::<AppState>),
        )
        .route(
            "/calendar/events/:uid",
            axum::routing::delete(calendar_api::delete_event::<AppState>),
        )
        // Contacts REST API bridge
        .route(
            "/contacts",
            axum::routing::get(contacts_api::list_contacts::<AppState>),
        )
        .route(
            "/contacts",
            axum::routing::post(contacts_api::create_contact::<AppState>),
        )
        .route(
            "/contacts/:uid",
            axum::routing::put(contacts_api::update_contact::<AppState>),
        )
        .route(
            "/contacts/:uid",
            axum::routing::delete(contacts_api::delete_contact::<AppState>),
        )
        .route(
            "/contacts/export",
            axum::routing::get(contacts_api::export_contacts::<AppState>),
        )
        .route(
            "/contacts/import",
            axum::routing::post(contacts_api::import_contacts::<AppState>),
        )
        // Chat REST API
        .route(
            "/chat/rooms",
            axum::routing::get(chat_api::list_rooms::<AppState>)
                .post(chat_api::create_room::<AppState>),
        )
        .route(
            "/chat/rooms/:room_id/messages",
            axum::routing::get(chat_api::get_messages::<AppState>)
                .post(chat_api::send_message::<AppState>),
        )
        // Photos REST API
        .route("/photos", axum::routing::get(photos_api::list_photos))
        .route(
            "/photos/albums",
            axum::routing::get(photos_api::list_albums).post(photos_api::create_album),
        )
        .route(
            "/photos/thumbnail/:path",
            axum::routing::get(photos_api::get_thumbnail),
        )
        .route(
            "/photos/exif/:path",
            axum::routing::get(photos_api::get_exif),
        )
        // Notes REST API
        .route(
            "/notes",
            axum::routing::get(notes_api::list_notes::<AppState>)
                .post(notes_api::create_note::<AppState>),
        )
        .route(
            "/notes/search",
            axum::routing::get(notes_api::search_notes::<AppState>),
        )
        .route(
            "/notes/:id",
            axum::routing::get(notes_api::get_note::<AppState>)
                .put(notes_api::update_note::<AppState>)
                .delete(notes_api::delete_note::<AppState>),
        )
        // Tasks REST API
        .route(
            "/tasks",
            axum::routing::get(tasks_api::list_tasks::<AppState>)
                .post(tasks_api::create_task::<AppState>),
        )
        .route(
            "/tasks/:id",
            axum::routing::get(tasks_api::update_task::<AppState>)
                .put(tasks_api::update_task::<AppState>)
                .delete(tasks_api::delete_task::<AppState>),
        )
        .route(
            "/tasks/:id/status",
            axum::routing::patch(tasks_api::move_task::<AppState>),
        )
        // Push notification endpoints
        .route(
            "/push/register",
            axum::routing::post(push_notifications::register_push_token::<AppState>),
        )
        .route(
            "/push/unregister",
            axum::routing::post(push_notifications::unregister_push_token::<AppState>),
        )
        .route(
            "/push/tokens",
            axum::routing::get(push_notifications::list_push_tokens::<AppState>),
        )
        // Video streaming endpoints
        .route("/stream", axum::routing::get(streaming::stream_video))
        // Whiteboard endpoints
        .route(
            "/whiteboard",
            axum::routing::get(whiteboard_api::list_whiteboards::<AppState>)
                .post(whiteboard_api::create_whiteboard::<AppState>),
        )
        // Offline mode endpoints
        .route(
            "/offline/sync",
            axum::routing::post(offline_api::trigger_sync::<AppState>),
        )
        .route(
            "/offline/status",
            axum::routing::get(offline_api::get_status::<AppState>),
        )
        .route(
            "/offline/pending",
            axum::routing::get(offline_api::list_pending::<AppState>),
        )
        .route(
            "/offline/resolve/:id",
            axum::routing::post(offline_api::resolve_conflict::<AppState>),
        )
        .route(
            "/offline/cached",
            axum::routing::get(offline_api::list_cached::<AppState>),
        )
        // Antivirus endpoints
        .route(
            "/antivirus/scan/:path",
            axum::routing::post(antivirus_api::scan_file::<AppState>),
        )
        .route(
            "/antivirus/status",
            axum::routing::get(antivirus_api::antivirus_status::<AppState>),
        )
        .route(
            "/antivirus/scan-all",
            axum::routing::post(antivirus_api::scan_all::<AppState>),
        )
        .route(
            "/antivirus/history",
            axum::routing::get(antivirus_api::scan_history::<AppState>),
        )
        // DLP endpoints
        .route(
            "/dlp/policies",
            axum::routing::get(dlp_api::list_policies::<AppState>)
                .post(dlp_api::create_policy::<AppState>),
        )
        .route(
            "/dlp/policies/:id",
            axum::routing::put(dlp_api::update_policy::<AppState>)
                .delete(dlp_api::delete_policy::<AppState>),
        )
        .route(
            "/dlp/scan/:path",
            axum::routing::post(dlp_api::scan_file_dlp::<AppState>),
        )
        .route(
            "/dlp/alerts",
            axum::routing::get(dlp_api::list_alerts::<AppState>),
        )
        .route(
            "/whiteboard/:id",
            axum::routing::get(whiteboard_api::get_whiteboard::<AppState>)
                .put(whiteboard_api::save_whiteboard::<AppState>),
        )
        .route(
            "/whiteboard/:id/image",
            axum::routing::get(whiteboard_api::export_whiteboard_image::<AppState>),
        )
        // Mail API (P3-03)
        .route(
            "/mail/accounts",
            axum::routing::get(mail_api::list_accounts::<AppState>)
                .post(mail_api::create_account::<AppState>),
        )
        .route(
            "/mail/accounts/:id",
            axum::routing::delete(mail_api::delete_account::<AppState>),
        )
        .route(
            "/mail/accounts/:id/folders",
            axum::routing::get(mail_api::mail_folders::<AppState>),
        )
        .route(
            "/mail/accounts/:id/folders/:folder/messages",
            axum::routing::get(mail_api::mail_messages::<AppState>),
        )
        .route(
            "/mail/accounts/:id/folders/:folder/messages/:uid",
            axum::routing::get(mail_api::mail_message_detail::<AppState>),
        )
        .route(
            "/mail/accounts/:id/send",
            axum::routing::post(mail_api::send_email::<AppState>),
        )
        .route(
            "/mail/accounts/:id/folders/:folder/messages/:uid/attachments/:part/download",
            axum::routing::post(mail_api::download_attachment::<AppState>),
        )
        // Link Analytics API (P3-06)
        .route(
            "/analytics/overview",
            axum::routing::get(link_analytics_api::analytics_overview),
        )
        .route(
            "/analytics/links",
            axum::routing::get(link_analytics_api::list_link_analytics),
        )
        .route(
            "/analytics/links/:id/stats",
            axum::routing::get(link_analytics_api::analytics_link_stats),
        )
        // Watermark API (P3-07)
        .route(
            "/watermark/preview",
            axum::routing::post(watermark_api::preview_watermark::<AppState>),
        )
        .route(
            "/watermark/apply/:path",
            axum::routing::post(watermark_api::apply_watermark::<AppState>),
        )
        .route(
            "/watermark/policies",
            axum::routing::get(watermark_api::list_policies::<AppState>)
                .post(watermark_api::create_policy::<AppState>),
        )
        .merge(Router::from(openapi::swagger_ui()))
}

pub fn build_router_with_static(
    state: AppState,
    static_dir: Option<&str>,
    cors_allowed_origins: &str,
    api_version: &str,
) -> Router {
    let request_counter = state.request_count.clone();
    let duration_buckets = state.request_duration_buckets.clone();
    let duration_sum_ms = state.request_duration_sum_ms.clone();
    let status_counts = state.request_status_counts.clone();
    let storage_op_counts = state.storage_op_counts.clone();
    let auth_enabled = state.auth_enabled();
    let oidc = state.oidc.clone();
    let cedar = state.cedar.clone();
    let auth_layer = axum::middleware::from_fn(move |req, next| {
        let fut: std::pin::Pin<
            Box<dyn std::future::Future<Output = axum::response::Response> + Send>,
        > = if auth_enabled {
            Box::pin(auth::oidc::auth_middleware(oidc.clone(), req, next))
        } else {
            let mut req = req;
            req.extensions_mut()
                .insert(common::auth::Claims::anonymous());
            Box::pin(next.run(req))
        };
        fut
    });

    let cedar_layer = axum::middleware::from_fn(move |req, next| {
        Box::pin(auth::cedar::cedar_middleware(cedar.clone(), req, next))
    });

    let admin_user = state.admin_user.clone();
    let admin_password = state.admin_password.clone();
    let admin_password_for_default_check = admin_password.clone();
    let admin_password_rotated = state.admin_password_rotated.clone();
    let user_store = state.user_store.clone();
    let api_key_store = state.api_key_store.clone();
    let simple_auth_layer =
        axum::middleware::from_fn(move |req: axum::http::Request<Body>, next: Next| {
            simple_auth::simple_auth_middleware_with_api_keys(
                req,
                admin_user.clone(),
                admin_password.clone(),
                user_store.clone(),
                Some(api_key_store.clone()),
                next,
            )
        });

    // Enforce password change when default password is in use.
    // This runs AFTER simple_auth, so we know the request passed authentication.
    let default_password_layer =
        axum::middleware::from_fn(move |req: axum::http::Request<Body>, next: Next| {
            let pw = admin_password_for_default_check.clone();
            let rotated = admin_password_rotated.clone();
            async move {
                if !rotated.load(std::sync::atomic::Ordering::Relaxed)
                    && let Some(ref pw_val) = pw
                    && security::is_default_password(pw_val)
                {
                    let path = req.uri().path();
                    if !security::is_password_change_allowed_path(path) {
                        return Ok::<_, std::convert::Infallible>(
                            security::response_require_password_change(),
                        );
                    }
                }
                Ok(next.run(req).await)
            }
        });

    let maintenance_mode = state.maintenance_mode.clone();
    let maintenance_layer = axum::middleware::from_fn(
        move |req: axum::http::Request<Body>, next: Next| {
            let flag = maintenance_mode.clone();
            async move {
                if flag.load(std::sync::atomic::Ordering::Relaxed) {
                    let method = req.method();
                    let path = req.uri().path();
                    // Allow read operations and the maintenance toggle endpoint.
                    let is_read = matches!(method.as_str(), "GET" | "HEAD" | "OPTIONS");
                    // Allow the admin maintenance toggle even during maintenance.
                    let is_maintenance_toggle = path == "/api/admin/maintenance";
                    if !is_read && !is_maintenance_toggle {
                        return Ok::<_, std::convert::Infallible>(
                            crate::api_error::ApiError::service_unavailable(
                                crate::api_error::ApiError::MAINTENANCE_MODE,
                                "Server is in maintenance mode. Write operations are temporarily disabled.",
                            ),
                        );
                    }
                }
                Ok(next.run(req).await)
            }
        },
    );

    let cors_origins = cors_allowed_origins.to_string();
    if cors_origins == "*" {
        tracing::warn!(
            "SECURITY WARNING: CORS is configured to allow all origins ('*'). \
             This is appropriate for development but should be restricted in production."
        );
    }
    let cors_auth_enabled = state.auth_enabled();
    if cors_origins == "*" && cors_auth_enabled {
        tracing::error!(
            "CORS allowed origins is '*' while auth is enabled -- \
             set a specific origin in production to prevent credential theft"
        );
    }
    let cors_layer = axum::middleware::from_fn(move |req: Request<Body>, next: Next| {
        let allowed = cors_origins.clone();
        async move {
            if req.headers().contains_key("origin") {
                let origin_value = if allowed == "*" {
                    axum::http::HeaderValue::from_static("*")
                } else {
                    let req_origin = req
                        .headers()
                        .get("origin")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("");
                    let origin_str = if allowed.split(',').any(|o| o.trim() == req_origin) {
                        req_origin
                    } else {
                        ""
                    };
                    match axum::http::HeaderValue::from_str(origin_str) {
                        Ok(v) if !origin_str.is_empty() => v,
                        _ => {
                            return (StatusCode::FORBIDDEN, "CORS origin not allowed")
                                .into_response();
                        }
                    }
                };

                if req.method() == axum::http::Method::OPTIONS {
                    let mut headers = axum::http::HeaderMap::new();
                    headers.insert("Access-Control-Allow-Origin", origin_value);
                    headers.insert("Access-Control-Allow-Methods", axum::http::HeaderValue::from_static(
                        "GET, POST, PUT, DELETE, PATCH, OPTIONS, PROPFIND, MKCOL, COPY, MOVE, LOCK, UNLOCK, PROPPATCH"
                    ));
                    headers.insert("Access-Control-Allow-Headers", axum::http::HeaderValue::from_static(
                        "Content-Type, Authorization, Depth, Destination, If, If-Match, If-None-Match, Lock-Token, Overwrite"
                    ));
                    headers.insert(
                        "Access-Control-Max-Age",
                        axum::http::HeaderValue::from_static("86400"),
                    );
                    return (StatusCode::NO_CONTENT, headers, "").into_response();
                }

                let mut response = next.run(req).await;
                response
                    .headers_mut()
                    .insert("Access-Control-Allow-Origin", origin_value);
                response.headers_mut().insert(
                    "Access-Control-Expose-Headers",
                    axum::http::HeaderValue::from_static("ETag, Content-Length, DAV, Lock-Token"),
                );
                response
            } else {
                next.run(req).await
            }
        }
    });

    let rate_limiter = Arc::new(ferro_rate_limiter::TokenBucketLimiter::new(
        state.rate_limit_burst,
        state.rate_limit_refill,
        std::time::Duration::from_secs(1),
    ));
    let rate_limit_layer =
        axum::middleware::from_fn(move |req: axum::http::Request<Body>, next: Next| {
            let limiter = rate_limiter.clone();
            async move {
                let client_ip = req
                    .headers()
                    .get("x-forwarded-for")
                    .and_then(|v: &axum::http::HeaderValue| v.to_str().ok())
                    .and_then(|s: &str| s.split(',').next())
                    .map(|s: &str| s.trim().to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                use ferro_rate_limiter::RateLimiter;
                match limiter.check(&client_ip).await {
                    Ok(result) if result.allowed => next.run(req).await,
                    _ => api_error::ApiError::too_many_requests(
                        api_error::ApiError::RATE_LIMITED,
                        "Rate limit exceeded",
                    ),
                }
            }
        });

    let versioned_api_path = format!("/api/{}", api_version);
    let api_version_for_header = api_version.to_string();
    let deprecation_layer = axum::middleware::from_fn(
        move |req: axum::extract::Request, next: axum::middleware::Next| {
            let ver = api_version_for_header.clone();
            async move {
                let mut response = next.run(req).await;
                response.headers_mut().insert(
                    axum::http::HeaderName::from_static("deprecation"),
                    axum::http::HeaderValue::from_static("true"),
                );
                response.headers_mut().insert(
                    axum::http::HeaderName::from_static("sunset"),
                    axum::http::HeaderValue::from_static("Sat, 01 May 2027 00:00:00 GMT"),
                );
                let link = format!("</api/{}>; rel=\"successor-version\"", ver);
                let header_value = axum::http::HeaderValue::from_str(&link)
                    .unwrap_or_else(|_| axum::http::HeaderValue::from_static("invalid-version"));
                response
                    .headers_mut()
                    .insert(axum::http::header::LINK, header_value);
                response
            }
        },
    );

    let mut router_builder = Router::new();
    // Always register WebDAV catch-all at /
    // Static file serving (when --static-dir is set) handles common extensions,
    // but WebDAV methods (PROPFIND, MKCOL, etc.) go to the WebDAV handler
    router_builder = router_builder.route("/", any(webdav::handle_any::<AppState>));
    let router = router_builder
        .route("/.well-known/ferro", axum::routing::get(health_check))
        .route("/healthz", axum::routing::get(liveness))
        .route("/health", axum::routing::get(health_endpoint))
        .route("/readyz", axum::routing::get(readiness))
        .route("/startupz", axum::routing::get(startup))
        .route(
            "/s/:token",
            axum::routing::get(shares::serve_share).post(shares::handle_share_upload),
        )
        // Remote mount management
        .route(
            "/admin/mounts",
            axum::routing::get(remote_mount::list_mounts::<AppState>)
                .post(remote_mount::create_mount::<AppState>),
        )
        .route(
            "/admin/mounts/:id",
            axum::routing::delete(remote_mount::delete_mount::<AppState>),
        )
        .route(
            "/admin/mounts/:id/test",
            axum::routing::get(remote_mount::test_mount::<AppState>),
        )
        // Extended share endpoints (G-24, G-25)
        .route(
            "/s/:token/upload",
            axum::routing::post(shares_ext::upload_to_share),
        )
        .route(
            "/s/:token/uploads",
            axum::routing::get(shares_ext::list_share_uploads),
        )
        .route(
            "/s/:token/view",
            axum::routing::get(shares_ext::serve_view_share),
        )
        .nest(
            "/wopi",
            ferro_server_wopi::routes::<AppState>().layer(axum::Extension(
                ferro_server_wopi::WopiState {
                    storage: state.storage.clone(),
                    lock_manager: state.lock_manager.clone(),
                    wopi_token_secret: state.wopi_token_secret.clone(),
                    wopi_office_url: state.wopi_office_url.clone(),
                },
            )),
        )
        .nest(
            "/hosting",
            ferro_server_wopi::discovery_route::<AppState>().layer(axum::Extension(
                ferro_server_wopi::WopiState {
                    storage: state.storage.clone(),
                    lock_manager: state.lock_manager.clone(),
                    wopi_token_secret: state.wopi_token_secret.clone(),
                    wopi_office_url: state.wopi_office_url.clone(),
                },
            )),
        )
        .route("/metrics", axum::routing::get(metrics::metrics_handler))
        .route(
            "/metrics/prometheus",
            axum::routing::get(prometheus_metrics::prometheus_metrics_handler),
        )
        .route(
            "/.well-known/webfinger",
            axum::routing::get(federation::webfinger),
        )
        .route(
            "/fed/actor/:username",
            axum::routing::get(federation::get_actor),
        )
        .route(
            "/fed/actor/:username/followers",
            axum::routing::get(federation::list_followers),
        )
        .route(
            "/fed/actor/:username/following",
            axum::routing::get(federation::list_following),
        )
        .route(
            "/fed/inbox",
            axum::routing::post(federation::inbox).get(federation::list_inbox),
        )
        .route("/fed/outbox", axum::routing::get(federation::list_outbox))
        .route("/fed/nodeinfo", axum::routing::get(federation::nodeinfo))
        .nest(
            &versioned_api_path,
            api_routes(&state, state.webrtc_offers.clone()),
        )
        .nest(
            "/api",
            api_routes(&state, state.webrtc_offers.clone()).layer(deprecation_layer),
        )
        .route(
            "/ws/collab/:document_id",
            axum::routing::get(collab_ws::collab_ws_handler),
        )
        .route(
            "/ws/chat/:room_id",
            axum::routing::get(chat_api::ws_chat_handler::<AppState>),
        )
        // CalDAV and CardDAV routes (registered before WebDAV fallback)
        .route("/dav/cal", axum::routing::options(dav::caldav_options))
        .route(
            "/dav/cal/",
            axum::routing::get(dav::caldav_list::<AppState>).put(dav::caldav_create::<AppState>),
        )
        .route(
            "/dav/cal/:calendar",
            axum::routing::any(dav::caldav_calendar_or_event::<AppState>),
        )
        .route(
            "/dav/cal/:calendar/",
            axum::routing::any(dav::caldav_calendar_or_event::<AppState>),
        )
        .route(
            "/dav/cal/:calendar/:uid",
            axum::routing::any(dav::caldav_calendar_or_event::<AppState>),
        )
        .route("/dav/card", axum::routing::options(dav::carddav_options))
        .route(
            "/dav/card/",
            axum::routing::get(dav::carddav_list::<AppState>).put(dav::carddav_create::<AppState>),
        )
        .route(
            "/dav/card/:book",
            axum::routing::any(dav::carddav_book_or_contact::<AppState>),
        )
        .route(
            "/dav/card/:book/",
            axum::routing::any(dav::carddav_book_or_contact::<AppState>),
        )
        .route(
            "/dav/card/:book/:uid",
            axum::routing::any(dav::carddav_book_or_contact::<AppState>),
        )
        // /remote/*path moved to api_and_webdav_fallback to avoid matchit 0.7.3
        // bug where catch-all wildcard routes corrupt named-parameter routes
        // in the same tree (github.com/ibraheemdev/matchit/issues/31).
        // Combined fallback: dispatches REST file content requests vs WebDAV.
        //
        // matchit 0.7.3 does not allow catch-all parameters ({*path}) inside
        // nested routes, and .nest("/api/v1", ...) prevents a top-level
        // /api/v1/files/{*path} from matching (the nested router claims the
        // /api/v1 prefix). Using fallback() ensures we run after all route
        // matching, and we dispatch based on path prefix.
        .fallback(api_and_webdav_fallback)
        .layer(rate_limit_layer)
        .layer({
            let tenant_limiter = state.tenant_rate_limiter.clone();
            axum::middleware::from_fn(move |req: axum::http::Request<Body>, next: Next| {
                let limiter = tenant_limiter.clone();
                async move {
                    // Only apply tenant rate limiting if a tenant limiter is configured.
                    let Some(limiter) = limiter else {
                        return next.run(req).await;
                    };

                    // Extract tenant ID from X-Tenant-ID header.
                    let tenant_id = req
                        .headers()
                        .get("x-tenant-id")
                        .and_then(|v| v.to_str().ok())
                        .map(|s| s.to_string());

                    let Some(tid) = tenant_id else {
                        // No tenant header — pass through (global rate limit already applied).
                        return next.run(req).await;
                    };

                    use ferro_rate_limiter::RateLimiter;
                    match limiter.check(&tid).await {
                        Ok(result) if result.allowed => {
                            let mut response = next.run(req).await;
                            if let Ok(val) =
                                axum::http::HeaderValue::from_str(&result.remaining.to_string())
                            {
                                response.headers_mut().insert("X-RateLimit-Remaining", val);
                            }
                            response
                        }
                        _ => api_error::ApiError::too_many_requests(
                            api_error::ApiError::RATE_LIMITED,
                            "Tenant rate limit exceeded",
                        ),
                    }
                }
            })
        })
        .layer(cedar_layer)
        .layer(auth_layer)
        .layer(simple_auth_layer)
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            guests::guest_expiry_middleware::<AppState>,
        ))
        .layer(default_password_layer)
        .layer(maintenance_layer)
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            security::auth_guard_middleware::<AppState>,
        ))
        .layer(cors_layer)
        .layer(axum::middleware::from_fn(request_id::request_id_middleware))
        .layer(axum::middleware::from_fn(
            move |req: Request<Body>, next: Next| {
                let counter = request_counter.clone();
                let buckets = duration_buckets.clone();
                let statuses = status_counts.clone();
                let storage_ops = storage_op_counts.clone();
                let sum = duration_sum_ms.clone();
                request_logging::request_logging_middleware(
                    counter,
                    buckets,
                    sum,
                    statuses,
                    Some(storage_ops),
                    req,
                    next,
                )
            },
        ))
        .layer(axum::middleware::from_fn(
            security_headers::security_headers_middleware,
        ))
        .layer(axum::middleware::from_fn(
            security_headers::panic_handler_middleware,
        ))
        .layer(CompressionLayer::new())
        .layer(axum::extract::DefaultBodyLimit::max(
            state.max_body_size as usize,
        ))
        // Cap concurrent in-flight requests to prevent the tokio runtime and
        // storage backend from being overwhelmed. Excess connections queue in
        // the kernel listen backlog instead of competing for resources.
        .layer(ConcurrencyLimitLayer::new(state.max_concurrent_requests))
        // Reject requests with both Content-Length and Transfer-Encoding
        // to prevent HTTP request smuggling (CL-TE / TE-CL desync).
        .layer(axum::middleware::from_fn(
            security::smuggling_rejection_middleware,
        ))
        .with_state(state.clone())
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            audit::audit_middleware,
        ));

    let schema = ferro_graphql::build_schema(state.graphql_context());
    let mut router = router.layer(axum::Extension(schema));

    // Helper for SPA middleware MIME type detection.
    fn mime_guess(rel: &str) -> &'static str {
        if rel.ends_with(".js") {
            "application/javascript"
        } else if rel.ends_with(".wasm") {
            "application/wasm"
        } else if rel.ends_with(".css") {
            "text/css; charset=utf-8"
        } else if rel.ends_with(".html") || rel.ends_with(".htm") {
            "text/html; charset=utf-8"
        } else if rel.ends_with(".svg") {
            "image/svg+xml"
        } else if rel.ends_with(".json") {
            "application/json"
        } else if rel.ends_with(".png") {
            "image/png"
        } else if rel.ends_with(".ico") {
            "image/x-icon"
        } else if rel.ends_with(".woff") || rel.ends_with(".woff2") {
            "font/woff2"
        } else {
            "application/octet-stream"
        }
    }

    // Serve SPA static files when --static-dir is set.
    // Uses custom handler instead of ServeDir to fix trailing-slash redirect bug
    // where Leptos Router path="/" fails to match on /ui/.
    if let Some(dir) = static_dir {
        let static_dir_path = std::path::PathBuf::from(dir);
        tracing::info!("Serving static web assets from {:?}", static_dir_path);

        // SPA middleware: intercepts requests for static assets.
        // - / serves index.html (SPA entry point)
        // - /*.html, /*.css, /*.js, /*.wasm serve static files
        // - /ui* paths serve static files with index.html fallback for SPA routing
        // - All other paths fall through to API/WebDAV handlers
        let spa_middleware = axum::middleware::from_fn(
            move |req: axum::http::Request<axum::body::Body>, next: axum::middleware::Next| {
                let path = req.uri().path().to_owned();
                let static_dir_path = static_dir_path.clone();
                async move {
                    // Helper to serve a file from static_dir
                    let serve_file = |rel: &str| {
                        let static_dir_path = static_dir_path.clone();
                        let rel = rel.to_string();
                        async move {
                            let file_path = std::path::Path::new(&static_dir_path).join(&rel);
                            if file_path.is_file() {
                                match tokio::fs::read(&file_path).await {
                                    Ok(content) => {
                                        let ct = mime_guess(&rel);
                                        Some(
                                            (
                                                StatusCode::OK,
                                                [(axum::http::header::CONTENT_TYPE, ct)],
                                                content,
                                            )
                                                .into_response(),
                                        )
                                    }
                                    Err(_) => None,
                                }
                            } else {
                                None
                            }
                        }
                    };

                    // Root path: serve index.html
                    if path == "/" {
                        if let Some(resp) = serve_file("index.html").await {
                            return resp;
                        }
                        return (StatusCode::NOT_FOUND, "Not found").into_response();
                    }

                    // Check for common static file extensions
                    let has_static_ext = path.ends_with(".html")
                        || path.ends_with(".css")
                        || path.ends_with(".js")
                        || path.ends_with(".wasm");

                    if has_static_ext {
                        // Strip leading slash to get relative path
                        let rel = path.trim_start_matches('/');
                        if let Some(resp) = serve_file(rel).await {
                            return resp;
                        }
                        // File not found - fall through to next handler
                    }

                    // Redirect /ui/ to /ui -- Leptos Router path="/" fails to match
                    // when browser URL has trailing slash after base "/ui".
                    if path == "/ui/" {
                        return axum::response::Redirect::permanent("/ui").into_response();
                    }

                    // /ui and /ui/* paths: serve static files with SPA fallback
                    if path == "/ui" || path.starts_with("/ui/") {
                        let rel = path.trim_start_matches("/ui/");
                        if let Some(resp) = serve_file(rel).await {
                            return resp;
                        }
                        // Serve index.html for SPA client-side routing
                        return match tokio::fs::read(static_dir_path.join("index.html")).await {
                            Ok(content) => (
                                StatusCode::OK,
                                [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
                                content,
                            )
                                .into_response(),
                            Err(_) => (StatusCode::NOT_FOUND, "Not found").into_response(),
                        };
                    }

                    // Not a static file path, pass to next handler (API/WebDAV)
                    next.run(req).await
                }
            },
        );
        router = router.layer(spa_middleware);
    }

    router
}

pub(crate) async fn api_and_webdav_fallback(
    method: axum::http::Method,
    uri: axum::http::Uri,
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Body,
) -> Response {
    let path_str = uri.path().to_string();
    // Check for both /api/v1/files/ and /api/files/ (deprecated) prefixes
    let rest = path_str
        .strip_prefix("/api/v1/files/")
        .or_else(|| path_str.strip_prefix("/api/files/"));

    // Check for /fed/files/ prefix (moved from explicit route to fallback to
    // avoid matchit 0.7.3 bug where catch-all wildcard routes in the same tree
    // as named-parameter routes cause the parameterized routes to silently fail).
    let fed_files_rest = path_str.strip_prefix("/fed/files/");

    if let Some(file_path) = rest
        && !file_path.is_empty()
    {
        // ----------------------------------------------------------------
        // Versioning API: intercept before the generic file-content handler.
        //
        // matchit `{path}` only captures a single segment, so nested paths
        // like /api/v1/files/docs/test.txt/versions never match the
        // versioning routes registered via .nest(""). They fall through to
        // this fallback, which previously treated them as file content
        // requests. We check for the /versions and /diff suffixes here.
        // ----------------------------------------------------------------

        // GET|DELETE /files/{*path}/versions/{version_id}
        if let Some(idx) = file_path.rfind("/versions/") {
            let filepath = &file_path[..idx];
            let after = &file_path[idx + "/versions/".len()..];
            if !filepath.is_empty()
                && let Ok(vid) = after.parse::<u64>()
            {
                let ver_state = ferro_server_versioning::VersioningState {
                    data_dir: state.data_dir.clone(),
                    admin_user: state.admin_user.clone(),
                    storage: state.storage.clone(),
                    max_file_versions: state.max_file_versions,
                };
                return match method {
                    axum::http::Method::GET => {
                        ferro_server_versioning::get_version(
                            axum::Extension(ver_state),
                            axum::extract::Path((filepath.to_string(), vid)),
                        )
                        .await
                    }
                    axum::http::Method::DELETE => {
                        ferro_server_versioning::delete_version(
                            axum::Extension(ver_state),
                            axum::extract::Path((filepath.to_string(), vid)),
                        )
                        .await
                    }
                    _ => (
                        axum::http::StatusCode::METHOD_NOT_ALLOWED,
                        "Method not allowed",
                    )
                        .into_response(),
                };
            }
        }

        // GET|POST /files/{*path}/versions
        if let Some(filepath) = file_path.strip_suffix("/versions")
            && !filepath.is_empty()
            && matches!(method, axum::http::Method::GET | axum::http::Method::POST)
        {
            let ver_state = ferro_server_versioning::VersioningState {
                data_dir: state.data_dir.clone(),
                admin_user: state.admin_user.clone(),
                storage: state.storage.clone(),
                max_file_versions: state.max_file_versions,
            };
            return match method {
                axum::http::Method::GET => {
                    ferro_server_versioning::list_versions(
                        axum::Extension(ver_state),
                        axum::extract::Path(filepath.to_string()),
                    )
                    .await
                }
                axum::http::Method::POST => {
                    ferro_server_versioning::create_version(
                        axum::Extension(ver_state),
                        axum::extract::Path(filepath.to_string()),
                    )
                    .await
                }
                _ => StatusCode::METHOD_NOT_ALLOWED.into_response(),
            };
        }

        // GET /files/{*path}/diff
        if let Some(filepath) = file_path.strip_suffix("/diff")
            && !filepath.is_empty()
            && method == axum::http::Method::GET
        {
            let ver_state = ferro_server_versioning::VersioningState {
                data_dir: state.data_dir.clone(),
                admin_user: state.admin_user.clone(),
                storage: state.storage.clone(),
                max_file_versions: state.max_file_versions,
            };
            let params: std::collections::HashMap<String, String> = uri
                .query()
                .map(|q| {
                    q.split('&')
                        .filter_map(|p| {
                            let mut parts = p.splitn(2, '=');
                            Some((parts.next()?.to_string(), parts.next()?.to_string()))
                        })
                        .collect()
                })
                .unwrap_or_default();
            return ferro_server_versioning::diff_versions(
                axum::Extension(ver_state),
                axum::extract::Path(filepath.to_string()),
                axum::extract::Query(ferro_server_versioning::DiffParams {
                    from: params.get("from").cloned().unwrap_or_default(),
                    to: params.get("to").cloned().unwrap_or_default(),
                }),
            )
            .await;
        }

        // File content handler (original behavior)
        let body_bytes = match http_body_util::BodyExt::collect(body).await {
            Ok(collected) => collected.to_bytes(),
            Err(_) => {
                return (
                    axum::http::StatusCode::BAD_REQUEST,
                    "Failed to read request body",
                )
                    .into_response();
            }
        };
        return api::files_content_handler(
            method,
            uri,
            State(state),
            headers,
            Some(axum::extract::Path(file_path.to_string())),
            body_bytes,
        )
        .await;
    }
    // Federation file proxy: /fed/files/{*path}
    // Dispatched here instead of as an explicit route to avoid matchit 0.7.3
    // bug where catch-all wildcard routes corrupt named-parameter routes in the
    // same tree. See: https://github.com/ibraheemdev/matchit/issues/31
    if let Some(file_path) = fed_files_rest
        && !file_path.is_empty()
    {
        return match method {
            axum::http::Method::GET => {
                api_federation::get_fed_file::<AppState>(
                    State(state),
                    axum::extract::Path(file_path.to_string()),
                    headers,
                )
                .await
            }
            axum::http::Method::PUT => {
                api_federation::put_fed_file::<AppState>(
                    State(state),
                    axum::extract::Path(file_path.to_string()),
                    headers,
                    body,
                )
                .await
            }
            axum::http::Method::DELETE => {
                api_federation::delete_fed_file::<AppState>(
                    State(state),
                    axum::extract::Path(file_path.to_string()),
                    headers,
                )
                .await
            }
            _ => StatusCode::METHOD_NOT_ALLOWED.into_response(),
        };
    }
    // Remote mount proxy: /remote/{*path}
    // Dispatched here instead of as an explicit route to avoid matchit 0.7.3 bug.
    if path_str.starts_with("/remote/") {
        return remote_mount::proxy_remote_mount::<AppState>(
            method,
            uri,
            State(state),
            headers,
            body,
        )
        .await;
    }
    // Fall through to WebDAV handler
    webdav::handle_any::<AppState>(method, uri, State(state), None, headers, body).await
}
