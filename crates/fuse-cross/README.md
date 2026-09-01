# ferro-fuse-cross

Cross-platform FUSE mount for Ferro file sync server.

## Features

- **Linux**: FUSE2 support via `fuser` crate
- **macOS**: macFUSE or FUSE-T support via `fuser` crate
- **Windows**: WinFSP support via `fuser` crate
- All POSIX operations: read, write, mkdir, unlink, rename, readdir
- Bearer token authentication
- WebDAV backend (PROPFIND, GET, PUT, DELETE, MKCOL, MOVE)

## Prerequisites

### Linux
```bash
# Debian/Ubuntu
sudo apt install fuse3

# Fedora/RHEL
sudo dnf install fuse3

# Arch
sudo pacman -S fuse3
```

### macOS
```bash
# Install macFUSE (requires restart)
brew install --cask macfuse

# Or use FUSE-T (user-space, no kernel extension)
# Download from https://www.fuse-t.org/
```

### Windows
```bash
# Download and install WinFSP from https://winfsp.dev/rel/
```

## Usage

```bash
# Basic mount
ferro-fuse-cross --server-url https://ferro.wyattau.com --mount ~/ferro --token YOUR_TOKEN

# With environment variables
export FERRO_URL=https://ferro.wyattau.com
export FERRO_TOKEN=your_token_here
ferro-fuse-cross --mount ~/ferro

# Allow root access
ferro-fuse-cross --server-url https://ferro.wyattau.com --mount ~/ferro --token YOUR_TOKEN --allow-root
```

## Automount Scripts

See `scripts/` directory:
- `ferro-mount-linux.sh` — Linux bash script
- `ferro-mount-macos.sh` — macOS bash script
- `ferro-mount-windows.bat` — Windows batch script
- `ferro-mount.service` — systemd service (Linux)
- `ferro-mount.plist` — launchd agent (macOS)

## How It Works

1. Mounts a FUSE filesystem at the specified mount point
2. Translates POSIX file operations to WebDAV HTTP requests
3. Authenticates via Bearer token (obtained from Keycloak OIDC)
4. Files are stored on the Ferro server and accessible from any client

## Supported Operations

| Operation | Status |
|-----------|--------|
| read | ✅ |
| write | ✅ |
| mkdir | ✅ |
| unlink | ✅ |
| rename | ✅ |
| readdir | ✅ |
| getattr | ✅ |
| lookup | ✅ |
| open | ✅ |
| release | ✅ |

## Testing

```bash
# Run unit tests
cargo test -p ferro-fuse-cross

# Run integration test (requires running Ferro server)
cargo test -p ferro-fuse-cross -- --test-threads=1
```
