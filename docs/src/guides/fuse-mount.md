# FUSE Mount

Mount a remote Ferro server as a local directory on Linux using `ferro-fuse`.

## Installation

```bash
cargo install ferro-fuse
```

### Prerequisites

- Linux kernel with FUSE support (most distributions include this)
- `fuse3` development libraries (for building from source)
- The `fusermount` command (included with FUSE)

## Mounting a Server

### Basic mount

```bash
ferro-fuse \
  --server-url https://ferro.example.com \
  --mount /mnt/ferro \
  --token YOUR_TOKEN
```

### Using environment variables

```bash
export FERRO_URL=https://ferro.example.com
export FERRO_MOUNT=/mnt/ferro
export FERRO_TOKEN=YOUR_TOKEN
ferro-fuse
```

### Allow root access

```bash
ferro-fuse \
  --server-url https://ferro.example.com \
  --mount /mnt/ferro \
  --token YOUR_TOKEN \
  --allow-root
```

### Background mode

```bash
ferro-fuse \
  --server-url https://ferro.example.com \
  --mount /mnt/ferro \
  --token YOUR_TOKEN \
  --no-foreground
```

## Authentication

The FUSE mount authenticates using a Bearer token. Set the token via:

- `--token` CLI flag
- `FERRO_TOKEN` environment variable

When the server uses simple auth (`--admin-user` / `--admin-password`), the token is the admin password. When OIDC is configured, use a valid access token.

## Offline Cache

The FUSE mount includes an in-memory cache for improved read performance:

| Parameter | Value |
|-----------|-------|
| Cache size | 10 MB |
| Max entries | 10,000 |
| Eviction policy | LRU |

The cache is in-memory only -- it does not persist across mounts.

## Unmounting

### Foreground (Ctrl+C)

Press `Ctrl+C` when running in the foreground. The mount point is cleaned up automatically.

### fusermount

```bash
fusermount -u /mnt/ferro
```

### Force unmount

```bash
fusermount -uz /mnt/ferro
```

## Supported Operations

| Operation | Description |
|-----------|-------------|
| `read` | Read file contents |
| `write` | Create or overwrite files |
| `mkdir` | Create directories |
| `rmdir` | Remove empty directories |
| `unlink` | Delete files |
| `rename` | Move or rename files and directories |
| `readdir` | List directory contents |
| `getattr` / `stat` | Retrieve file metadata |
| `open` / `release` | Open and close file handles |

## Tips

- Use `/etc/fstab` for persistent mounts (not recommended for remote FUSE)
- Set `--allow-root` if you need root to access the mount
- The mount translates POSIX operations to WebDAV HTTP requests
- Large file operations may be slower than native filesystem access due to HTTP overhead
- Use `df -h /mnt/ferro` to check mount status

## Troubleshooting

### "Transport endpoint is not connected"

The FUSE connection was lost. Unmount and remount:

```bash
fusermount -uz /mnt/ferro
ferro-fuse --server-url https://ferro.example.com --mount /mnt/ferro --token TOKEN
```

### Permission denied

Ensure your token is valid and the mount point exists:

```bash
mkdir -p /mnt/ferro
curl -H "Authorization: Bearer TOKEN" https://ferro.example.com/.well-known/ferro
```
