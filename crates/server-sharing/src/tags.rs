use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
pub use ferro_server_collaboration::tags::{
    AddTagsRequest, FileTags, SearchTagQuery, TagStore,
};

use crate::SharingState;
use crate::api_error::ApiError;

pub async fn list_tags(Extension(state): Extension<SharingState>) -> Response {
    let all_tags = state.tags.list_all_tags();
    let tags_json: Vec<serde_json::Value> = all_tags
        .into_iter()
        .map(|(tag, count)| serde_json::json!({ "tag": tag, "count": count }))
        .collect();
    (StatusCode::OK, axum::Json(serde_json::json!({ "tags": tags_json }))).into_response()
}

pub async fn get_tags(Extension(state): Extension<SharingState>, Path(path): Path<String>) -> Response {
    let tags = state.tags.get_tags(&path);
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({ "path": path, "tags": tags })),
    )
        .into_response()
}

pub async fn add_tags(
    Extension(state): Extension<SharingState>,
    Path(path): Path<String>,
    axum::Json(body): axum::Json<AddTagsRequest>,
) -> Response {
    let mut errors: Vec<String> = Vec::new();
    let mut added: Vec<String> = Vec::new();

    for tag in &body.tags {
        match state.tags.add_tag(&path, tag) {
            Ok(()) => added.push(tag.clone()),
            Err(e) => errors.push(format!("{}: {}", tag, e)),
        }
    }

    if added.is_empty() && !errors.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "added": added,
                "errors": errors,
            })),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "added": added,
            "errors": errors,
        })),
    )
        .into_response()
}

pub async fn remove_tag(
    Extension(state): Extension<SharingState>,
    axum::extract::Path((path, tag)): axum::extract::Path<(String, String)>,
) -> Response {
    let removed = state.tags.remove_tag(&path, &tag);
    if removed {
        (StatusCode::OK, axum::Json(serde_json::json!({ "status": "ok" }))).into_response()
    } else {
        ApiError::not_found(ApiError::NOT_FOUND, "Tag not found on file")
    }
}

pub async fn search_by_tag(
    Extension(state): Extension<SharingState>,
    Query(params): Query<SearchTagQuery>,
) -> Response {
    let files = state.tags.find_by_tag(&params.tag);
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({ "tag": params.tag, "files": files })),
    )
        .into_response()
}
