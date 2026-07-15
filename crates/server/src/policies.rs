use axum::extract::State;
use axum::response::Response;

use crate::AppState;
use ferro_auth::policies::PolicyState;
use ferro_server_state::ServerState;

pub async fn list_policies(State(state): State<AppState>) -> Response {
    let policy_state = PolicyState {
        cedar: state.cedar().clone(),
    };
    ferro_auth::policies::list_policies(State(policy_state)).await
}

pub async fn add_policy(
    State(state): State<AppState>,
    body: axum::Json<ferro_auth::policies::AddPolicyRequest>,
) -> Response {
    let policy_state = PolicyState {
        cedar: state.cedar().clone(),
    };
    ferro_auth::policies::add_policy(State(policy_state), body).await
}

pub async fn delete_policy(
    State(state): State<AppState>,
    body: axum::Json<ferro_auth::policies::DeletePolicyRequest>,
) -> Response {
    let policy_state = PolicyState {
        cedar: state.cedar().clone(),
    };
    ferro_auth::policies::delete_policy(State(policy_state), body).await
}
