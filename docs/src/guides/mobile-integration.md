# Mobile Integration Guide

This guide covers the API contracts and implementation approach for iOS (File Provider extension) and Android (SAF provider) integration with the Ferro server.

## API Contracts

All API types are defined in `crates/mobile-contract/src/`. The Rust types serve as the source of truth for both platforms.

### Authentication

```
POST /api/mobile/auth
```

**Request:**
```json
{
  "username": "user",
  "password": "pass",
  "token": null
}
```

**Response:**
```json
{
  "accessToken": "eyJ...",
  "refreshToken": "eyJ...",
  "expiresIn": 3600,
  "user": {
    "id": "usr_abc",
    "username": "user",
    "displayName": "User",
    "storageQuotaBytes": 10737418240,
    "storageUsedBytes": 1073741824
  }
}
```

See `api::MobileAuthRequest`, `api::MobileAuthResponse`, `api::MobileUser`.

### File Listing

```
POST /api/mobile/files/list
```

**Request:**
```json
{
  "path": "/Documents",
  "depth": 1,
  "includeMetadata": true,
  "pageSize": 100,
  "cursor": null
}
```

**Response:**
```json
{
  "files": [
    {
      "path": "/Documents/report.pdf",
      "name": "report.pdf",
      "isDir": false,
      "size": 2048,
      "modified": "2025-01-15T10:30:00Z",
      "created": "2025-01-10T08:00:00Z",
      "contentType": "application/pdf",
      "etag": "\"abc123\"",
      "permissions": {
        "canRead": true,
        "canWrite": true,
        "canDelete": true,
        "canShare": false
      }
    }
  ],
  "nextCursor": "eyJsYXN0X3BhdGgiOiIvRG9jdW1lbnRzL3JlcG9ydC5wZGYifQ=="
}
```

See `api::FileListRequest`, `api::FileListResponse`, `api::MobileFile`, `api::FilePermissions`.

### Upload

```
PUT /api/mobile/files/upload?path=/Documents/new.pdf
```

Binary body with `Content-Type` header. Server returns:

```json
{
  "path": "/Documents/new.pdf",
  "etag": "\"def456\"",
  "size": 4096
}
```

See `api::UploadRequest`, `api::UploadResponse`.

### Download

```
GET /api/mobile/files/download?path=/Documents/report.pdf
Range: bytes=0-1023
```

Binary body response. Supports `Range` headers for partial content.

See `api::DownloadRequest`.

### Folder Creation

```
POST /api/mobile/files/mkdir
```

```json
{ "path": "/Documents/NewFolder" }
```

See `api::CreateFolderRequest`.

### Delete

```
DELETE /api/mobile/files/delete
```

```json
{ "path": "/Documents/old.pdf", "recursive": false }
```

See `api::DeleteRequest`.

### Move

```
POST /api/mobile/files/move
```

```json
{ "from": "/Documents/old.pdf", "to": "/Archive/old.pdf", "overwrite": false }
```

See `api::MoveRequest`.

### Share

```
POST /api/mobile/files/share
```

```json
{
  "path": "/Documents/report.pdf",
  "expiryHours": 48,
  "allowDownload": true,
  "allowUpload": false,
  "password": null
}
```

**Response:**
```json
{
  "id": "shr_abc",
  "token": "abc123",
  "url": "https://ferro.example.com/share/abc123",
  "expiresAt": "2025-01-17T10:30:00Z"
}
```

See `api::ShareCreateRequest`, `api::ShareInfo`.

## Sync Protocol

The sync protocol uses a cursor-based change feed.

### Checkpoint

```
GET /api/mobile/sync/checkpoint?deviceId=ios-001
```

Returns `sync::SyncCheckpoint`:
```json
{
  "cursor": "abc123",
  "lastSync": "2025-01-15T10:00:00Z",
  "deviceId": "ios-001"
}
```

### Changes Since

```
POST /api/mobile/sync/changes
```

```json
{
  "cursor": "abc123",
  "pathPrefix": "/Documents"
}
```

**Response:**
```json
{
  "changes": [
    {
      "path": "/Documents/new.pdf",
      "changeType": { "type": "Created" },
      "etag": "\"def456\"",
      "size": 4096,
      "modified": "2025-01-15T10:30:00Z"
    }
  ],
  "newCursor": "xyz789",
  "hasMore": false
}
```

See `sync::ChangesSinceRequest`, `sync::ChangesSinceResponse`, `sync::FileChange`, `sync::ChangeType`.

### Batch Upload (push local changes)

```
POST /api/mobile/sync/upload
```

```json
{
  "files": [
    {
      "path": "/Documents/new.pdf",
      "contentType": "application/pdf",
      "size": 4096,
      "etag": null
    }
  ]
}
```

**Response:**
```json
{
  "results": [
    {
      "path": "/Documents/new.pdf",
      "status": "created",
      "etag": "\"def456\""
    }
  ]
}
```

See `sync::SyncUploadBatch`, `sync::SyncFileUpload`, `sync::SyncUploadResult`, `sync::SyncUploadStatus`.

## Error Handling

All error responses use `MobileApiError` variants:

| HTTP Status | Error | Description |
|-------------|-------|-------------|
| 401 | `Unauthorized` | Invalid or expired token |
| 403 | `Forbidden` | Insufficient permissions |
| 404 | `NotFound` | Resource does not exist |
| 409 | `Conflict` | Concurrent modification conflict |
| 413 | `QuotaExceeded` | Storage quota exceeded |
| 500+ | `ServerError` | Internal server error |

See `error::MobileApiError`.

## Push Notifications

Mobile devices register push tokens via:

```
POST /api/mobile/notifications/register
```

```json
{
  "deviceId": "ios-001",
  "platform": "ios",
  "pushToken": "apns_token...",
  "appVersion": "1.0.0"
}
```

Notification events:
- `FileShared` — someone shared a file with this user
- `ShareReceived` — a share link was accessed
- `QuotaWarning` — storage usage exceeds threshold
- `SyncConflict` — conflicting edit detected
- `CommentAdded` — comment on a file

See `notifications::PushTokenRegistration`, `notifications::NotificationPayload`, `notifications::NotificationEvent`.

## iOS File Provider Extension

### Architecture

The iOS app uses a **File Provider extension** (`NSFileProviderExtension`) to expose Ferro files in the native Files app. The extension communicates with the main app via app groups and a shared SQLite database.

### Implementation Approach

1. **Extension Target**: Create a File Provider extension target in Xcode. The extension runs in a separate process and is invoked by the system when the user browses Ferro in Files.app.

2. **Domain Setup**: Register an `NSFileProviderDomain` named "Ferro" on app launch. This creates a top-level Ferro entry in Files.app.

3. **Materialization Strategy**:
   - Use **on-demand materialization** — files are not stored locally until the user opens them.
   - On `enumerator(for:)`, fetch the file listing from `/api/mobile/files/list` and return `NSFileProviderItem` instances.
   - On `startProvidingItem(at:)`, download the file content via `GET /api/mobile/files/download` and write it to the container's `File Provider` directory.

4. **Sync with SQLite Cache**:
   - Use a shared SQLite database (via app group) to cache file metadata.
   - The main app syncs periodically using the cursor-based change protocol.
   - The extension reads from SQLite for instant listings and fetches fresh data in the background.

5. **Conflict Resolution**:
   - On upload conflict (HTTP 409), present the user with a merge dialog.
   - Store conflicted versions with a `.conflict` suffix in the local cache.

6. **Background Sync**:
   - Use `BGAppRefreshTask` in the main app to periodically sync changes.
   - Push notifications trigger immediate sync for urgent updates (shared files, conflicts).

7. **Key Classes**:
   - `FerroFileProviderEnumerator` — implements `NSFileProviderEnumerator`
   - `FerroFileProviderItem` — implements `NSFileProviderItem`
   - `FerroSyncManager` — orchestrates sync with the server
   - `FerroDatabase` — SQLite wrapper for cached metadata

### Info.plist Configuration

```xml
<key>NSExtension</key>
<dict>
    <key>NSExtensionPointIdentifier</key>
    <string>com.apple.fileprovider</string>
    <key>NSExtensionPrincipalClass</key>
    <string>$(PRODUCT_MODULE_NAME).FileProviderExtension</string>
</dict>
```

## Android SAF Provider

### Architecture

The Android app uses a **DocumentsProvider** (Storage Access Framework) to expose Ferro files in the system file picker and other apps.

### Implementation Approach

1. **Provider Declaration**: Register a `DocumentsProvider` subclass in `AndroidManifest.xml`:

```xml
<provider
    android:name=".FerroDocumentsProvider"
    android:authorities="${applicationId}.documents"
    android:grantUriPermissions="true"
    android:exported="true"
    android:permission="android.permission.MANAGE_DOCUMENTS">
    <intent-filter>
        <action android:name="android.content.action.DOCUMENTS_PROVIDER" />
    </intent-filter>
</provider>
```

2. **Root Implementation**:
   - `queryRoots()` returns a single root entry for the Ferro server.
   - Store server connection info in encrypted SharedPreferences.

3. **Document Listing**:
   - `queryChildDocuments()` calls `/api/mobile/files/list` for the given parent URI.
   - Map `MobileFile` to `Cursor` columns: `_ID`, `_DISPLAY_NAME`, `MIME_TYPE`, `SIZE`, `LAST_MODIFIED`, `FLAGS`.

4. **File Access**:
   - `openDocument()` downloads the file via `/api/mobile/files/download` and pipes it to a `ParcelFileDescriptor`.
   - Use a local cache directory (`context.cacheDir/ferro`) for recently accessed files.

5. **Write Support**:
   - `createDocument()` creates a folder or uploads a file via PUT.
   - `deleteDocument()` calls the delete endpoint.
   - `renameDocument()` calls the move endpoint with the same parent path.

6. **Caching**:
   - Use Room database for metadata caching.
   - LRU cache for file content (configurable max size).
   - Sync cache on app foreground using the cursor-based change protocol.

7. **Key Classes**:
   - `FerroDocumentsProvider` — extends `DocumentsProvider`
   - `FerroDatabase` — Room database for metadata
   - `FerroApiClient` — HTTP client for server communication
   - `FerroSyncWorker` — WorkManager periodic sync

### MIME Type Mapping

| Extension | MIME Type |
|-----------|-----------|
| pdf | application/pdf |
| jpg/jpeg | image/jpeg |
| png | image/png |
| txt | text/plain |
| (folder) | vnd.android.document/directory |

## Shared Implementation Notes

### Token Management
- Store access and refresh tokens securely (iOS Keychain / Android EncryptedSharedPreferences).
- Refresh tokens before expiry (use `expiresIn` field).
- Re-authenticate on 401 responses.

### Offline Support
- Queue write operations when offline and replay on reconnect.
- Use ETags to detect server-side changes during offline periods.
- Present conflict resolution UI when ETags don't match.

### Bandwidth Optimization
- Use `Range` headers for partial downloads (video preview, large files).
- Compress upload bodies with gzip.
- Use `pageSize` and `cursor` for paginated listings.
