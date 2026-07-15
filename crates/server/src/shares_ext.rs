//! Extended share types: secure view, file drop, upload shares.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use rusqlite::params;
use std::collections::HashMap;

use crate::AppState;
use crate::api_error::ApiError;
use crate::shares::ShareLink;
use ferro_server_state::ServerState;

pub use ferro_server_sharing::shares_ext::{
    CreateShareRequestExt, ShareType, ShareUploadEntry, sanitize_filename, sniff_mime_type,
};

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `POST /api/shares/ext` -- Create a share link with extended type support.
pub async fn create_share_ext_impl<S: ServerState>(state: &S, req: CreateShareRequestExt) -> Response {
    use crate::shares::CreateShareRequest;

    if req.share_type == ShareType::Upload
        && let Ok(meta) = state.storage().head(&req.path).await
        && !meta.is_collection
    {
        return ApiError::bad_request(ApiError::INVALID_INPUT, "Upload share target must be a directory");
    }

    if req.share_type == ShareType::View
        && let Ok(meta) = state.storage().head(&req.path).await
        && meta.is_collection
    {
        return ApiError::bad_request(
            ApiError::INVALID_INPUT,
            "View share target must be a file, not a directory",
        );
    }

    let base_req = CreateShareRequest {
        path: req.path.clone(),
        password: req.password.clone(),
        expires_in_hours: req.expires_in_hours,
        max_downloads: req.max_downloads,
        allow_download: Some(req.allow_download),
        allow_upload: req.allow_upload.or_else(|| {
            if req.share_type == ShareType::Upload {
                Some(true)
            } else {
                None
            }
        }),
    };
    let link = state.share_store().create(base_req, "anonymous".to_string()).await;

    if let Some(db) = state.db() {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        if let Err(e) = conn.execute(
            "UPDATE shares SET share_type = ?1, allow_download = ?2, upload_target = ?3 WHERE token = ?4",
            params![
                req.share_type.to_string(),
                if req.allow_download { 1i32 } else { 0i32 },
                if req.share_type == ShareType::Upload {
                    Some(req.path.clone())
                } else {
                    None::<String>
                },
                link.token,
            ],
        ) {
            tracing::warn!(error = %e, "failed to persist extended share fields");
        }
    }

    (
        StatusCode::CREATED,
        axum::Json(serde_json::json!({
            "token": link.token,
            "url": format!("/s/{}", link.token),
            "path": link.path,
            "share_type": req.share_type.to_string(),
            "expires_at": link.expires_at.to_rfc3339(),
            "max_downloads": link.max_downloads,
            "allow_download": req.allow_download,
            "allow_upload": link.allow_upload,
        })),
    )
        .into_response()
}

/// `POST /api/shares/ext` -- Create a share link with extended type support.
pub async fn create_share_ext(
    State(state): State<AppState>,
    axum::Json(req): axum::Json<CreateShareRequestExt>,
) -> Response {
    create_share_ext_impl(&state, req).await
}

/// `POST /s/:token/upload` -- Upload a file to an upload-only share.
pub async fn upload_to_share_impl<S: ServerState>(state: &S, token: String, body: axum::body::Body) -> Response {
    let link = match state.share_store().get(&token).await {
        Some(l) => l,
        None => return ApiError::not_found(ApiError::SHARE_NOT_FOUND, "Share not found"),
    };

    if link.expires_at < chrono::Utc::now() {
        return ApiError::gone(ApiError::SHARE_EXPIRED, "Share expired");
    }

    let share_type = get_share_type(state, &token);
    if share_type != ShareType::Upload {
        return ApiError::bad_request(ApiError::INVALID_INPUT, "This share link does not accept uploads");
    }

    let bytes = match axum::body::to_bytes(body, state.max_body_size() as usize).await {
        Ok(b) => b,
        Err(e) => {
            return ApiError::with_details(
                StatusCode::PAYLOAD_TOO_LARGE,
                ApiError::PAYLOAD_TOO_LARGE,
                "Upload too large",
                e.to_string(),
            );
        }
    };

    let file_name = format!("upload_{}", uuid::Uuid::new_v4());
    let target_path = format!("{}/{}", link.path.trim_end_matches('/'), file_name);

    if state.storage().head(&link.path).await.is_err()
        && let Err(e) = state.storage().create_collection(&link.path, "anonymous").await
    {
        tracing::warn!(error = %e, path = %link.path, "failed to create upload target directory");
        return ApiError::internal(ApiError::INTERNAL_ERROR, "Failed to create upload directory");
    }

    let content_type = sniff_mime_type(&file_name);
    if let Err(e) = state.storage().put(&target_path, bytes.clone(), "anonymous").await {
        tracing::warn!(error = %e, path = %target_path, "failed to store uploaded file");
        return ApiError::internal(ApiError::INTERNAL_ERROR, "Failed to store uploaded file");
    }

    let audit_entry =
        crate::audit::build_audit_entry("POST", &format!("/s/{}/upload", token), "anonymous", 201, None, None);
    state
        .audit_log()
        .log(ferro_server_state::AuditEntry {
            timestamp: audit_entry.timestamp,
            method: audit_entry.method,
            path: audit_entry.path,
            user: audit_entry.user,
            status: audit_entry.status,
            client_ip: audit_entry.client_ip,
            user_agent: audit_entry.user_agent,
            content_length: audit_entry.content_length,
        })
        .await;

    if let Some(db) = state.db() {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let upload_id = uuid::Uuid::new_v4().to_string();
        if let Err(e) = conn.execute(
            "INSERT INTO share_uploads (id, share_token, file_path, size, mime_type, uploaded_by) VALUES (?1, ?2, ?3, ?4, ?5, 'anonymous')",
            params![upload_id, token, target_path, bytes.len() as i64, content_type],
        ) {
            tracing::warn!(error = %e, "failed to record share upload");
        }
    }

    (
        StatusCode::CREATED,
        axum::Json(serde_json::json!({
            "path": target_path,
            "size": bytes.len(),
            "content_type": content_type,
        })),
    )
        .into_response()
}

/// `POST /s/:token/upload` -- Upload a file to an upload-only share.
pub async fn upload_to_share(
    State(state): State<AppState>,
    Path(token): Path<String>,
    body: axum::body::Body,
) -> Response {
    upload_to_share_impl(&state, token, body).await
}

/// `GET /s/:token/uploads` -- List uploads for an upload share.
pub async fn list_share_uploads_impl<S: ServerState>(state: &S, token: String) -> Response {
    let entries = if let Some(db) = state.db() {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = match conn.prepare(
            "SELECT id, file_path, size, mime_type, uploaded_at FROM share_uploads WHERE share_token = ?1 ORDER BY uploaded_at DESC",
        ) {
            Ok(s) => s,
            Err(_) => {
                return (
                    StatusCode::OK,
                    axum::Json(serde_json::json!({ "uploads": [] })),
                )
                    .into_response();
            }
        };
        let mut entries = Vec::new();
        if let Ok(rows) = stmt.query_map(params![token], |row| {
            Ok(ShareUploadEntry {
                id: row.get(0)?,
                file_name: row
                    .get::<_, String>(1)?
                    .rsplit('/')
                    .next()
                    .unwrap_or("unknown")
                    .to_string(),
                size: row.get::<_, i64>(2)? as u64,
                mime_type: row.get(3)?,
                uploaded_at: row.get(4)?,
            })
        }) {
            for row in rows.flatten() {
                entries.push(row);
            }
        }
        entries
    } else {
        Vec::new()
    };

    (StatusCode::OK, axum::Json(serde_json::json!({ "uploads": entries }))).into_response()
}

/// `GET /s/:token/uploads` -- List uploads for an upload share.
pub async fn list_share_uploads(State(state): State<AppState>, Path(token): Path<String>) -> Response {
    list_share_uploads_impl(&state, token).await
}

/// `GET /s/:token/view` -- Serve a secure view share (preview only).
pub async fn serve_view_share_impl<S: ServerState>(
    state: &S,
    token: String,
    params: HashMap<String, String>,
) -> Response {
    if state.share_store().is_share_locked(&token) {
        return ApiError::with_details(
            StatusCode::TOO_MANY_REQUESTS,
            ApiError::RATE_LIMITED,
            "Too many failed password attempts. Try again later.",
            format!("{} seconds remaining", 300),
        );
    }

    let link = match state.share_store().get(&token).await {
        Some(l) => l,
        None => return ApiError::not_found(ApiError::SHARE_NOT_FOUND, "Share not found"),
    };

    if link.expires_at < chrono::Utc::now() {
        return ApiError::gone(ApiError::SHARE_EXPIRED, "Share expired");
    }

    let share_type = get_share_type(state, &token);
    if share_type != ShareType::View {
        return ApiError::bad_request(ApiError::INVALID_INPUT, "This share link is not a view share");
    }

    if let Some(ref required_password) = link.password {
        let provided = params.get("password").map(|s| s.as_str());
        match provided {
            Some(pw) if constant_time_eq(pw, required_password) => {
                state.share_store().clear_failed_attempts(&token);
            }
            Some(_) => {
                state.share_store().record_failed_attempt(&token);
                return ApiError::unauthorized(ApiError::SHARE_PASSWORD_INVALID, "Invalid password");
            }
            None => {
                return ApiError::with_details(
                    StatusCode::UNAUTHORIZED,
                    ApiError::SHARE_PASSWORD_REQUIRED,
                    "Password required",
                    "true",
                );
            }
        }
    }

    let meta = match state.storage().head(&link.path).await {
        Ok(m) => m,
        Err(_) => return ApiError::not_found(ApiError::FILE_NOT_FOUND, "File not found"),
    };

    if meta.is_collection {
        return ApiError::bad_request(
            ApiError::INVALID_INPUT,
            "View share target must be a file, not a directory",
        );
    }

    let allow_download = get_allow_download(state, &token);

    let reader = match state.storage().get_stream(&link.path).await {
        Ok(r) => r,
        Err(_) => return ApiError::not_found(ApiError::FILE_NOT_FOUND, "File not found"),
    };

    state.share_store().increment_download(&token).await;

    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        "Content-Type",
        axum::http::HeaderValue::from_str(&meta.mime_type)
            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("application/octet-stream")),
    );
    headers.insert(
        "Content-Length",
        axum::http::HeaderValue::from_str(&meta.size.to_string())
            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("0")),
    );

    if allow_download {
        headers.insert(
            "Content-Disposition",
            axum::http::HeaderValue::from_str(&format!(
                "inline; filename=\"{}\"",
                link.path.rsplit('/').next().unwrap_or("preview")
            ))
            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("inline; filename=\"preview\"")),
        );
    } else {
        headers.insert("Content-Disposition", axum::http::HeaderValue::from_static("inline"));
        headers.insert(
            "Content-Security-Policy",
            axum::http::HeaderValue::from_static(
                "default-src 'none'; img-src 'self' data:; style-src 'self' 'unsafe-inline';",
            ),
        );
    }

    let stream = tokio_util::io::ReaderStream::new(reader);
    let body = axum::body::Body::from_stream(stream);
    (StatusCode::OK, headers, body).into_response()
}

/// `GET /s/:token/view` -- Serve a secure view share (preview only).
pub async fn serve_view_share(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    serve_view_share_impl(&state, token, params).await
}

// ---------------------------------------------------------------------------
// HTML Templates
// ---------------------------------------------------------------------------

pub(crate) fn serve_upload_dropzone(link: &ShareLink) -> Response {
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>File Drop</title>
<style>
  * {{ box-sizing: border-box; margin: 0; padding: 0; }}
  body {{ font-family: system-ui, sans-serif; display: flex; justify-content: center; align-items: center; min-height: 100vh; background: #f5f5f5; }}
  .container {{ background: #fff; padding: 2rem; border-radius: 8px; box-shadow: 0 2px 8px rgba(0,0,0,.1); max-width: 480px; width: 100%; }}
  h1 {{ font-size: 1.25rem; margin-bottom: .5rem; }}
  p {{ color: #666; font-size: .9rem; margin-bottom: 1.5rem; }}
  .dropzone {{ border: 2px dashed #ccc; border-radius: 6px; padding: 2rem; text-align: center; cursor: pointer; transition: border-color .2s; }}
  .dropzone.dragover {{ border-color: #2563eb; }}
  .dropzone p {{ margin: 0; color: #888; }}
  input[type=file] {{ display: none; }}
  .progress {{ margin-top: 1rem; height: 4px; background: #e5e7eb; border-radius: 2px; overflow: hidden; display: none; }}
  .progress-bar {{ height: 100%; background: #2563eb; width: 0; transition: width .2s; }}
  .result {{ margin-top: 1rem; font-size: .85rem; display: none; }}
  .result.ok {{ color: #16a34a; }}
  .result.err {{ color: #dc2626; }}
</style>
</head>
<body>
<div class="container">
  <h1>File Drop</h1>
  <p>Drop files here or click to browse. Files are uploaded to the shared folder.</p>
  <div class="dropzone" id="dropzone">
    <p>Drag &amp; drop or click to upload</p>
    <input type="file" id="fileInput" multiple>
  </div>
  <div class="progress" id="progress"><div class="progress-bar" id="bar"></div></div>
  <div class="result" id="result"></div>
</div>
<script>
const token = '{token}';
const dz = document.getElementById('dropzone');
const fi = document.getElementById('fileInput');
const prog = document.getElementById('progress');
const bar = document.getElementById('bar');
const res = document.getElementById('result');
dz.addEventListener('click', () => fi.click());
dz.addEventListener('dragover', e => {{ e.preventDefault(); dz.classList.add('dragover'); }});
dz.addEventListener('dragleave', () => dz.classList.remove('dragover'));
dz.addEventListener('drop', e => {{ e.preventDefault(); dz.classList.remove('dragover'); uploadFiles(e.dataTransfer.files); }});
fi.addEventListener('change', () => {{ uploadFiles(fi.files); fi.value=''; }});
async function uploadFiles(files) {{
  for (const f of files) {{
    const fd = new FormData();
    fd.append('file', f);
    prog.style.display='block'; bar.style.width='0'; res.style.display='none';
    try {{
      const xhr = new XMLHttpRequest();
      xhr.open('POST', '/s/' + token);
      xhr.upload.onprogress = e => {{ if(e.lengthComputable) bar.style.width=(e.loaded/e.total*100)+'%'; }};
      await new Promise((ok,no)=>{{ xhr.onload=ok; xhr.onerror=no; xhr.send(fd); }});
      if(xhr.status===201) {{ res.textContent='Uploaded: '+f.name+' ('+f.size+' bytes)'; res.className='result ok'; }}
      else {{ res.textContent='Error: '+xhr.statusText; res.className='result err'; }}
    }} catch(e) {{ res.textContent='Upload failed'; res.className='result err'; }}
  }}
}}
</script>
</body>
</html>"#,
        token = link.token,
    );

    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        "Content-Type",
        axum::http::HeaderValue::from_static("text/html; charset=utf-8"),
    );
    headers.insert("Content-Disposition", axum::http::HeaderValue::from_static("inline"));
    (StatusCode::OK, headers, html).into_response()
}

pub(crate) fn serve_preview_html(link: &ShareLink, meta: &common::metadata::FileMetadata) -> Response {
    let filename = link.path.rsplit('/').next().unwrap_or("preview");
    let escaped_filename = html_escape(filename);
    let content_type = &meta.mime_type;

    let viewer_script = if content_type.starts_with("image/") {
        format!(
            r#"<img src="/s/{token}/view" alt="{name}" style="max-width:100%;height:auto;display:block;margin:0 auto;" />"#,
            token = link.token,
            name = escaped_filename,
        )
    } else if content_type.starts_with("video/") {
        format!(
            r#"<video controls style="max-width:100%;height:auto;display:block;margin:0 auto;"><source src="/s/{token}/view" type="{ct}"></video>"#,
            token = link.token,
            ct = content_type,
        )
    } else if content_type.starts_with("audio/") {
        format!(
            r#"<audio controls style="display:block;margin:0 auto;"><source src="/s/{token}/view" type="{ct}"></audio>"#,
            token = link.token,
            ct = content_type,
        )
    } else if content_type == "application/pdf" {
        format!(
            r#"<embed src="/s/{token}/view" type="application/pdf" style="width:100%;height:90vh;border:none;" />"#,
            token = link.token,
        )
    } else {
        format!(
            r#"<iframe src="/s/{token}/view" style="width:100%;height:90vh;border:none;"></iframe>"#,
            token = link.token,
        )
    };

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Preview - {name}</title>
<style>
  * {{ box-sizing: border-box; margin: 0; padding: 0; }}
  body {{ font-family: system-ui, sans-serif; background: #fff; }}
  .header {{ display: flex; justify-content: space-between; align-items: center; padding: .5rem 1rem; border-bottom: 1px solid #e5e7eb; background: #fafafa; }}
  .header h1 {{ font-size: 1rem; font-weight: 500; }}
  .badge {{ font-size: .75rem; color: #6b7280; background: #f3f4f6; padding: .25rem .5rem; border-radius: 4px; }}
  .viewer {{ padding: 1rem; }}
</style>
</head>
<body>
<div class="header">
  <h1>{name}</h1>
  <span class="badge">Preview only</span>
</div>
<div class="viewer">{viewer}</div>
</body>
</html>"#,
        name = escaped_filename,
        viewer = viewer_script,
    );

    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        "Content-Type",
        axum::http::HeaderValue::from_static("text/html; charset=utf-8"),
    );
    headers.insert("Content-Disposition", axum::http::HeaderValue::from_static("inline"));
    headers.insert(
        "Content-Security-Policy",
        axum::http::HeaderValue::from_static(
            "default-src 'self'; img-src 'self' data: blob:; media-src 'self' blob:; style-src 'self' 'unsafe-inline'; frame-src 'self'; object-src 'self';",
        ),
    );
    (StatusCode::OK, headers, html).into_response()
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub(crate) fn get_share_type<S: ServerState>(state: &S, token: &str) -> ShareType {
    if let Some(db) = state.db() {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        if let Ok(st) = conn.query_row(
            "SELECT share_type FROM shares WHERE token = ?1",
            params![token],
            |row| row.get::<_, Option<String>>(0),
        ) {
            return st.as_deref().map(ShareType::from_str_opt).unwrap_or_default();
        }
    }
    ShareType::Download
}

pub(crate) fn get_allow_download<S: ServerState>(state: &S, token: &str) -> bool {
    if let Some(db) = state.db() {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        if let Ok(allowed) = conn.query_row(
            "SELECT allow_download FROM shares WHERE token = ?1",
            params![token],
            |row| row.get::<_, i32>(0),
        ) {
            return allowed != 0;
        }
    }
    true
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    use subtle::ConstantTimeEq;
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_share_type_parsing() {
        assert_eq!(ShareType::from_str_opt("download"), ShareType::Download);
        assert_eq!(ShareType::from_str_opt("upload"), ShareType::Upload);
        assert_eq!(ShareType::from_str_opt("view"), ShareType::View);
        assert_eq!(ShareType::from_str_opt("unknown"), ShareType::Download);
    }

    #[test]
    fn test_share_type_display() {
        assert_eq!(ShareType::Download.to_string(), "download");
        assert_eq!(ShareType::Upload.to_string(), "upload");
        assert_eq!(ShareType::View.to_string(), "view");
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("hello.txt"), "hello.txt");
        assert_eq!(sanitize_filename("my file.pdf"), "my_file.pdf");
        assert_eq!(sanitize_filename("../../../etc/passwd"), ".._.._.._etc_passwd");
    }

    #[test]
    fn test_sniff_mime_type() {
        assert_eq!(sniff_mime_type("photo.jpg"), "image/jpeg");
        assert_eq!(sniff_mime_type("doc.pdf"), "application/pdf");
        assert_eq!(sniff_mime_type("data.csv"), "application/octet-stream");
    }
}
