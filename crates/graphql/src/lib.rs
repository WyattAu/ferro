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

    async fn me(&self, _ctx: &Context<'_>) -> async_graphql::Result<UserItem> {
        Ok(UserItem {
            username: "admin".to_string(),
            role: "admin".to_string(),
        })
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
