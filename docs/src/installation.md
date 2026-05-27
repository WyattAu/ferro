# Installation

## Binary Download

Download pre-built binaries from [GitHub Releases](https://github.com/WyattAu/ferro/releases/latest):

```bash
curl -sL https://github.com/WyattAu/ferro/releases/latest/download/ferro-server-linux -o ferro-server
chmod +x ferro-server
./ferro-server --port 8080
```

## Docker

### Docker Compose (recommended)

```bash
cd deploy
docker compose up -d
```

The Docker image is published at `ghcr.io/wyattau/ferro:latest`.

### Docker with PostgreSQL

```bash
POSTGRES_PASSWORD=your-password docker compose -f docker-compose.yml -f docker-compose.pg.yml up -d
```

### Docker with PostgreSQL and Redis

```bash
POSTGRES_PASSWORD=your-password docker compose -f docker-compose.yml -f docker-compose.pg.yml -f docker-compose.redis.yml up -d
```

## Cargo Build from Source

### Prerequisites

- Rust 1.95.0+ (edition 2024, pinned in `rust-toolchain.toml`)
- OpenSSL (for PostgreSQL support)

### Build

```bash
git clone https://github.com/WyattAu/ferro.git
cd ferro
cargo build --release --bin ferro-server
./target/release/ferro-server
```

### Build with all storage backends

```bash
cargo build --release --features s3,gcs,azure --bin ferro-server
```

### Run tests

```bash
cargo test --all
```

### Nix

```bash
nix develop           # Full dev environment
nix develop .#web     # WASM build environment
nix develop .#desktop # Tauri desktop environment
```

## System Requirements

### Minimum

| Resource | Requirement |
|----------|-------------|
| CPU | 1 core |
| RAM | 128 MB |
| Disk | 50 MB (binary) |
| OS | Linux, macOS, Windows |

### Recommended (production)

| Resource | Requirement |
|----------|-------------|
| CPU | 2+ cores |
| RAM | 512 MB |
| Disk | Depends on storage backend |
| OS | Linux (kernel 5.4+) |

### Runtime Dependencies

| Dependency | Required | Purpose |
|------------|----------|---------|
| OpenSSL | If using PostgreSQL | TLS for database connections |
| FUSE kernel module | For ferro-fuse | Filesystem mount support |
| tuntap kernel module | For Firecracker | MicroVM networking |

## Installing Additional Components

### FUSE mount client

```bash
cargo install ferro-fuse
```

### Admin CLI

```bash
cargo install ferro-cli
```

### Desktop app

See [Desktop App Guide](./guides/desktop-app.md) for Tauri installation instructions.
