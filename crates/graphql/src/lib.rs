//! GraphQL API schema and handlers for Ferro server.
//!
//! Defines the [`AppSchema`], query/mutation root types, and axum route
//! handlers. Data access is abstracted via [`GraphQLContext`] so the crate
//! has zero dependency on the server's `AppState`.

use async_graphql::{Context, EmptySubscription, Object, Schema};
use axum::response::{Html, IntoResponse, Response};

// ---------------------------------------------------------------------------
// Context — concrete struct with boxed async functions for data access
// ---------------------------------------------------------------------------

type BoxFuture<T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send>>;

/// Minimal data-access surface required by GraphQL resolvers.
#[allow(clippy::type_complexity)]
pub struct GraphQLContext {
    /// List files under a prefix.
    pub list_files: Box<
        dyn Fn(&str) -> BoxFuture<Result<Vec<common::metadata::FileMetadata>, String>>
            + Send
            + Sync,
    >,
    /// Get metadata for a single file.
    pub head_file: Box<
        dyn Fn(&str) -> BoxFuture<Result<common::metadata::FileMetadata, String>> + Send + Sync,
    >,
    /// Create a directory collection.
    pub create_collection: Box<
        dyn Fn(&str, &str) -> BoxFuture<Result<common::metadata::FileMetadata, String>>
            + Send
            + Sync,
    >,
    /// Delete a file or collection.
    pub delete_file: Box<dyn Fn(&str) -> BoxFuture<Result<(), String>> + Send + Sync>,
    /// List all share links.
    pub list_shares: Box<dyn Fn() -> BoxFuture<Vec<ShareEntry>> + Send + Sync>,
    /// List recent audit entries.
    pub recent_audit: Box<dyn Fn(usize, usize) -> BoxFuture<Vec<AuditEntry>> + Send + Sync>,
    /// Authenticated user info extracted from request context.
    /// `None` when no auth middleware is active (anonymous).
    pub current_user: Option<CurrentUser>,
}

/// Authenticated user identity for GraphQL resolvers.
#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub username: String,
    pub role: String,
}

// ---------------------------------------------------------------------------
// Data types consumed by resolvers
// ---------------------------------------------------------------------------

/// Share link data consumed by GraphQL resolvers.
#[derive(Debug, Clone)]
pub struct ShareEntry {
    pub token: String,
    pub path: String,
    pub expires_at: String,
    pub password_protected: bool,
    pub max_downloads: Option<u32>,
    pub download_count: u32,
    pub created_by: String,
}

/// Audit entry data consumed by GraphQL resolvers.
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub method: String,
    pub path: String,
    pub user: String,
    pub status: u16,
    pub timestamp: String,
}

fn get_ctx<'a>(ctx: &'a Context<'a>) -> async_graphql::Result<&'a GraphQLContext> {
    ctx.data::<GraphQLContext>()
        .map_err(|_| async_graphql::Error::new("GraphQLContext not configured"))
}

// ---------------------------------------------------------------------------
// Schema types
// ---------------------------------------------------------------------------

pub struct Query;

#[Object]
impl Query {
    async fn files(
        &self,
        ctx: &Context<'_>,
        path: Option<String>,
        limit: Option<i32>,
    ) -> async_graphql::Result<Vec<FileItem>> {
        let data = get_ctx(ctx)?;
        let prefix = path.unwrap_or_else(|| "/".to_string());
        let files = (data.list_files)(&prefix)
            .await
            .map_err(async_graphql::Error::new)?;
        let limit = limit.unwrap_or(100).min(1000) as usize;
        Ok(files.into_iter().take(limit).map(FileItem::from).collect())
    }

    async fn file(
        &self,
        ctx: &Context<'_>,
        path: String,
    ) -> async_graphql::Result<Option<FileItem>> {
        let data = get_ctx(ctx)?;
        match (data.head_file)(&path).await {
            Ok(meta) => Ok(Some(FileItem::from(meta))),
            Err(_) => Ok(None),
        }
    }

    async fn shares(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<ShareItem>> {
        let data = get_ctx(ctx)?;
        let links = (data.list_shares)().await;
        Ok(links.into_iter().map(ShareItem::from).collect())
    }

    async fn me(&self, ctx: &Context<'_>) -> async_graphql::Result<UserItem> {
        let data = get_ctx(ctx)?;
        match &data.current_user {
            Some(user) => Ok(UserItem {
                username: user.username.clone(),
                role: user.role.clone(),
            }),
            None => Ok(UserItem {
                username: "anonymous".to_string(),
                role: "viewer".to_string(),
            }),
        }
    }

    async fn health(&self, _ctx: &Context<'_>) -> async_graphql::Result<HealthItem> {
        Ok(HealthItem {
            status: "ok".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }

    async fn audit_log(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> async_graphql::Result<Vec<AuditItemItem>> {
        let data = get_ctx(ctx)?;
        let limit = limit.unwrap_or(50) as usize;
        let offset = offset.unwrap_or(0) as usize;
        let entries = (data.recent_audit)(limit, offset).await;
        Ok(entries.into_iter().map(AuditItemItem::from).collect())
    }
}

pub struct Mutation;

#[Object]
impl Mutation {
    async fn create_folder(
        &self,
        ctx: &Context<'_>,
        path: String,
    ) -> async_graphql::Result<FileItem> {
        let data = get_ctx(ctx)?;
        let meta = (data.create_collection)(&path, "admin")
            .await
            .map_err(async_graphql::Error::new)?;
        Ok(FileItem::from(meta))
    }

    async fn delete_file(&self, ctx: &Context<'_>, path: String) -> async_graphql::Result<bool> {
        let data = get_ctx(ctx)?;
        (data.delete_file)(&path)
            .await
            .map_err(async_graphql::Error::new)?;
        Ok(true)
    }
}

pub type AppSchema = Schema<Query, Mutation, EmptySubscription>;

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

#[derive(async_graphql::SimpleObject)]
pub struct FileItem {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub is_collection: bool,
    pub mime_type: String,
    pub modified: String,
    pub owner: String,
}

impl From<common::metadata::FileMetadata> for FileItem {
    fn from(m: common::metadata::FileMetadata) -> Self {
        Self {
            path: m.path.clone(),
            name: m.path.rsplit('/').next().unwrap_or_default().to_string(),
            size: m.size,
            is_collection: m.is_collection,
            mime_type: m.mime_type,
            modified: m.modified_at.to_string(),
            owner: m.owner,
        }
    }
}

#[derive(async_graphql::SimpleObject)]
pub struct ShareItem {
    pub token: String,
    pub path: String,
    pub expires_at: String,
    pub password_protected: bool,
    pub max_downloads: Option<u32>,
    pub download_count: u32,
    pub created_by: String,
}

impl From<ShareEntry> for ShareItem {
    fn from(e: ShareEntry) -> Self {
        Self {
            token: e.token,
            path: e.path,
            expires_at: e.expires_at,
            password_protected: e.password_protected,
            max_downloads: e.max_downloads,
            download_count: e.download_count,
            created_by: e.created_by,
        }
    }
}

#[derive(async_graphql::SimpleObject)]
pub struct UserItem {
    pub username: String,
    pub role: String,
}

#[derive(async_graphql::SimpleObject)]
pub struct HealthItem {
    pub status: String,
    pub version: String,
}

/// GraphQL audit item (renamed to avoid clash with internal `AuditEntry`).
#[derive(async_graphql::SimpleObject)]
pub struct AuditItemItem {
    pub method: String,
    pub path: String,
    pub user: String,
    pub status: u16,
    pub timestamp: String,
}

impl From<AuditEntry> for AuditItemItem {
    fn from(e: AuditEntry) -> Self {
        Self {
            method: e.method,
            path: e.path,
            user: e.user,
            status: e.status,
            timestamp: e.timestamp,
        }
    }
}

// ---------------------------------------------------------------------------
// Schema builder and handlers
// ---------------------------------------------------------------------------

/// Build the GraphQL schema with the given context.
pub fn build_schema(ctx: GraphQLContext) -> AppSchema {
    Schema::build(Query, Mutation, EmptySubscription)
        .data(ctx)
        .finish()
}

/// POST /graphql — execute a GraphQL request.
pub async fn graphql_handler(
    axum::Extension(schema): axum::Extension<AppSchema>,
    axum::Json(req): axum::Json<async_graphql::Request>,
) -> Response {
    let res = schema.execute(req).await;
    let status = if res.is_ok() {
        axum::http::StatusCode::OK
    } else {
        axum::http::StatusCode::BAD_REQUEST
    };
    (status, axum::Json(res)).into_response()
}

/// GET /graphql — serve the GraphQL Playground UI.
pub async fn graphql_playground() -> Html<String> {
    Html(async_graphql::http::playground_source(
        async_graphql::http::GraphQLPlaygroundConfig::new("/api/graphql"),
    ))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ctx() -> GraphQLContext {
        use chrono::Utc;
        use common::metadata::ContentHash;

        let now = Utc::now();
        let h = ContentHash::new(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
        )
        .expect("valid hardcoded hash");
        // Clone for each closure consumer to avoid move conflicts.
        let h_list = h.clone();
        let h_head = h.clone();
        let h_create = h;

        GraphQLContext {
            list_files: Box::new({
                move |_prefix| {
                    let hash = h_list.clone();
                    let now = now;
                    Box::pin(async move {
                        Ok(vec![
                            common::metadata::FileMetadata {
                                path: "/test.txt".into(),
                                content_hash: hash.clone(),
                                size: 42,
                                mime_type: "text/plain".into(),
                                is_collection: false,
                                created_at: now,
                                modified_at: now,
                                owner: "admin".into(),
                                etag: "abc123".into(),
                            },
                            common::metadata::FileMetadata {
                                path: "/docs/".into(),
                                content_hash: hash,
                                size: 0,
                                mime_type: "inode/directory".into(),
                                is_collection: true,
                                created_at: now,
                                modified_at: now,
                                owner: "admin".into(),
                                etag: "def456".into(),
                            },
                        ])
                    })
                }
            }),
            head_file: Box::new({
                let hash = h_head;
                move |path| {
                    let hash = hash.clone();
                    let now = now;
                    let p = path.to_string();
                    Box::pin(async move {
                        if p == "/test.txt" {
                            Ok(common::metadata::FileMetadata {
                                path: p,
                                content_hash: hash,
                                size: 42,
                                mime_type: "text/plain".into(),
                                is_collection: false,
                                created_at: now,
                                modified_at: now,
                                owner: "admin".into(),
                                etag: "abc123".into(),
                            })
                        } else {
                            Err("not found".into())
                        }
                    })
                }
            }),
            create_collection: Box::new({
                let hash = h_create;
                move |path, _owner| {
                    let hash = hash.clone();
                    let now = now;
                    let pp = path.to_string();
                    Box::pin(async move {
                        Ok(common::metadata::FileMetadata {
                            path: pp,
                            content_hash: hash,
                            size: 0,
                            mime_type: "inode/directory".into(),
                            is_collection: true,
                            created_at: now,
                            modified_at: now,
                            owner: "admin".into(),
                            etag: "ghi789".into(),
                        })
                    })
                }
            }),
            delete_file: Box::new(|path| {
                let p = path.to_string();
                Box::pin(async move {
                    if p.is_empty() {
                        Err("invalid path".into())
                    } else {
                        Ok(())
                    }
                })
            }),
            list_shares: Box::new(|| {
                Box::pin(async {
                    vec![ShareEntry {
                        token: "abc123".into(),
                        path: "/shared.txt".into(),
                        expires_at: "2026-12-31".into(),
                        password_protected: true,
                        max_downloads: Some(10),
                        download_count: 3,
                        created_by: "admin".into(),
                    }]
                })
            }),
            recent_audit: Box::new(|limit, _offset| {
                Box::pin(async move {
                    (0..limit)
                        .map(|i| AuditEntry {
                            method: "GET".into(),
                            path: format!("/file{}.txt", i),
                            user: "admin".into(),
                            status: 200,
                            timestamp: "2026-05-12T00:00:00Z".into(),
                        })
                        .collect()
                })
            }),
            current_user: None,
        }
    }

    fn schema() -> AppSchema {
        build_schema(test_ctx())
    }

    // -- Schema construction -------------------------------------------------

    #[test]
    fn schema_builds_without_error() {
        let _schema = schema();
    }

    // -- Query: health -------------------------------------------------------

    #[tokio::test]
    async fn query_health_returns_ok() {
        let s = schema();
        let res = s.execute("{ health { status version } }").await;
        assert!(res.is_ok(), "health query failed: {:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["health"]["status"], "ok");
        assert!(!data["health"]["version"].as_str().unwrap().is_empty());
    }

    // -- Query: files --------------------------------------------------------

    #[tokio::test]
    async fn query_files_returns_list() {
        let s = schema();
        let res = s
            .execute("{ files { path size isCollection mimeType } }")
            .await;
        assert!(res.is_ok(), "files query failed: {:?}", res.errors);
        let data = res.data.into_json().unwrap();
        let files = data["files"].as_array().unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0]["path"], "/test.txt");
        assert_eq!(files[0]["size"], 42);
        assert!(!files[0]["isCollection"].as_bool().unwrap());
        assert_eq!(files[1]["path"], "/docs/");
        assert!(files[1]["isCollection"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn query_files_with_limit_respects_bound() {
        let s = schema();
        let res = s.execute("{ files(limit: 1) { path } }").await;
        assert!(res.is_ok());
        let data = res.data.into_json().unwrap();
        assert_eq!(data["files"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn query_files_with_explicit_path() {
        let s = schema();
        let res = s.execute(r#"{ files(path: "/test.txt") { path } }"#).await;
        assert!(res.is_ok());
        let data = res.data.into_json().unwrap();
        assert!(!data["files"].as_array().unwrap().is_empty());
    }

    // -- Query: file (single) ------------------------------------------------

    #[tokio::test]
    async fn query_single_file_exists() {
        let s = schema();
        let res = s
            .execute(r#"{ file(path: "/test.txt") { path size mimeType } }"#)
            .await;
        assert!(res.is_ok());
        let data = res.data.into_json().unwrap();
        assert_eq!(data["file"]["path"], "/test.txt");
        assert_eq!(data["file"]["size"], 42);
    }

    #[tokio::test]
    async fn query_single_file_not_found_returns_null() {
        let s = schema();
        let res = s
            .execute(r#"{ file(path: "/nonexistent") { path } }"#)
            .await;
        assert!(res.is_ok());
        let data = res.data.into_json().unwrap();
        assert!(data["file"].is_null());
    }

    // -- Query: shares -------------------------------------------------------

    #[tokio::test]
    async fn query_shares_returns_list() {
        let s = schema();
        let res = s
            .execute("{ shares { token path passwordProtected downloadCount } }")
            .await;
        assert!(res.is_ok());
        let data = res.data.into_json().unwrap();
        let shares = data["shares"].as_array().unwrap();
        assert_eq!(shares.len(), 1);
        assert_eq!(shares[0]["token"], "abc123");
        assert_eq!(shares[0]["path"], "/shared.txt");
        assert!(shares[0]["passwordProtected"].as_bool().unwrap());
        assert_eq!(shares[0]["downloadCount"], 3);
    }

    // -- Query: me -----------------------------------------------------------

    #[tokio::test]
    async fn query_me_returns_user() {
        let s = schema();
        let res = s.execute("{ me { username role } }").await;
        assert!(res.is_ok());
        let data = res.data.into_json().unwrap();
        assert_eq!(data["me"]["username"], "anonymous");
        assert_eq!(data["me"]["role"], "viewer");
    }

    // -- Query: audit_log ----------------------------------------------------

    #[tokio::test]
    async fn query_audit_log_returns_entries() {
        let s = schema();
        let res = s
            .execute("{ auditLog(limit: 3) { method path status } }")
            .await;
        assert!(res.is_ok());
        let data = res.data.into_json().unwrap();
        let entries = data["auditLog"].as_array().unwrap();
        assert_eq!(entries.len(), 3);
    }

    #[tokio::test]
    async fn query_audit_log_default_limit() {
        let s = schema();
        let res = s.execute("{ auditLog { method } }").await;
        assert!(res.is_ok());
        let data = res.data.into_json().unwrap();
        assert_eq!(data["auditLog"].as_array().unwrap().len(), 50);
    }

    // -- Mutation: create_folder ---------------------------------------------

    #[tokio::test]
    async fn mutation_create_folder() {
        let s = schema();
        let res = s
            .execute(r#"mutation { createFolder(path: "/new-folder/") { path isCollection } }"#)
            .await;
        assert!(res.is_ok(), "mutation failed: {:?}", res.errors);
        let data = res.data.into_json().unwrap();
        assert_eq!(data["createFolder"]["path"], "/new-folder/");
        assert!(data["createFolder"]["isCollection"].as_bool().unwrap());
    }

    // -- Mutation: delete_file -----------------------------------------------

    #[tokio::test]
    async fn mutation_delete_file() {
        let s = schema();
        let res = s
            .execute(r#"mutation { deleteFile(path: "/to-delete.txt") }"#)
            .await;
        assert!(res.is_ok());
        let data = res.data.into_json().unwrap();
        assert!(data["deleteFile"].as_bool().unwrap());
    }

    // -- Handler: graphql_handler --------------------------------------------

    #[tokio::test]
    async fn handler_returns_json_on_valid_query() {
        let schema = schema();
        let req = async_graphql::Request::new("{ health { status } }");
        let response = graphql_handler(axum::Extension(schema), axum::Json(req)).await;
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body["data"]["health"]["status"], "ok");
    }

    #[tokio::test]
    async fn handler_returns_bad_request_on_invalid_query() {
        let schema = schema();
        let req = async_graphql::Request::new("{ invalidField }");
        let response = graphql_handler(axum::Extension(schema), axum::Json(req)).await;
        let status = response.status();
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
    }

    // -- Handler: playground -------------------------------------------------

    #[tokio::test]
    async fn playground_returns_html() {
        let html = graphql_playground().await;
        assert!(html.0.contains("<html"));
        assert!(html.0.contains("/api/graphql"));
    }

    // -- Edge cases ----------------------------------------------------------

    #[tokio::test]
    async fn delete_file_empty_path_is_error() {
        let ctx = test_ctx();
        let s = build_schema(ctx);
        let res = s.execute(r#"mutation { deleteFile(path: "") }"#).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn head_file_error_returns_null_not_error() {
        let ctx = test_ctx();
        let s = build_schema(ctx);
        let res = s
            .execute(r#"{ file(path: "/does-not-exist") { path } }"#)
            .await;
        assert!(res.is_ok());
        let data = res.data.into_json().unwrap();
        assert!(data["file"].is_null());
    }

    #[tokio::test]
    async fn schema_execute_returns_is_ok_flag() {
        let s = schema();
        let req = async_graphql::Request::new("{ health { status } }");
        let res = s.execute(req).await;
        assert!(res.is_ok());
    }

    // -- Auth: me() -----------------------------------------------------------

    #[tokio::test]
    async fn query_me_returns_anonymous_when_no_user() {
        let ctx = test_ctx();
        let s = build_schema(ctx);
        let res = s.execute("{ me { username role } }").await;
        assert!(res.is_ok());
        let data = res.data.into_json().unwrap();
        assert_eq!(data["me"]["username"], "anonymous");
        assert_eq!(data["me"]["role"], "viewer");
    }

    #[tokio::test]
    async fn query_me_returns_authenticated_user() {
        let mut ctx = test_ctx();
        ctx.current_user = Some(CurrentUser {
            username: "alice".to_string(),
            role: "admin".to_string(),
        });
        let s = build_schema(ctx);
        let res = s.execute("{ me { username role } }").await;
        assert!(res.is_ok());
        let data = res.data.into_json().unwrap();
        assert_eq!(data["me"]["username"], "alice");
        assert_eq!(data["me"]["role"], "admin");
    }
}
