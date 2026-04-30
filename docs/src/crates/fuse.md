# ferro-fuse

FUSE3 filesystem mount for Ferro. Translates POSIX file operations into WebDAV HTTP requests, letting you access a remote Ferro server as a local directory on Linux.

## Key Types

| Type | Description |
|------|-------------|
| `FUSEMount` | Main FUSE filesystem implementation |

## Features

- Read, write, create, and delete files via WebDAV
- Directory creation (`mkdir`), removal (`rmdir`), and listing (`readdir`)
- Rename and copy operations
- In-memory file cache (10 MB, 10,000 entries) for read performance
- Token-based authentication via `FERRO_TOKEN` environment variable
- `allow-root` mount option for privileged access
- Automatic directory creation for mount point

## Installation

```bash
cargo install ferro-fuse
```

## CLI Options

| Option | Env | Default | Description |
|--------|-----|---------|-------------|
| `--server-url` | `FERRO_URL` | `http://localhost:8080` | Ferro server URL |
| `--mount` | `FERRO_MOUNT` | (required) | Local mount point path |
| `--token` | `FERRO_TOKEN` | (none) | Bearer token for authentication |
| `--allow-root` | -- | `false` | Allow root user to access the mount |
| `--no-foreground` | -- | `true` (foreground) | Run in the background |

## Minimal Usage

### Mount a server

```bash
ferro-fuse --server-url https://ferro.example.com --mount /mnt/ferro --token YOUR_TOKEN
```

### With environment variables

```bash
export FERRO_URL=https://ferro.example.com
export FERRO_MOUNT=/mnt/ferro
export FERRO_TOKEN=YOUR_TOKEN
ferro-fuse
```

### Unmount

```bash
fusermount -u /mnt/ferro
```

Or press `Ctrl+C` when running in the foreground.

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

## Platform Support

This crate targets **Linux only** via the `fuse3` crate. Building on non-Linux platforms compiles but the binary exits with an error at startup.

## See Also

- [FUSE Mount Guide](../guides/fuse-mount.md) for detailed setup instructions
