use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::api::auth_info,
        crate::api::list_files,
        crate::api::get_file,
        crate::api::put_file,
        crate::api::delete_file,
        crate::api::mkdir,
        crate::api::copy_file,
        crate::api::move_file_rest,
        crate::admin_api::admin_stats,
        crate::admin_api::admin_storage,
        crate::admin_api::admin_audit,
    ),
    components(
        schemas(
            crate::api::AuthInfoResponse,
            crate::api::FileEntryJson,
            crate::api::ListFilesResponse,
            crate::api::PutFileResponse,
            crate::api::MkdirResponse,
            crate::api::CopyMoveResponse,
            crate::admin_api::AdminStatsResponse,
            crate::admin_api::AdminStorageResponse,
            crate::admin_api::AdminAuditResponse,
            crate::api_error::ApiError,
        )
    ),
    tags(
        (name = "auth", description = "Authentication endpoints"),
        (name = "files", description = "File management endpoints"),
        (name = "admin", description = "Administration endpoints"),
    )
)]
pub struct ApiDoc;

pub fn swagger_ui() -> SwaggerUi {
    SwaggerUi::new("/api/docs").url("/api/docs/openapi.json", ApiDoc::openapi())
}
