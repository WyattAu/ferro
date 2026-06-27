pub mod error;
pub mod group;
pub mod handler;
pub mod schema;
pub mod user;

use axum::Router;
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct ScimState {
    pub group_store: Arc<group::GroupStore>,
}

impl ScimState {
    pub fn new() -> Self {
        Self {
            group_store: Arc::new(group::GroupStore::new()),
        }
    }
}

pub fn routes(state: ScimState) -> Router {
    Router::new()
        .route(
            "/scim/v2/Users",
            axum::routing::get(handler::list_users).post(handler::create_user),
        )
        .route(
            "/scim/v2/Users/{id}",
            axum::routing::get(handler::get_user)
                .put(handler::replace_user)
                .delete(handler::delete_user),
        )
        .route(
            "/scim/v2/Groups",
            axum::routing::get(handler::list_groups).post(handler::create_group),
        )
        .route(
            "/scim/v2/Groups/{id}",
            axum::routing::get(handler::get_group)
                .put(handler::replace_group)
                .delete(handler::delete_group),
        )
        .route(
            "/scim/v2/ServiceProviderConfig",
            axum::routing::get(handler::sp_config),
        )
        .route("/scim/v2/Schemas", axum::routing::get(handler::schemas))
        .route(
            "/scim/v2/ResourceTypes",
            axum::routing::get(handler::resource_types),
        )
        .with_state(state)
}
