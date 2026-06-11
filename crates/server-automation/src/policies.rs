use axum::extract::Extension;
use axum::response::Response;
use std::sync::Arc;

use crate::AutomationState;

pub async fn list_policies(Extension(state): Extension<Arc<AutomationState>>) -> Response {
    let policy_state = ferro_auth::policies::PolicyState {
        cedar: state.cedar.clone(),
    };
    ferro_auth::policies::list_policies(axum::extract::State(policy_state)).await
}

pub async fn add_policy(
    Extension(state): Extension<Arc<AutomationState>>,
    body: axum::Json<ferro_auth::policies::AddPolicyRequest>,
) -> Response {
    let policy_state = ferro_auth::policies::PolicyState {
        cedar: state.cedar.clone(),
    };
    ferro_auth::policies::add_policy(axum::extract::State(policy_state), body).await
}

pub async fn delete_policy(
    Extension(state): Extension<Arc<AutomationState>>,
    body: axum::Json<ferro_auth::policies::DeletePolicyRequest>,
) -> Response {
    let policy_state = ferro_auth::policies::PolicyState {
        cedar: state.cedar.clone(),
    };
    ferro_auth::policies::delete_policy(axum::extract::State(policy_state), body).await
}
