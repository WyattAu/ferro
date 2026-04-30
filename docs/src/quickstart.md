# Quick Start

Get Ferro running in under 30 seconds with Docker.

## Docker Quickstart

```bash
docker compose up -d
```

The server is available at `http://localhost:8080`. The Docker image includes the bundled Leptos web UI and a Caddy reverse proxy with automatic HTTPS.

## Basic Operations

### Upload a file

```bash
curl -X PUT http://localhost:8080/hello.txt \
  -H "Content-Type: text/plain" \
  -d "Hello, Ferro!"
```

### Download a file

```bash
curl http://localhost:8080/hello.txt
```

### List files

```bash
curl -X PROPFIND http://localhost:8080/ \
  -H "Depth: 1" \
  -H "Content-Type: application/xml" \
  -d '<?xml version="1.0" encoding="utf-8"?>
       <D:propfind xmlns:D="DAV:">
         <D:prop>
           <D:resourcetype/>
           <D:getcontentlength/>
           <D:getlastmodified/>
         </D:prop>
       </D:propfind>'
```

### Create a folder

```bash
curl -X MKCOL http://localhost:8080/documents/
```

### Delete a file

```bash
curl -X DELETE http://localhost:8080/hello.txt
```

### Move a file

```bash
curl -X MOVE http://localhost:8080/hello.txt \
  -H "Destination: /documents/hello.txt"
```

### Copy a file

```bash
curl -X COPY http://localhost:8080/hello.txt \
  -H "Destination: /documents/hello-backup.txt"
```

## Connecting with a WebDAV Client

### rclone

```bash
rclone config
# Choose: WebDAV, vendor: Other, URL: http://localhost:8080

rclone ls ferro:
rclone copy local-file.txt ferro:documents/
```

### macOS Finder

1. Open Finder
2. Go > Connect to Server (Cmd+K)
3. Enter `http://localhost:8080/`
4. Click Connect

### Windows Explorer

1. Open File Explorer
2. Right-click "This PC" > Map network drive
3. Enter `http://localhost:8080/`

### Linux (GNOME)

1. Open Files (Nautilus)
2. Other Locations > Connect to Server
3. Enter `dav://localhost:8080/`

## Health Check

Verify the server is running:

```bash
curl http://localhost:8080/.well-known/ferro
```

## Next Steps

- [Configuration](./configuration.md) -- Customize your server
- [Architecture](./architecture.md) -- Understand how it works
- [Deployment](./deployment/SUMMARY.md) -- Production deployment options
