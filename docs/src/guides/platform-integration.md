# Platform Integration Contracts

Integration specs for deferred platform items from the v3.3 Client Ecosystem milestone.

---

## CL-001: Tauri Desktop Polish

**Status:** Not started
**Effort:** 10 days
**Priority:** P0
**Tracking:** ROADMAP.md line 1293

### Objective

Complete the Tauri desktop app with native file picker integration, system tray improvements, and auto-update.

### Existing Code

| File | Role |
|------|------|
| `crates/desktop/src/tray.rs` | Tray menu actions and status tooltips |
| `crates/desktop/src/tauri_commands.rs` | Tauri command handlers (mount, config, sync) |
| `crates/desktop/src/commands.rs` | `DesktopState` lifecycle and sync engine |
| `crates/desktop/src/mount.rs` | `MountService` wrapping `RcloneManager` |
| `crates/desktop/src/rclone.rs` | rclone sidecar process management |
| `crates/desktop/src/config.rs` | `DesktopConfig` (server_url, mount_point, auto_mount) |

### API Contracts

#### 1. Native File Picker

New Tauri commands in `tauri_commands.rs`:

```rust
#[tauri::command]
pub async fn cmd_open_file_picker(
    mode: FilePickerMode,        // Open | Save | Folder
    filter: Option<Vec<String>>, // file extension filters
    state: State<'_, DesktopState>,
) -> Result<Option<String>, String>

#[tauri::command]
pub async fn cmd_save_file_as(
    source_path: String,
    default_name: Option<String>,
    state: State<'_, DesktopState>,
) -> Result<Option<String>, String>
```

Implementation uses `tauri-plugin-dialog` for native OS dialogs. The picker is used by the file browser UI when uploading files from local disk or saving remote files locally.

#### 2. System Tray Enhancements

Extend `TrayAction` enum in `tray.rs`:

```rust
pub enum TrayAction {
    Mount,
    Unmount,
    OpenBrowser,
    OpenFolder,
    ShowStatus,
    SyncNow,
    PauseSync,
    ResumeSync,
    CheckForUpdates,  // NEW
    Quit,
}
```

New tray state in `commands.rs`:

```rust
pub struct TrayState {
    pub mount_status: MountStatus,
    pub sync_active: bool,
    pub sync_paused: bool,
    pub pending_updates: bool,
}
```

Tray tooltip format (extending `status_tooltip` + `sync_tooltip_suffix`):
```
Ferro: Connected | Syncing | Update available
```

#### 3. Auto-Update

New module `crates/desktop/src/updater.rs`:

```rust
pub struct UpdateInfo {
    pub version: String,
    pub release_notes: String,
    pub download_url: String,
    pub sha256: String,
    pub size_bytes: u64,
}

pub struct Updater {
    config: DesktopConfig,
    current_version: String,
}

impl Updater {
    pub async fn check_for_updates(&self) -> Result<Option<UpdateInfo>, UpdateError>;
    pub async fn download_update(&self, info: &UpdateInfo) -> Result<PathBuf, UpdateError>;
    pub async fn apply_update(&self, installer_path: &Path) -> Result<(), UpdateError>;
}
```

Uses Tauri's built-in updater plugin (`tauri-plugin-updater`) with a JSON manifest endpoint at `{server_url}/api/desktop/updates.json`.

Update manifest schema:

```json
{
  "version": "3.3.0",
  "notes": "Security fix + FUSE stability",
  "platforms": {
    "darwin-aarch64": { "url": "...", "sha256": "..." },
    "darwin-x86_64": { "url": "...", "sha256": "..." },
    "linux-x86_64": { "url": "...", "sha256": "..." },
    "windows-x86_64": { "url": "...", "sha256": "..." }
  }
}
```

### Implementation Approach

1. **Days 1-3:** File picker — add `tauri-plugin-dialog`, implement `cmd_open_file_picker` and `cmd_save_file_as`, wire into frontend file browser component.
2. **Days 4-6:** Tray polish — extend `TrayAction`, add `TrayState`, update tooltip rendering, add tray context menu with update notification.
3. **Days 7-10:** Auto-update — add `tauri-plugin-updater`, implement `Updater` struct, set up manifest endpoint, add version check on startup + periodic (6h), tray badge for pending updates.

### Testing Strategy

| Test | Type | Coverage |
|------|------|----------|
| `test_file_picker_returns_path` | Unit | Picker returns valid path on selection |
| `test_file_picker_cancel` | Unit | Returns `None` on cancel |
| `test_tray_tooltip_states` | Unit | Tooltip string for each `MountStatus` + sync combo |
| `test_updater_check_manifest` | Unit | Parses update manifest JSON correctly |
| `test_updater_no_update` | Unit | Returns `None` when current version matches |
| `test_updater_sha256_verification` | Unit | Rejects mismatched SHA256 |
| File picker E2E | Manual | Open/save dialog on macOS, Linux, Windows |
| Tray E2E | Manual | All menu items functional, tooltips update live |
| Auto-update E2E | Manual | Full cycle: check → download → install → restart |

---

## CL-002: FUSE Mount Stability

**Status:** Not started
**Effort:** 5 days
**Priority:** P1
**Tracking:** ROADMAP.md line 1294

### Objective

Extend FUSE mount test coverage, handle network interruptions gracefully, add reconnection logic.

### Existing Code

| File | Role |
|------|------|
| `crates/desktop/src/mount.rs` | `MountService` mount/unmount lifecycle |
| `crates/desktop/src/rclone.rs` | rclone process management, `MountStatus` enum |
| `docs/src/guides/fuse-mount.md` | Current FUSE mount user guide |
| `crates/fuse/` (referenced) | `ferro-fuse` binary, WebDAV-to-FUSE translation |

### API Contracts

#### 1. Reconnection Manager

New module `crates/desktop/src/reconnect.rs`:

```rust
pub struct ReconnectPolicy {
    pub initial_backoff_ms: u64,   // default: 1000
    pub max_backoff_ms: u64,       // default: 30000
    pub multiplier: f64,           // default: 2.0
    pub max_attempts: u32,         // default: 0 (unlimited)
    pub jitter: bool,              // default: true
}

pub enum ConnectionState {
    Connected,
    Reconnecting { attempt: u32, next_retry_ms: u64 },
    Disconnected { reason: String },
    Failed { reason: String },
}

pub struct ReconnectManager {
    policy: ReconnectPolicy,
    state: RwLock<ConnectionState>,
    on_state_change: Option<Box<dyn Fn(ConnectionState) + Send + Sync>>,
}

impl ReconnectManager {
    pub fn new(policy: ReconnectPolicy) -> Self;
    pub async fn on_disconnect(&self, reason: &str);
    pub async fn state(&self) -> ConnectionState;
    pub async fn reset(&self);
}
```

#### 2. Mount Health Monitor

Extend `MountService` in `mount.rs`:

```rust
impl MountService {
    /// Spawn a health-check loop that monitors mount liveness.
    pub fn start_health_check(self: &Arc<Self>, interval: Duration) -> JoinHandle<()>;

    /// Check if the mount point is actually accessible (not just "process alive").
    pub async fn check_mount_liveness(&self) -> MountHealth;
}

pub struct MountHealth {
    pub is_alive: bool,
    pub latency_ms: u64,
    pub last_error: Option<String>,
    pub consecutive_failures: u32,
}
```

Health check performs: `stat(mount_point)` → `read(mount_point)/..` → verify response within timeout.

#### 3. Network Interruption Handling

New types in `rclone.rs`:

```rust
pub enum MountEvent {
    Mounted,
    Unmounted,
    NetworkLost,
    NetworkRestored,
    Reconnecting { attempt: u32 },
    Reconnected,
    Error { message: String },
}

pub struct MountEventBus {
    subscribers: Vec<oneshot::Sender<MountEvent>>,
}
```

rclone stderr monitoring (existing in `RcloneManager::mount`) is extended to detect:
- `ERROR: ... connection reset` → emit `NetworkLost`
- `ERROR: ... i/o timeout` → emit `NetworkLost`
- Successful reconnection after error → emit `NetworkRestored`
- Process exit with code != 0 → emit `Error`, trigger `ReconnectManager`

#### 4. Stable Mount Point

Extend `DesktopConfig`:

```rust
pub struct DesktopConfig {
    // ... existing fields ...
    pub reconnect_policy: Option<ReconnectPolicy>,
    pub health_check_interval_secs: u32,  // default: 30, 0 = disabled
    pub auto_remount: bool,               // default: true
}
```

### Implementation Approach

1. **Days 1-2:** Reconnection manager — implement `ReconnectManager` with exponential backoff + jitter, integrate with `RcloneManager::mount` failure path.
2. **Day 3:** Health monitor — add `start_health_check` to `MountService`, integrate mount liveness into tray tooltip status.
3. **Day 4:** Network event bus — extend rclone stderr parser to detect network errors, wire events to reconnect manager and UI notification.
4. **Day 5:** Tests and edge cases — handle rapid disconnect/reconnect cycling (debounce), mount point cleanup on persistent failure.

### Testing Strategy

| Test | Type | Coverage |
|------|------|----------|
| `test_reconnect_backoff` | Unit | Exponential backoff with jitter stays within bounds |
| `test_reconnect_max_attempts` | Unit | Stops after max_attempts when set |
| `test_reconnect_reset` | Unit | Resets attempt counter on successful reconnect |
| `test_mount_health_success` | Unit | Liveness check passes when mount exists |
| `test_mount_health_failure` | Unit | Detects stale mount point |
| `test_network_loss_detection` | Unit | stderr parser emits `NetworkLost` on connection reset |
| `test_debounce_rapid_disconnects` | Unit | Multiple disconnects within window produce single reconnect |
| FUSE stress test | Integration | Kill network for 10s, verify auto-reconnect |
| Mount point persistence | Integration | Process crash → auto-remount within 30s |
| Tray status accuracy | Integration | Tooltip reflects real-time connection state |

---

## CL-003: iOS Files Provider

**Status:** Not started
**Effort:** 15 days
**Priority:** P1
**Tracking:** ROADMAP.md line 1295

### Objective

Implement iOS Files Provider extension using `ferro-mobile-contract` API bindings.

### Existing Code

| File | Role |
|------|------|
| `crates/mobile-contract/src/api.rs` | REST API types: `MobileFile`, `FileListRequest/Response`, `UploadRequest/Response`, etc. |
| `crates/mobile-contract/src/sync.rs` | `SyncCheckpoint`, `ChangesSinceRequest/Response`, `FileChange` |
| `crates/mobile-contract/src/error.rs` | `MobileApiError` enum |
| `crates/mobile-contract/src/notifications.rs` | `PushTokenRegistration`, `NotificationPayload` |
| `crates/desktop/src/mobile.rs` | `IosFilesProvider` stub, `MobileSyncConfig`, `FileProviderCapabilities` |

### API Contracts

#### 1. File Provider Extension Bridge

The iOS File Provider extension runs as a separate process. It communicates with the main app via App Groups (shared container) and the Ferro REST API.

New types in `crates/mobile-contract/src/file_provider.rs`:

```rust
pub struct FileProviderItem {
    pub identifier: String,       // unique ID for FPItem
    pub path: String,             // remote path on Ferro server
    pub filename: String,
    pub content_type: String,
    pub file_size: u64,
    pub last_modified: DateTime<Utc>,
    pub etag: String,
    pub is_directory: bool,
    pub parent_identifier: Option<String>,
}

pub struct FileProviderDomain {
    pub display_name: String,
    pub server_url: String,
    pub auth_token: String,
    pub quota_bytes: u64,
    pub used_bytes: u64,
}

pub enum FileProviderAction {
    GetItem { identifier: String },
    GetItemByPath { path: String },
    EnumerateChildren { parent_identifier: String, page_token: Option<String> },
    FetchContents { identifier: String, range: Option<Range<u64>> },
    CreateItem { item: FileProviderItem },
    ModifyItem { identifier: String, new_data: Option<Vec<u8>>, new_name: Option<String> },
    DeleteItem { identifier: String },
    MoveItem { identifier: String, new_parent: String },
}
```

#### 2. Swift ↔ Rust Bridge (UniFFI)

The Rust types are exported to Swift via UniFFI:

```rust
// crates/mobile-contract/src/lib.rs additions
pub mod file_provider;

#[uniffi::export]
pub trait FileProviderBackend: Send + Sync {
    fn resolve_item(&self, identifier: String) -> Result<FileProviderItem, MobileApiError>;
    fn enumerate(&self, parent: String, page_token: Option<String>)
        -> Result<(Vec<FileProviderItem>, Option<String>), MobileApiError>;
    fn fetch_contents(&self, identifier: String, offset: u64, length: u64)
        -> Result<Vec<u8>, MobileApiError>;
    fn upload(&self, parent: String, name: String, data: Vec<u8>, content_type: String)
        -> Result<FileProviderItem, MobileApiError>;
    fn delete(&self, identifier: String) -> Result<(), MobileApiError>;
    fn move_item(&self, identifier: String, new_parent: String)
        -> Result<FileProviderItem, MobileApiError>;
}
```

#### 3. Sync Strategy

File Provider uses incremental sync via `ChangesSinceRequest` from `mobile-contract/src/sync.rs`:

```rust
pub struct IosFileProviderSync {
    checkpoint: Option<SyncCheckpoint>,
    domain: FileProviderDomain,
    cache_dir: PathBuf,
}

impl IosFileProviderSync {
    /// Pull remote changes and map to FileProviderItem updates.
    pub async fn sync_changes(&self) -> Result<SyncResult, MobileApiError>;

    /// Upload local edits (from File Provider write operations) to server.
    pub async fn push_local_changes(&self) -> Result<SyncResult, MobileApiError>;
}

pub struct SyncResult {
    pub items_created: u32,
    pub items_modified: u32,
    pub items_deleted: u32,
    pub conflicts: Vec<String>,
    pub new_checkpoint: String,
}
```

#### 4. Thumbnail Provider

```rust
pub trait ThumbnailProvider: Send + Sync {
    fn generate_thumbnail(&self, identifier: String, size: ThumbnailSize)
        -> Result<Vec<u8>, MobileApiError>;
    fn supports_format(&self, content_type: &str) -> bool;
}

pub enum ThumbnailSize {
    Small,   // 64x64
    Medium,  // 128x128
    Large,   // 256x256
}
```

Thumbnails are generated server-side via WebDAV `GET` with `Range` header, then downscaled on-device.

### Implementation Approach

1. **Days 1-3:** Define `file_provider.rs` types and UniFFI exports. Build Swift bridge with `FileProviderBackend` trait implementation that calls the REST API via `mobile-contract` types.
2. **Days 4-7:** Implement `IosFileProviderSync` — integrate `ChangesSinceRequest/Response` for incremental sync, build local cache layer using SQLite (CoreData not required; raw SQLite avoids bridging complexity).
3. **Days 8-10:** File Provider extension — implement `NSFileProviderExtension` in Swift, wire `FileProviderBackend` methods to extension lifecycle (`beginObserving`, `stopObserving`, `import`).
4. **Days 11-13:** Thumbnails and polishing — add `ThumbnailProvider`, handle file conflicts (keep both with `.conflicted` suffix), test with iOS Files app.
5. **Days 14-15:** Integration testing and App Store prep.

### Testing Strategy

| Test | Type | Coverage |
|------|------|----------|
| `test_file_provider_item_serialization` | Unit | Roundtrip `FileProviderItem` through UniFFI |
| `test_enumerate_children_pagination` | Unit | Page token handling in `enumerate` |
| `test_sync_changes_incremental` | Unit | Only processes items newer than checkpoint |
| `test_sync_conflict_resolution` | Unit | Server + local edit on same file |
| `test_thumbnail_size_variants` | Unit | Each `ThumbnailSize` produces correct dimensions |
| `test_backend_error_mapping` | Unit | `MobileApiError` → `NSError` codes |
| File Provider in Files app | Manual | Browse, open, edit, delete files |
| Background sync | Manual | App killed → Files app shows fresh data |
| Conflict scenario | Manual | Edit same file on server + device simultaneously |
| Offline behavior | Manual | Airplane mode → graceful error in Files app |

---

## CL-004: Android SAF Provider

**Status:** Not started
**Effort:** 15 days
**Priority:** P1
**Tracking:** ROADMAP.md line 1296

### Objective

Implement Android Storage Access Framework provider using `ferro-mobile-contract` API bindings.

### Existing Code

| File | Role |
|------|------|
| `crates/mobile-contract/src/api.rs` | REST API types: `MobileFile`, `FileListRequest/Response`, `UploadRequest/Response` |
| `crates/mobile-contract/src/sync.rs` | `SyncCheckpoint`, `ChangesSinceRequest/Response`, `FileChange` |
| `crates/mobile-contract/src/error.rs` | `MobileApiError` enum |
| `crates/mobile-contract/src/notifications.rs` | `PushTokenRegistration`, `NotificationPayload` |
| `crates/desktop/src/mobile.rs` | `AndroidSAFProvider` stub, `MobileSyncConfig`, `FileProviderCapabilities` |

### API Contracts

#### 1. SAF DocumentsProvider Bridge

The Android SAF provider extends `DocumentsProvider` and communicates with Rust via JNI.

New types in `crates/mobile-contract/src/saf.rs`:

```rust
pub struct SafDocument {
    pub document_id: String,
    pub path: String,
    pub display_name: String,
    pub mime_type: String,
    pub size: u64,
    pub last_modified: i64, // epoch millis
    pub flags: SafDocumentFlags,
}

bitflags! {
    pub struct SafDocumentFlags: u32 {
        const DIR = 0x1;
        const FILE = 0x2;
        const SUPPORTS_WRITE = 0x4;
        const SUPPORTS_DELETE = 0x8;
        const SUPPORTS_RENAME = 0x10;
        const SUPPORTS_THUMBNAIL = 0x20;
        const SUPPORTS_METADATA = 0x40;
    }
}

pub enum SafAction {
    QueryRoots,
    QueryChildren { parent_id: String },
    OpenDocument { document_id: String, mode: SafOpenMode },
    CreateDocument { parent_id: String, display_name: String, mime_type: String },
    DeleteDocument { document_id: String },
    RenameDocument { document_id: String, new_name: String },
    GetDocumentMetadata { document_id: String },
}

pub enum SafOpenMode {
    Read,
    Write,
    ReadWrite,
    WriteTruncate,
}

pub struct SafRoot {
    pub root_id: String,
    pub title: String,
    pub summary: String,
    pub icon: Option<Vec<u8>>,
    pub flags: u32,
    pub mime_types: Vec<String>,
}
```

#### 2. JNI Bridge

```rust
// crates/mobile-contract/src/lib.rs additions
pub mod saf;

#[uniffi::export]
pub trait SafDocumentBackend: Send + Sync {
    fn query_roots(&self) -> Result<Vec<SafRoot>, MobileApiError>;
    fn query_children(&self, parent_id: String) -> Result<Vec<SafDocument>, MobileApiError>;
    fn open_document(&self, document_id: String, mode: SafOpenMode)
        -> Result<DocumentHandle, MobileApiError>;
    fn create_document(&self, parent_id: String, display_name: String, mime_type: String)
        -> Result<SafDocument, MobileApiError>;
    fn delete_document(&self, document_id: String) -> Result<(), MobileApiError>;
    fn rename_document(&self, document_id: String, new_name: String)
        -> Result<SafDocument, MobileApiError>;
    fn get_metadata(&self, document_id: String) -> Result<SafDocument, MobileApiError>;
}

pub struct DocumentHandle {
    pub fd: i32,                    // file descriptor for pipe
    pub size: u64,
    pub content_type: String,
}
```

#### 3. Content Provider URI Mapping

```rust
pub struct SafUriMapper {
    root_path: String,
    server_url: String,
}

impl SafUriMapper {
    /// Convert a `content://com.ferro.documents/root/...` URI to a remote path.
    pub fn uri_to_remote_path(&self, document_id: &str) -> Result<String, MobileApiError>;

    /// Convert a remote path to a document ID.
    pub fn remote_path_to_id(&self, path: &str) -> String;

    /// Parse SAF open mode flags into internal representation.
    pub fn parse_mode(mode: &str) -> SafOpenMode;
}
```

#### 4. Streaming I/O

Unlike iOS, Android SAF requires streaming via `ParcelFileDescriptor`. The Rust layer provides a pipe-based bridge:

```rust
pub struct StreamingReader {
    document_id: String,
    offset: u64,
    length: u64,
    client: HttpClient,
}

impl StreamingReader {
    /// Create a pair of file descriptors: [read_fd, write_fd].
    /// Background task writes HTTP response body to write_fd.
    pub fn create_pipe(document_id: String, offset: u64, length: u64)
        -> Result<(OwnedFd, OwnedFd), MobileApiError>;
}
```

### Implementation Approach

1. **Days 1-3:** Define `saf.rs` types and UniFFI exports. Build JNI bridge for `SafDocumentBackend` trait. Map Ferro REST API types (`MobileFile`, `FileListRequest`) to SAF types (`SafDocument`, `SafAction`).
2. **Days 4-7:** Implement `SafUriMapper` and `SafDocumentBackend` — wire `query_children` to `FileListRequest`, `open_document` to streaming HTTP GET with range support, `create_document` to multipart upload.
3. **Days 8-10:** Android `DocumentsProvider` — implement `FerroDocumentsProvider` in Kotlin, register in `AndroidManifest.xml`, wire `SafDocumentBackend` methods via JNI.
4. **Days 11-13:** Streaming I/O — implement `StreamingReader` pipe bridge, handle write operations (`WriteTruncate` mode), support thumbnail generation via `ContentResolver.loadThumbnail`.
5. **Days 14-15:** Integration testing and Play Store prep.

### Testing Strategy

| Test | Type | Coverage |
|------|------|----------|
| `test_saf_uri_mapper_roundtrip` | Unit | `remote_path_to_id` ↔ `uri_to_remote_path` roundtrip |
| `test_saf_document_flags` | Unit | Correct flags for files vs directories |
| `test_saf_open_mode_parse` | Unit | All `SafOpenMode` variants from string |
| `test_streaming_reader_pipe` | Unit | Data flows through pipe correctly |
| `test_query_children_pagination` | Unit | Handles server pagination via cursor |
| `test_conflict_on_concurrent_write` | Unit | Returns `SyncConflict` error |
| SAF in Files app | Manual | Browse, open, edit, save files via SAF picker |
| Google Drive integration | Manual | Access Ferro files from Google Docs/Sheets |
| Background sync | Manual | App killed → fresh data on next access |
| Large file streaming | Manual | Stream 1GB file without OOM |
| Offline mode | Manual | Airplane mode → cached files accessible |

---

## Cross-Cutting Concerns

### Authentication Flow

All four items share the same auth contract defined in `mobile-contract/src/api.rs`:

```
MobileAuthRequest → server → MobileAuthResponse (access_token + refresh_token)
```

Desktop (CL-001) uses `DesktopConfig.username`/`password` directly with rclone.
Mobile (CL-003, CL-004) uses the token-based auth with `MobileAuthRequest`.

### Sync Protocol

CL-002 (FUSE), CL-003 (iOS), and CL-004 (Android) all use the same incremental sync protocol from `mobile-contract/src/sync.rs`:

```
ChangesSinceRequest { cursor } → ChangesSinceResponse { changes, new_cursor, has_more }
```

### Error Handling

| Crate | Error Type |
|-------|-----------|
| Desktop (CL-001, CL-002) | `anyhow::Result` in internal code, `String` in Tauri commands |
| Mobile (CL-003, CL-004) | `MobileApiError` → platform-native errors via UniFFI |

### Effort Summary

| Item | Days | Depends On |
|------|------|-----------|
| CL-001 | 10 | None |
| CL-002 | 5 | None |
| CL-003 | 15 | `mobile-contract` types stable |
| CL-004 | 15 | `mobile-contract` types stable |
| **Total** | **45** | |
