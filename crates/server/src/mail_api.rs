use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::api_error::ApiError;
use crate::db::DbHandle;
use crate::AppState;

/// IMAP/SMTP mail account configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailAccount {
    pub id: String,
    pub name: String,
    pub email_address: String,
    pub imap_host: String,
    pub imap_port: u16,
    pub imap_security: String,
    pub imap_username: String,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_security: String,
    pub smtp_username: String,
    pub created_at: String,
}

/// Request body for creating a new mail account.
#[derive(Debug, Deserialize)]
pub struct CreateMailAccountRequest {
    pub name: String,
    pub email_address: String,
    pub imap_host: String,
    pub imap_port: Option<u16>,
    pub imap_security: Option<String>,
    pub imap_username: String,
    pub imap_password: String,
    pub smtp_host: String,
    pub smtp_port: Option<u16>,
    pub smtp_security: Option<String>,
    pub smtp_username: String,
    pub smtp_password: String,
}

/// Request body for sending email.
#[derive(Debug, Deserialize)]
pub struct SendEmailRequest {
    pub to: Vec<String>,
    pub cc: Option<Vec<String>>,
    pub bcc: Option<Vec<String>>,
    pub subject: String,
    pub body_text: Option<String>,
    pub body_html: Option<String>,
    pub attachments: Option<Vec<EmailAttachment>>,
}

/// An email attachment.
#[derive(Debug, Deserialize)]
pub struct EmailAttachment {
    pub filename: String,
    pub content_type: String,
    /// Base64-encoded content.
    pub content: String,
}

/// IMAP folder representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailFolder {
    pub name: String,
    pub delimiter: Option<String>,
    pub flags: Vec<String>,
}

/// Email message header summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailMessage {
    pub uid: u32,
    pub subject: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub date: Option<String>,
    pub seen: bool,
    pub has_attachments: bool,
    pub size: u32,
}

/// Full email message with body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailMessageDetail {
    pub uid: u32,
    pub subject: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub cc: Option<String>,
    pub date: Option<String>,
    pub body_text: Option<String>,
    pub body_html: Option<String>,
    pub attachments: Vec<MailAttachmentInfo>,
}

/// Attachment metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailAttachmentInfo {
    pub part: String,
    pub filename: String,
    pub content_type: String,
    pub size: u32,
}

/// Mail account store backed by SQLite.
pub struct MailAccountStore {
    #[allow(dead_code)]
    db: DbHandle,
}

impl MailAccountStore {
    pub fn new(db: DbHandle) -> Self {
        Self { db }
    }

    fn encrypt_password(password: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        let hash = hasher.finalize();
        format!(
            "sha256:{}",
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, hash)
        )
    }

    fn list_accounts_from_db(
        conn: &rusqlite::Connection,
    ) -> Result<Vec<MailAccount>, rusqlite::Error> {
        let mut stmt = conn.prepare(
            "SELECT id, name, email_address, imap_host, imap_port, imap_security, imap_username, smtp_host, smtp_port, smtp_security, smtp_username, created_at FROM mail_accounts ORDER BY created_at",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(MailAccount {
                id: row.get(0)?,
                name: row.get(1)?,
                email_address: row.get(2)?,
                imap_host: row.get(3)?,
                imap_port: row.get::<_, i32>(4)? as u16,
                imap_security: row.get(5)?,
                imap_username: row.get(6)?,
                smtp_host: row.get(7)?,
                smtp_port: row.get::<_, i32>(8)? as u16,
                smtp_security: row.get(9)?,
                smtp_username: row.get(10)?,
                created_at: row.get(11)?,
            })
        })?;
        rows.collect()
    }

    fn get_account_from_db(
        conn: &rusqlite::Connection,
        id: &str,
    ) -> Result<MailAccount, rusqlite::Error> {
        conn.query_row(
            "SELECT id, name, email_address, imap_host, imap_port, imap_security, imap_username, smtp_host, smtp_port, smtp_security, smtp_username, created_at FROM mail_accounts WHERE id = ?1",
            params![id],
            |row| {
                Ok(MailAccount {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    email_address: row.get(2)?,
                    imap_host: row.get(3)?,
                    imap_port: row.get::<_, i32>(4)? as u16,
                    imap_security: row.get(5)?,
                    imap_username: row.get(6)?,
                    smtp_host: row.get(7)?,
                    smtp_port: row.get::<_, i32>(8)? as u16,
                    smtp_security: row.get(9)?,
                    smtp_username: row.get(10)?,
                    created_at: row.get(11)?,
                })
            },
        )
    }
}

// ---------------------------------------------------------------------------
// IMAP helpers
// ---------------------------------------------------------------------------

fn imap_connect(
    host: &str,
    port: u16,
    security: &str,
    username: &str,
    password: &str,
) -> Result<imap::Session<native_tls::TlsStream<std::net::TcpStream>>, String> {
    let tls = native_tls::TlsConnector::builder()
        .build()
        .map_err(|e| format!("TLS error: {e}"))?;

    let client = if security == "ssl" || port == 993 {
        imap::connect((host, port), host, &tls)
    } else {
        imap::connect_starttls((host, port), host, &tls)
    }
    .map_err(|e| format!("IMAP connect error: {e}"))?;

    let session = client
        .login(username, password)
        .map_err(|e| format!("IMAP login error: {}", e.0))?;

    Ok(session)
}

fn parse_fetch_headers(
    header_bytes: &[u8],
) -> (Option<String>, Option<String>, Option<String>, Option<String>) {
    let mut subject = None;
    let mut from = None;
    let mut to = None;
    let mut date = None;

    if let Ok(header_str) = std::str::from_utf8(header_bytes) {
        for line in header_str.lines() {
            let lower = line.to_lowercase();
            if lower.starts_with("subject:") {
                subject = Some(line[8..].trim().to_string());
            } else if lower.starts_with("from:") {
                from = Some(line[5..].trim().to_string());
            } else if lower.starts_with("to:") {
                to = Some(line[3..].trim().to_string());
            } else if lower.starts_with("date:") {
                date = Some(line[5..].trim().to_string());
            }
        }
    }

    (subject, from, to, date)
}

// ---------------------------------------------------------------------------
// Route handlers
// ---------------------------------------------------------------------------

/// GET /mail/accounts — list configured mail accounts.
pub async fn list_accounts(State(state): State<AppState>) -> Response {
    let Some(ref db) = state.db else {
        return ApiError::internal(ApiError::INTERNAL_ERROR, "Database not configured");
    };
    let conn = db.lock().unwrap_or_else(|e| e.into_inner());
    match MailAccountStore::list_accounts_from_db(&conn) {
        Ok(accounts) => (
            StatusCode::OK,
            Json(serde_json::json!({ "accounts": accounts })),
        )
            .into_response(),
        Err(e) => {
            warn!("Failed to list mail accounts: {}", e);
            ApiError::internal(ApiError::INTERNAL_ERROR, "Failed to list mail accounts")
        }
    }
}

/// POST /mail/accounts — add a new mail account.
pub async fn create_account(
    State(state): State<AppState>,
    Json(req): Json<CreateMailAccountRequest>,
) -> Response {
    let Some(ref db) = state.db else {
        return ApiError::internal(ApiError::INTERNAL_ERROR, "Database not configured");
    };

    if req.name.is_empty() || req.email_address.is_empty() {
        return ApiError::bad_request(
            ApiError::INVALID_INPUT,
            "Name and email address are required",
        );
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let imap_port = req.imap_port.unwrap_or(993);
    let imap_security = req
        .imap_security
        .unwrap_or_else(|| "ssl".to_string());
    let smtp_port = req.smtp_port.unwrap_or(587);
    let smtp_security = req
        .smtp_security
        .unwrap_or_else(|| "starttls".to_string());
    let imap_password_enc = MailAccountStore::encrypt_password(&req.imap_password);
    let smtp_password_enc = MailAccountStore::encrypt_password(&req.smtp_password);

    let conn = db.lock().unwrap_or_else(|e| e.into_inner());
    match conn.execute(
        "INSERT INTO mail_accounts (id, name, email_address, imap_host, imap_port, imap_security, imap_username, imap_password, smtp_host, smtp_port, smtp_security, smtp_username, smtp_password, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            id, req.name, req.email_address, req.imap_host, imap_port as i32,
            imap_security, req.imap_username, imap_password_enc,
            req.smtp_host, smtp_port as i32, smtp_security, req.smtp_username,
            smtp_password_enc, now,
        ],
    ) {
        Ok(_) => {
            let account = MailAccount {
                id,
                name: req.name,
                email_address: req.email_address,
                imap_host: req.imap_host,
                imap_port,
                imap_security,
                imap_username: req.imap_username,
                smtp_host: req.smtp_host,
                smtp_port,
                smtp_security,
                smtp_username: req.smtp_username,
                created_at: now,
            };
            (StatusCode::CREATED, Json(serde_json::json!(account))).into_response()
        }
        Err(e) => {
            warn!("Failed to create mail account: {}", e);
            ApiError::internal(ApiError::INTERNAL_ERROR, "Failed to create mail account")
        }
    }
}

/// DELETE /mail/accounts/{id} — remove a mail account.
pub async fn delete_account(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let Some(ref db) = state.db else {
        return ApiError::internal(ApiError::INTERNAL_ERROR, "Database not configured");
    };
    let conn = db.lock().unwrap_or_else(|e| e.into_inner());
    match conn.execute(
        "DELETE FROM mail_accounts WHERE id = ?1",
        params![id],
    ) {
        Ok(0) => ApiError::not_found(ApiError::NOT_FOUND, "Mail account not found"),
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            warn!("Failed to delete mail account: {}", e);
            ApiError::internal(ApiError::INTERNAL_ERROR, "Failed to delete mail account")
        }
    }
}

/// GET /mail/accounts/{id}/folders — list IMAP folders.
pub async fn mail_folders(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Response {
    let Some(ref db) = state.db else {
        return ApiError::internal(ApiError::INTERNAL_ERROR, "Database not configured");
    };
    let conn = db.lock().unwrap_or_else(|e| e.into_inner());
    let account = match MailAccountStore::get_account_from_db(&conn, &id) {
        Ok(a) => a,
        Err(_) => return ApiError::not_found(ApiError::NOT_FOUND, "Mail account not found"),
    };
    drop(conn);

    // Connect without password — in production, credentials would come from a secure store
    let mut session = match imap_connect(
        &account.imap_host,
        account.imap_port,
        &account.imap_security,
        &account.imap_username,
        "",
    ) {
        Ok(s) => s,
        Err(e) => return ApiError::bad_gateway(ApiError::BAD_GATEWAY, e),
    };

    let mailboxes = match session.list(Some(""), Some("*")) {
        Ok(m) => m,
        Err(e) => {
            return ApiError::bad_gateway(
                ApiError::BAD_GATEWAY,
                format!("IMAP list error: {e}"),
            );
        }
    };

    let mut folders = Vec::new();
    for mailbox in &mailboxes {
        folders.push(MailFolder {
            name: mailbox.name().to_string(),
            delimiter: mailbox.delimiter().map(|d| d.to_string()),
            flags: mailbox.attributes().iter().map(|a| format!("{a:?}")).collect(),
        });
    }

    let _ = session.logout();

    (
        StatusCode::OK,
        Json(serde_json::json!({ "folders": folders })),
    )
        .into_response()
}

/// GET /mail/accounts/{id}/folders/{folder}/messages — list messages.
pub async fn mail_messages(
    State(state): State<AppState>,
    Path((id, folder)): Path<(String, String)>,
    axum::extract::Query(params): axum::extract::Query<
        std::collections::HashMap<String, String>,
    >,
) -> Response {
    let limit: u32 = params
        .get("limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(50);
    let offset: u32 = params
        .get("offset")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    let Some(ref db) = state.db else {
        return ApiError::internal(ApiError::INTERNAL_ERROR, "Database not configured");
    };
    let conn = db.lock().unwrap_or_else(|e| e.into_inner());
    let account = match MailAccountStore::get_account_from_db(&conn, &id) {
        Ok(a) => a,
        Err(_) => return ApiError::not_found(ApiError::NOT_FOUND, "Mail account not found"),
    };
    drop(conn);

    let mut session = match imap_connect(
        &account.imap_host,
        account.imap_port,
        &account.imap_security,
        &account.imap_username,
        "",
    ) {
        Ok(s) => s,
        Err(e) => return ApiError::bad_gateway(ApiError::BAD_GATEWAY, e),
    };

    if let Err(e) = session.select(&folder) {
        return ApiError::bad_gateway(
            ApiError::BAD_GATEWAY,
            format!("IMAP select error: {e}"),
        );
    }

    let all_messages = match session.uid_search("ALL") {
        Ok(m) => m,
        Err(e) => {
            return ApiError::bad_gateway(
                ApiError::BAD_GATEWAY,
                format!("IMAP uid_search error: {e}"),
            );
        }
    };

    let mut uids: Vec<u32> = all_messages.into_iter().collect();
    uids.sort_unstable();
    uids.reverse();

    let total = uids.len() as u32;
    let start = offset.min(total);
    let end = (start + limit).min(total);
    let page_uids: Vec<u32> = uids[start as usize..end as usize].to_vec();

    if page_uids.is_empty() {
        let _ = session.logout();
        return (
            StatusCode::OK,
            Json(serde_json::json!({ "messages": Vec::<MailMessage>::new(), "total": 0 })),
        )
            .into_response();
    }

    let uid_range = page_uids
        .iter()
        .map(|u| u.to_string())
        .collect::<Vec<_>>()
        .join(",");

    let fetch_items = match session.uid_fetch(&uid_range, "(UID FLAGS RFC822.SIZE RFC822.HEADER)") {
        Ok(f) => f,
        Err(e) => {
            return ApiError::bad_gateway(
                ApiError::BAD_GATEWAY,
                format!("IMAP uid_fetch error: {e}"),
            );
        }
    };

    let mut messages = Vec::new();
    for fetch in fetch_items.iter() {
        let uid = fetch.uid.unwrap_or(0);
        let flags = fetch.flags();
        let seen = flags.iter().any(|f| matches!(f, imap::types::Flag::Seen));
        let size = fetch.size.unwrap_or(0);

        let (subject, from, to, date) = match fetch.header() {
            Some(h) => parse_fetch_headers(h),
            None => (None, None, None, None),
        };

        messages.push(MailMessage {
            uid,
            subject,
            from,
            to,
            date,
            seen,
            has_attachments: false,
            size,
        });
    }

    let _ = session.logout();

    (
        StatusCode::OK,
        Json(serde_json::json!({ "messages": messages, "total": total })),
    )
        .into_response()
}

/// GET /mail/accounts/{id}/folders/{folder}/messages/{uid} — get a message.
pub async fn mail_message_detail(
    State(state): State<AppState>,
    Path((id, folder, uid)): Path<(String, String, u32)>,
) -> Response {
    let Some(ref db) = state.db else {
        return ApiError::internal(ApiError::INTERNAL_ERROR, "Database not configured");
    };
    let conn = db.lock().unwrap_or_else(|e| e.into_inner());
    let account = match MailAccountStore::get_account_from_db(&conn, &id) {
        Ok(a) => a,
        Err(_) => return ApiError::not_found(ApiError::NOT_FOUND, "Mail account not found"),
    };
    drop(conn);

    let mut session = match imap_connect(
        &account.imap_host,
        account.imap_port,
        &account.imap_security,
        &account.imap_username,
        "",
    ) {
        Ok(s) => s,
        Err(e) => return ApiError::bad_gateway(ApiError::BAD_GATEWAY, e),
    };

    if let Err(e) = session.select(&folder) {
        return ApiError::bad_gateway(
            ApiError::BAD_GATEWAY,
            format!("IMAP select error: {e}"),
        );
    }

    let uid_str = uid.to_string();
    let fetch_items = match session.uid_fetch(&uid_str, "(UID FLAGS RFC822)") {
        Ok(f) => f,
        Err(e) => {
            return ApiError::bad_gateway(
                ApiError::BAD_GATEWAY,
                format!("IMAP uid_fetch error: {e}"),
            );
        }
    };

    let result = match fetch_items.iter().next() {
        Some(f) => {
            let body = f.body().unwrap_or(b"");
            let mut subject = None;
            let mut from = None;
            let mut to = None;
            let mut cc = None;
            let mut date = None;
            let mut body_text = None;
            let mut body_html = None;

            if let Ok(body_str) = std::str::from_utf8(body) {
                if let Some(header_end) = body_str.find("\r\n\r\n") {
                    let headers = &body_str[..header_end];
                    for line in headers.lines() {
                        let lower = line.to_lowercase();
                        if lower.starts_with("subject:") {
                            subject = Some(line[8..].trim().to_string());
                        } else if lower.starts_with("from:") {
                            from = Some(line[5..].trim().to_string());
                        } else if lower.starts_with("to:") {
                            to = Some(line[3..].trim().to_string());
                        } else if lower.starts_with("cc:") {
                            cc = Some(line[3..].trim().to_string());
                        } else if lower.starts_with("date:") {
                            date = Some(line[5..].trim().to_string());
                        }
                    }

                    let content = &body_str[header_end + 4..];
                    if content.contains("text/html") {
                        body_html = Some(content.to_string());
                    } else {
                        body_text = Some(content.to_string());
                    }
                }
            }

            MailMessageDetail {
                uid,
                subject,
                from,
                to,
                cc,
                date,
                body_text,
                body_html,
                attachments: Vec::new(),
            }
        }
        None => {
            return ApiError::not_found(ApiError::NOT_FOUND, "Message not found");
        }
    };

    let _ = session.logout();
    (StatusCode::OK, Json(serde_json::json!(result))).into_response()
}

/// POST /mail/accounts/{id}/send — send an email via SMTP.
pub async fn send_email(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<SendEmailRequest>,
) -> Response {
    let Some(ref db) = state.db else {
        return ApiError::internal(ApiError::INTERNAL_ERROR, "Database not configured");
    };
    let account = {
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        match MailAccountStore::get_account_from_db(&conn, &id) {
            Ok(a) => a,
            Err(_) => return ApiError::not_found(ApiError::NOT_FOUND, "Mail account not found"),
        }
    };

    if req.to.is_empty() {
        return ApiError::bad_request(
            ApiError::INVALID_INPUT,
            "At least one recipient is required",
        );
    }

    let tls_params =
        lettre::transport::smtp::client::TlsParameters::builder(account.smtp_host.clone())
            .build()
            .map_err(|e| format!("TLS config error: {e}"))
            .unwrap();

    let tls_mode = match account.smtp_security.as_str() {
        "ssl" => lettre::transport::smtp::client::Tls::Wrapper(tls_params),
        _ => lettre::transport::smtp::client::Tls::Required(tls_params),
    };

    let transport =
        lettre::AsyncSmtpTransport::<lettre::Tokio1Executor>::builder_dangerous(&account.smtp_host)
            .port(account.smtp_port)
            .tls(tls_mode)
            .build();

    let from_addr: lettre::message::Mailbox = match format!("{} <{}>", account.name, account.email_address).parse() {
        Ok(a) => a,
        Err(_) => {
            return ApiError::bad_request(ApiError::INVALID_INPUT, "Invalid from address");
        }
    };

    let mut email_builder = lettre::Message::builder()
        .from(from_addr)
        .subject(&req.subject);

    for to_addr in &req.to {
        if let Ok(addr) = to_addr.parse() {
            email_builder = email_builder.to(addr);
        }
    }
    if let Some(ref cc_list) = req.cc {
        for cc_addr in cc_list {
            if let Ok(addr) = cc_addr.parse() {
                email_builder = email_builder.cc(addr);
            }
        }
    }
    if let Some(ref bcc_list) = req.bcc {
        for bcc_addr in bcc_list {
            if let Ok(addr) = bcc_addr.parse() {
                email_builder = email_builder.bcc(addr);
            }
        }
    }

    let has_attachments = req
        .attachments
        .as_ref()
        .is_some_and(|a| !a.is_empty());

    let email = if has_attachments {
        let mut multipart = lettre::message::MultiPart::mixed().singlepart(
            lettre::message::SinglePart::builder()
                .header(lettre::message::header::ContentType::TEXT_PLAIN)
                .body(req.body_text.clone().unwrap_or_default()),
        );

        if let Some(ref attachments) = req.attachments {
            for att in attachments {
                if let Ok(data) =
                    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &att.content)
                {
                    let ct = lettre::message::header::ContentType::parse(&att.content_type)
                        .unwrap_or(lettre::message::header::ContentType::TEXT_PLAIN);
                    let attachment = lettre::message::Attachment::new(att.filename.clone())
                        .body(data, ct);
                    multipart = multipart.singlepart(attachment);
                }
            }
        }

        email_builder.multipart(multipart)
    } else if let Some(ref html) = req.body_html {
        email_builder
            .multipart(lettre::message::MultiPart::alternative_plain_html(
                req.body_text.clone().unwrap_or_default(),
                html.clone(),
            ))
    } else {
        email_builder.body(req.body_text.clone().unwrap_or_default())
    };

    match email {
        Ok(email) => {
            use lettre::AsyncTransport;
            match transport.send(email).await {
                Ok(_) => (
                    StatusCode::OK,
                    Json(serde_json::json!({ "status": "sent" })),
                )
                    .into_response(),
                Err(e) => {
                    warn!("SMTP send failed: {}", e);
                    ApiError::bad_gateway(ApiError::BAD_GATEWAY, format!("SMTP error: {e}"))
                }
            }
        }
        Err(e) => {
            ApiError::bad_request(ApiError::INVALID_INPUT, format!("Email build error: {e}"))
        }
    }
}

/// POST /mail/accounts/{id}/folders/{folder}/messages/{uid}/attachments/{part}/download
pub async fn download_attachment(
    State(_state): State<AppState>,
    Path((_id, _folder, _uid, _part)): Path<(String, String, u32, String)>,
) -> Response {
    ApiError::not_implemented(
        "NOT_IMPLEMENTED",
        "Attachment download via IMAP is not yet implemented",
    )
}
