# WebDAV Client Guide

Ferro provides a fully compliant WebDAV server (Class 1, 2, and 3). This guide covers connecting with various clients.

## Quick Test

Verify your Ferro server is running:

```bash
curl -X PROPFIND http://localhost:8080/ -H "Depth: 0"
```

## Supported WebDAV Methods

| Method | Description |
|--------|-------------|
| `OPTIONS` | Returns supported methods and DAV capabilities |
| `PROPFIND` | List files and metadata (with `Depth` header) |
| `GET` | Download a file |
| `HEAD` | Get file metadata without content |
| `PUT` | Upload or update a file |
| `DELETE` | Delete a file or collection |
| `MKCOL` | Create a collection (directory) |
| `COPY` | Copy a resource |
| `MOVE` | Move/rename a resource |
| `LOCK` | Lock a resource (exclusive) |
| `UNLOCK` | Release a lock |
| `PROPPATCH` | Set/remove resource properties |

## rclone (Recommended)

rclone is the most full-featured WebDAV client and the primary target for Ferro's WebDAV implementation.

### Installation

```bash
# macOS
brew install rclone

# Ubuntu/Debian
sudo apt install rclone

# Or download from https://rclone.org/install/
```

### Configure

```bash
rclone config
# Choose: n (New remote)
# Name: ferro
# Storage: webdav
# URL: http://localhost:8080
# Vendor: other
# User: (leave blank for anonymous)
# Password: (leave blank for anonymous)
```

Or create the config directly in `~/.config/rclone/rclone.conf`:

```ini
[ferro]
type = webdav
url = http://localhost:8080
vendor = other
```

### Commands

```bash
# List files
rclone ls ferro:

# Copy a file
rclone copy localfile.txt ferro:documents/

# Sync a directory
rclone sync ./local-folder ferro:backup/

# Mount as a filesystem
rclone mount ferro: /mnt/ferro --vfs-cache-mode full

# Mount in background
rclone mount ferro: /mnt/ferro --vfs-cache-mode full --daemon
```

### Mount Options

For optimal performance with Ferro:

```bash
rclone mount ferro: /mnt/ferro \
  --vfs-cache-mode full \
  --vfs-cache-max-size 10G \
  --vfs-cache-max-age 24h \
  --buffer-size 32M \
  --dir-cache-time 5m \
  --transfers 4
```

### With OIDC Authentication

If OIDC is enabled, pass the Bearer token:

```ini
[ferro]
type = webdav
url = http://localhost:8080
vendor = other
bearer_token = your-access-token
```

## Cyberduck

1. Open Cyberduck
2. Click "Open Connection"
3. Select **WebDAV (HTTP)** or **WebDAV (HTTPS)**
4. Server: `localhost` (or your domain)
5. Port: `8080`
6. Path: `/`
7. Click "Connect"

For OIDC-enabled servers, use the **WebDAV (HTTP) with Bearer Token** profile and enter your access token.

## Windows Explorer

### Windows 10 / 11

1. Open File Explorer
2. Right-click "This PC" → "Map network drive"
3. Choose a drive letter
4. Folder: `http://localhost:8080`
5. Check "Connect using different credentials" if auth is enabled
6. Click "Finish"

### Troubleshooting Windows WebDAV

Windows Explorer has limited WebDAV support. If you encounter issues:

- Use **WinFsp** + **rclone mount** instead for full compatibility
- Increase the file size limit: set `FileSizeLimitInBytes` in `HKEY_LOCAL_MACHINE\SYSTEM\CurrentControlSet\Services\WebClient\Parameters`
- Restart the WebClient service: `net stop webclient && net start webclient`

## macOS Finder

1. Open Finder
2. Go → Connect to Server (⌘K)
3. Enter: `http://localhost:8080/`
4. Click "Connect"

For OIDC authentication, Finder has limited support. Use rclone mount instead:

```bash
rclone mount ferro: /Volumes/Ferro --vfs-cache-mode full
```

## GNOME Files (Nautilus) / Linux

### Connect via GUI

1. Open GNOME Files
2. Press `Ctrl+L` to enter a location
3. Enter: `dav://localhost:8080/`
4. Press Enter

### Mount via command line (GNOME)

```bash
gio mount dav://localhost:8080/
```

### Mount via command line (davfs2)

```bash
sudo apt install davfs2
sudo mount -t davfs http://localhost:8080 /mnt/ferro
```

Add to `/etc/fstab` for persistent mounting:

```
http://localhost:8080 /mnt/ferro davfs rw,noauto,user 0 0
```

## curl Examples

### List files (PROPFIND)

```bash
# Root listing (depth: 1)
curl -X PROPFIND http://localhost:8080/ \
  -H "Depth: 1" \
  -H "Content-Type: application/xml"

# Single file metadata (depth: 0)
curl -X PROPFIND http://localhost:8080/document.pdf \
  -H "Depth: 0"
```

### Create a directory (MKCOL)

```bash
curl -X MKCOL http://localhost:8080/new-folder/
```

### Upload a file (PUT)

```bash
curl -X PUT http://localhost:8080/new-folder/file.txt \
  --data-binary @localfile.txt \
  -H "Content-Type: text/plain"
```

### Download a file (GET)

```bash
curl -O http://localhost:8080/new-folder/file.txt
```

### Delete a file (DELETE)

```bash
curl -X DELETE http://localhost:8080/new-folder/file.txt
```

### Copy a file (COPY)

```bash
curl -X COPY http://localhost:8080/file.txt \
  -H "Destination: http://localhost:8080/file-copy.txt"
```

### Move a file (MOVE)

```bash
curl -X MOVE http://localhost:8080/file.txt \
  -H "Destination: http://localhost:8080/renamed-file.txt"
```

### Lock a file (LOCK)

```bash
curl -X LOCK http://localhost:8080/document.docx \
  -H "Content-Type: application/xml" \
  -d '<?xml version="1.0" encoding="utf-8"?>
<D:lockinfo xmlns:D="DAV:">
  <D:locktype><D:write/></D:locktype>
  <D:lockscope><D:exclusive/></D:lockscope>
  <D:owner><D:href>user@example.com</D:href></D:owner>
</D:lockinfo>'
```

### Refresh a lock (LOCK with If header)

Per RFC 4918 Section 9.10.2, a lock refresh is a LOCK request with no body and the `If` header containing the lock token.

```bash
curl -X LOCK http://localhost:8080/document.docx \
  -H "If: (<urn:uuid:lock-token>)"
```

### Unlock a file (UNLOCK)

```bash
curl -X UNLOCK http://localhost:8080/document.docx \
  -H "Lock-Token: <urn:uuid:lock-token>"
```

## Microsoft Office

Ferro's WOPI support enables online editing via Collabora Online or OnlyOffice. Microsoft Office can also open files directly via WebDAV:

1. File → Open → Browse
2. Enter the WebDAV URL: `http://localhost:8080/documents/report.docx`
3. Office connects via WebDAV and supports locking for collaborative editing

## Per-User Path Isolation

When OIDC is enabled, authenticated users are automatically isolated to `/users/{sub}/`. This means:

- User `alice@example.com` sees their files under `/users/alice-sub/`
- User `bob@example.com` sees their files under `/users/bob-sub/`
- The root `/` redirects to the user's own namespace

Anonymous users access the global namespace directly.

## Locking

Ferro supports WebDAV Class 2 locking (RFC 4918):

- Locks are exclusive and scoped to depth zero (single resource)
- Locks have a configurable timeout
- LOCK refresh is supported via the `If` header (RFC 4918 §9.10.2)
- Lock tokens use the `urn:uuid:` format
- Locks are stored in-memory and do not survive server restarts

## Pre-Signed URLs

For large file transfers, use pre-signed URLs to upload/download directly to cloud storage (S3, GCS, Azure), bypassing Ferro's CPU:

```bash
# Get an upload URL
curl "http://localhost:8080/api/upload-url?path=/large-file.zip&expires=3600"

# Upload directly to S3 using the returned URL
curl -X PUT "https://bucket.s3.amazonaws.com/large-file.zip?..." \
  --upload-file large-file.zip
```
