use async_graphql::{Context, EmptySubscription, Object, Schema};
use axum::response::{Html, IntoResponse, Response};

pub struct Query;

#[Object]
impl Query {
    async fn files(
        &self,
        ctx: &Context<'_>,
        path: Option<String>,
        limit: Option<i32>,
    ) -> async_graphql::Result<Vec<FileItem>> {
        let state = ctx.data::<crate::AppState>().unwrap();
        let prefix = path.unwrap_or_else(|| "/".to_string());
        let files = state
            .storage
            .list(&prefix)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        let limit = limit.unwrap_or(100).min(1000) as usize;
        Ok(files
            .into_iter()
            .take(limit)
            .map(FileItem::from)
            .collect())
    }

    async fn file(
        &self,
        ctx: &Context<'_>,
        path: String,
    ) -> async_graphql::Result<Option<FileItem>> {
        let state = ctx.data::<crate::AppState>().unwrap();
        match state.storage.head(&path).await {
            Ok(meta) => Ok(Some(FileItem::from(meta))),
            Err(_) => Ok(None),
        }
    }

    async fn shares(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<ShareItem>> {
        let state = ctx.data::<crate::AppState>().unwrap();
        let links = state.share_store.list().await;
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
    ) -> async_graphql::Result<Vec<AuditItem>> {
        let state = ctx.data::<crate::AppState>().unwrap();
        let limit = limit.unwrap_or(50) as usize;
        let offset = offset.unwrap_or(0) as usize;
        let entries = state.audit_log.recent_with_offset(limit, offset).await;
        Ok(entries.into_iter().map(AuditItem::from).collect())
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
        let state = ctx.data::<crate::AppState>().unwrap();
        let meta = state
            .storage
            .create_collection(&path, "admin")
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(FileItem::from(meta))
    }

    async fn delete_file(
        &self,
        ctx: &Context<'_>,
        path: String,
    ) -> async_graphql::Result<bool> {
        let state = ctx.data::<crate::AppState>().unwrap();
        state
            .storage
            .delete(&path)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(true)
    }
}

pub type AppSchema = Schema<Query, Mutation, EmptySubscription>;

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
            name: m.path.rsplit('/').next().unwrap_or("unknown").to_string(),
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

impl From<crate::shares::ShareLink> for ShareItem {
    fn from(l: crate::shares::ShareLink) -> Self {
        Self {
            token: l.token,
            path: l.path,
            expires_at: l.expires_at.to_string(),
            password_protected: l.password.is_some(),
            max_downloads: l.max_downloads,
            download_count: l.download_count,
            created_by: l.created_by,
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

#[derive(async_graphql::SimpleObject)]
pub struct AuditItem {
    pub method: String,
    pub path: String,
    pub user: String,
    pub status: u16,
    pub timestamp: String,
}

impl From<crate::audit::AuditEntry> for AuditItem {
    fn from(e: crate::audit::AuditEntry) -> Self {
        Self {
            method: e.method,
            path: e.path,
            user: e.user,
            status: e.status,
            timestamp: e.timestamp,
        }
    }
}

pub fn build_schema(state: crate::AppState) -> AppSchema {
    Schema::build(Query, Mutation, EmptySubscription)
        .data(state)
        .finish()
}

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

pub async fn graphql_playground() -> Html<String> {
    Html(async_graphql::http::playground_source(
        async_graphql::http::GraphQLPlaygroundConfig::new("/api/graphql"),
    ))
}
