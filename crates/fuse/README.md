# ferro-fuse

[![crates.io](https://img.shields.io/crates/v/ferro-fuse.svg)](https://crates.io/crates/ferro-fuse)
[![docs.rs](https://docs.rs/ferro-fuse/badge.svg)](https://docs.rs/ferro-fuse)
[![license](https://img.shields.io/badge/license-AGPL-3.0-blue.svg)](LICENSE)

FUSE3 filesystem mount for Ferro. Translates POSIX file operations into WebDAV HTTP requests, letting you access a remote Ferro server as a local directory on Linux.

## Features

- Read, write, create, and delete files via WebDAV
- Directory creation (`mkdir`), removal (`rmdir`), and listing (`readdir`)
- Rename (`rename`) and hard-link-like copy operations
- In-memory file cache (10 MB, 10 000 entries) for read performance
- Token-based authentication via `FERRO_TOKEN` environment variable
- `allow-root` mount option for privileged access
- Automatic directory creation for mount point

## Installation

```bash
cargo install ferro-fuse
```

## Usage

Mount a Ferro server as a local filesystem:

```bash
# Basic mount
ferro-fuse --server-url https://ferro.example.com --mount /mnt/ferro --token YOUR_TOKEN

# With environment variables
export FERRO_URL=https://ferro.example.com
export FERRO_MOUNT=/mnt/ferro
export FERRO_TOKEN=YOUR_TOKEN
ferro-fuse

# Allow root access
ferro-fuse --server-url https://ferro.example.com --mount /mnt/ferro --allow-root

# Run in background
ferro-fuse --server-url https://ferro.example.com --mount /mnt/ferro --no-foreground
```

### Options

| Option | Env | Default | Description |
|--------|-----|---------|-------------|
| `--server-url` | `FERRO_URL` | `http://localhost:8080` | Ferro server URL |
| `--mount` | `FERRO_MOUNT` | (required) | Local mount point path |
| `--token` | `FERRO_TOKEN` | (none) | Bearer token for authentication |
| `--allow-root` | — | `false` | Allow root user to access the mount |
| `--no-foreground` | — | `true` (foreground) | Run in the background |

### Unmounting

```bash
fusermount -u /mnt/ferro
```

or press `Ctrl+C` when running in the foreground.

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

This crate targets **Linux only** via the `fuse3` crate. Building on non-Linux platforms will compile but the binary will exit with an error message at startup.

## License

Licensed under AGPL-3.0-or-later.
