pub mod offers;
pub mod signaling;

use axum::Router;
use std::sync::Arc;

#[derive(Clone)]
pub struct WebRtcState {
    pub offers: Arc<offers::OfferStore>,
}

pub fn routes<S>(state: WebRtcState) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/offer", axum::routing::post(signaling::create_offer))
        .route("/offer/:session_id", axum::routing::get(signaling::get_offer))
        .route(
            "/offer/:session_id/answer",
            axum::routing::post(signaling::submit_answer),
        )
        .route(
            "/offer/:session_id/ice",
            axum::routing::post(signaling::add_ice_candidate),
        )
        .route("/offer/:session_id/poll", axum::routing::get(signaling::poll_answer))
        .layer(axum::Extension(state))
}
