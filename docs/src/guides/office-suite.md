# Office Suite Integration (WOPI)

Ferro implements the [WOPI (Web Application Open Platform Interface)](https://learn.microsoft.com/en-us/microsoft-365/cloud-storage-partner-program/online/) protocol, enabling in-browser document editing with Collabora Online (LibreOffice) or OnlyOffice.

## Architecture

```
User Browser → Ferro (/wopi/files/{path}) → Office Backend (Collabora/OnlyOffice)
                  ↑                                      |
                  |  WOPI REST calls                      |
                  +--------------------------------------+
```

Ferro acts as the WOPI **host**: it serves files and metadata to the office backend via the WOPI REST API. The office backend renders the editor in an iframe and communicates with Ferro for file operations.

## Prerequisites

- Ferro server running with authentication enabled
- Collabora Online or OnlyOffice deployed (Docker recommended)

## Option A: Collabora Online (CODE)

### Deploy Collabora

```bash
docker run -t --rm \
  -p 9980:9980 \
  -e "aliasgroup1=https://ferro.example.com" \
  -e "extra_params=--o:ssl.enable=false --o:ssl.termination=true" \
  --cap-add MKNOD \
  collabora/code:latest
```

Replace `https://ferro.example.com` with your Ferro server's public URL. Collabora validates the host header against this allowlist.

### Configure Ferro

In `ferro.toml`:

```toml
[wopi]
enabled = true
# The public URL of the Collabora server (must be reachable from users' browsers)
office_url = "https://collabora.example.com:9980"
# Allowed file extensions for in-browser editing
allowed_extensions = [".odt", ".ods", ".odp", ".docx", ".xlsx", ".pptx", ".txt", ".csv"]
# Discovery URL (auto-populated from office_url if omitted)
# discovery_url = "https://collabora.example.com:9980/hosting/discovery"
```

### Reverse Proxy Configuration

Both Ferro and Collabora must be behind a reverse proxy (Caddy or Nginx) for TLS termination.

**Caddy** (`Caddyfile`):

```
ferro.example.com {
    # Ferro server
    reverse_proxy / localhost:8080

    # Collabora WebSocket and HTTP
    reverse_proxy /cool/* localhost:9980 {
        header_up Host collabora.example.com
    }
    reverse_proxy /browser localhost:9980 {
        header_up Host collabora.example.com
    }
}
```

**Nginx** (`nginx.conf`):

```nginx
server {
    listen 443 ssl;
    server_name ferro.example.com;

    # Ferro
    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # Collabora Online
    location ^~ /cool {
        proxy_pass http://127.0.0.1:9980;
        proxy_set_header Host $host;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "Upgrade";
    }

    location ^~ /browser {
        proxy_pass http://127.0.0.1:9980;
        proxy_set_header Host $host;
    }

    # Collabora hosting discovery
    location ^~ /hosting/discovery {
        proxy_pass http://127.0.0.1:9980;
        proxy_set_header Host $host;
    }
}
```

## Option B: OnlyOffice Document Server

### Deploy OnlyOffice

```bash
docker run --rm -it -p 8081:80 \
  -e "JWT_SECRET=your-secret-key" \
  -e "WOPI_ENABLED=true" \
  onlyoffice/documentserver:latest
```

### Configure Ferro

```toml
[wopi]
enabled = true
office_url = "https://onlyoffice.example.com"
allowed_extensions = [".docx", ".xlsx", ".pptx", ".odt", ".ods", ".odp", ".txt", ".csv"]
```

### Caddy Configuration

```
ferro.example.com {
    # Ferro
    reverse_proxy / localhost:8080

    # OnlyOffice
    reverse_proxy /ds-vpath/* localhost:8081 {
        header_up Host onlyoffice.example.com
    }
}
```

## WOPI Endpoints

Ferro exposes these WOPI endpoints:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/wopi/files/{path}` | GET | Get file metadata and contents |
| `/wopi/files/{path}` | POST | Save file contents |
| `/wopi/files/{path}/contents` | GET | Download file contents |
| `/wopi/files/{path}/contents` | POST | Upload new file contents |

## File Access Tokens

Ferro generates short-lived WOPI access tokens (configurable TTL, default 1 hour) that grant the office backend read/write access to a specific file. These tokens are separate from user session tokens and are scoped to a single file path.

## Browser Integration

To open a file in the editor, navigate to:

```
https://ferro.example.com/wopi/edit?path=/Documents/report.docx
```

Ferro redirects to the office backend with the WOPI source URL and access token.

## Supported File Formats

| Format | Collabora | OnlyOffice |
|--------|-----------|------------|
| `.odt` (OpenDocument Text) | Yes | Yes |
| `.ods` (OpenDocument Spreadsheet) | Yes | Yes |
| `.odp` (OpenDocument Presentation) | Yes | Yes |
| `.docx` (Word) | Yes | Yes |
| `.xlsx` (Excel) | Yes | Yes |
| `.pptx` (PowerPoint) | Yes | Yes |
| `.txt` | Yes | Limited |
| `.csv` | Yes | Limited |

## Troubleshooting

### "Failed to load document"
- Verify Collabora/OnlyOffice is reachable from user browsers (check CORS headers).
- Ensure `aliasgroup1` in Collabora matches your Ferro domain.
- Check that WOPI discovery URL returns valid XML.

### CORS errors
- Collabora requires the Ferro origin to be listed in its allowed origins.
- Set the `aliasgroup1` environment variable to match `https://ferro.example.com`.

### Token validation failures
- Ensure system clocks are synchronized between Ferro and the office backend.
- Check the WOPI token TTL in Ferro configuration.

### Performance
- Collabora recommends 2 CPU cores and 2 GB RAM per concurrent editing session.
- For production, deploy a dedicated Collabora instance (not the single-container setup above).
