# How to: upload a video and share it Google-Drive/oCIS style (Ferro)

All endpoints verified live against a running server.

## 1. Upload the video (WebDAV PUT — works with any WebDAV client)

```bash
curl -u admin:PASSWORD -X PUT \
  -H "Content-Type: video/mp4" \
  --data-binary @my-video.mp4 \
  https://ferro.example.com/dav/videos/my-video.mp4
```

Any WebDAV client works (macOS Finder, Windows Explorer, Cyberduck, rclone).

## 2. Create a public share link

```bash
curl -u admin:PASSWORD -X POST https://ferro.example.com/api/v1/shares \
  -H "Content-Type: application/json" \
  -d '{
    "path": "/dav/videos/my-video.mp4",
    "allow_download": true,
    "expires_in_hours": 168,
    "max_downloads": null,
    "password": null
  }'
# → {"token": "<uuid>", "url": "/s/<uuid>", ...}
```

Share link: `https://ferro.example.com/s/<uuid>` — no login required.

Options mirror oCIS/Drive public links:
- `password` — link password (hash-verified, lockout after repeated failures)
- `expires_in_hours` — automatic expiry
- `max_downloads` — hard download cap (Range/seek requests do NOT count)
- `allow_download: false` — view-only (serves a preview page)
- `allow_upload: true` — turns the link into a drop zone (file collection)

QR code for the link: `GET /api/v1/shares/:token/qr`.

## 3. Stream it

`GET /s/<uuid>` supports HTTP Range (RFC 7233) — verified: `206 Partial
Content`, `Content-Range: bytes 10-19/54`, suffix ranges (`bytes=-10`),
416 for out-of-bounds, `Accept-Ranges: bytes` advertised. Browsers and
players can seek; seeking does not consume the download quota.

Note: share paths are storage paths — a WebDAV upload lands under your
user root, so the share path is `/dav/videos/my-video.mp4` prefixed by the
user root (`/users/<user>/dav/...`). Pass the storage path (visible in
server logs or via the file-list API) to `POST /shares` when sharing
files uploaded through WebDAV.

## Gap worth closing (next pass)

`serve_share` currently emits `application/octet-stream` because the DAV
PUT's `Content-Type: video/mp4` is not persisted into file metadata —
browsers want `video/mp4` on the response to play `<video>` inline.
Persist the PUT content-type (or sniff it at share time) so the player
gets the right type.
