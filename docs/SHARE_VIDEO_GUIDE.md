# How to: upload a video and share it Google-Drive/oCIS style (Ferro)

All endpoints and the web-UI flow verified live against a running server
(18 MB MP4 uploaded through the browser, streamed back with Range requests).

## 1. Upload the video

**Web UI** (verified): sign in at `/ui` → **Upload** → pick the file. The
file appears in the list and lands at your user root
(`/users/<user>/<name>`).

**WebDAV** (works with any client — Finder, Windows Explorer, Cyberduck, rclone):

```bash
curl -u admin:PASSWORD -X PUT \
  -H "Content-Type: video/mp4" \
  --data-binary @my-video.mp4 \
  https://ferro.example.com/dav/videos/my-video.mp4
```

DAV uploads land under `/dav/...`, i.e. storage path
`/users/<user>/dav/videos/my-video.mp4`.

## 2. Create a public share link

**Web UI** (verified): row action **Share** → *Create Link* → the dialog
shows the `SHARE URL`. Options in the dialog: password, expiry
(1h/24h/7d/30d), download limit.

**API**:

```bash
curl -u admin:PASSWORD -X POST https://ferro.example.com/api/v1/shares \
  -H "Content-Type: application/json" \
  -d '{
    "path": "/users/admin/4463.mp4",
    "allow_download": true,
    "expires_in_hours": 168
  }'
# → {"token": "<uuid>", "url": "/s/<uuid>", ...}
```

Options mirror oCIS/Drive public links:
- `password` — link password (hash-verified, lockout after repeated failures)
- `expires_in_hours` — automatic expiry
- `max_downloads` — hard download cap (Range/seek requests do NOT count)
- `allow_download: false` — view-only (serves a preview page)
- `allow_upload: true` — turns the link into a drop zone (file collection)

QR code for the link: `GET /api/v1/shares/:token/qr`.

## 3. Stream it

`GET /s/<uuid>` supports HTTP Range (RFC 7233) — verified live on a real
18 MB MP4, from an anonymous client and from inside the browser:

- `Range: bytes=0-1023` → `206 Partial Content`,
  `Content-Type: video/mp4`, `Content-Range: bytes 0-1023/18187229`
- suffix ranges (`bytes=-10`), open ranges (`bytes=N-`)
- `416` for out-of-bounds, `Accept-Ranges: bytes` always advertised
- seeking does not consume the download quota

`video/mp4` (and other types) come from extension-based MIME detection in
the storage metadata — browsers can play the link inline in `<video>`.

## Notes and gotchas (each one bit during testing)

1. **Share paths are storage paths.** A WebDAV upload to `/dav/videos/x.mp4`
   is stored at `/users/<user>/dav/videos/x.mp4` — pass that path to
   `POST /shares`. Files uploaded through the web UI live directly under
   `/users/<user>/`.
2. **Browse via an allowed CORS origin.** The server rejects
   non-simple methods (PROPFIND, PUT) carrying an `Origin` header that is
   not in `--cors-origins`. Browsing `http://127.0.0.1:8199` while the
   allow-list has `http://localhost:8199` yields 403 on the file list even
   though GETs work (GETs don't send Origin). Match them.
3. **First boot requires a password rotation.** With the default admin
   password in place, everything except health/config endpoints answers
   `403 PASSWORD_CHANGE_REQUIRED`. `POST /api/auth/change-password` lifts
   it. Restarting with `FERRO_ADMIN_PASSWORD` resets the password to the
   env value and re-arms the gate.
4. **Web UI login in basic mode.** `/api/config` now reports
   `auth_enabled: true` + `auth_mode: "basic"` for admin-password mode, and
   `/ui/auth/login` shows a username/password form (Basic auth, stored
   locally, verified against an authenticated endpoint before use). OIDC
   deployments keep the provider-redirect flow.
