use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use super::offers::SignalingOffer;

#[derive(Deserialize)]
pub struct CreateOfferRequest {
    pub sdp: String,
    pub file_path: String,
    pub file_name: String,
    pub file_size: u64,
}

#[derive(Serialize)]
pub struct CreateOfferResponse {
    pub session_id: String,
    pub watch_url: String,
}

pub async fn create_offer(
    State(state): State<crate::AppState>,
    axum::Json(req): axum::Json<CreateOfferRequest>,
) -> Response {
    let session_id = uuid::Uuid::new_v4().to_string();
    let offer = SignalingOffer {
        session_id: session_id.clone(),
        sdp: req.sdp,
        ice_candidates: Vec::new(),
        created_at: std::time::Instant::now(),
        file_path: req.file_path,
        file_name: req.file_name,
        file_size: req.file_size,
    };
    state.webrtc_offers.create(offer);

    let watch_url = format!("/api/webrtc/offer/{}", session_id);
    (
        StatusCode::OK,
        axum::Json(CreateOfferResponse {
            session_id,
            watch_url,
        }),
    )
        .into_response()
}

pub async fn get_offer(
    State(state): State<crate::AppState>,
    Path(session_id): Path<String>,
) -> Response {
    match state.webrtc_offers.get(&session_id) {
        Some(offer) => (StatusCode::OK, axum::Json(offer)).into_response(),
        None => (StatusCode::NOT_FOUND, "Offer not found or expired").into_response(),
    }
}

#[derive(Deserialize)]
pub struct AnswerRequest {
    pub sdp: String,
}

#[derive(Serialize)]
pub struct AnswerResponse {
    pub ice_candidates: Vec<String>,
}

pub async fn submit_answer(
    State(state): State<crate::AppState>,
    Path(session_id): Path<String>,
    axum::Json(req): axum::Json<AnswerRequest>,
) -> Response {
    match state.webrtc_offers.get(&session_id) {
        Some(offer) => {
            state
                .webrtc_offers
                .add_ice_candidate(&session_id, format!("__ANSWER__{}", req.sdp));
            (
                StatusCode::OK,
                axum::Json(AnswerResponse {
                    ice_candidates: offer.ice_candidates,
                }),
            )
                .into_response()
        }
        None => (StatusCode::NOT_FOUND, "Offer not found or expired").into_response(),
    }
}

#[derive(Deserialize)]
pub struct IceRequest {
    pub candidate: String,
}

pub async fn add_ice_candidate(
    State(state): State<crate::AppState>,
    Path(session_id): Path<String>,
    axum::Json(req): axum::Json<IceRequest>,
) -> Response {
    if state
        .webrtc_offers
        .add_ice_candidate(&session_id, req.candidate)
    {
        StatusCode::OK.into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

pub async fn poll_answer(
    State(state): State<crate::AppState>,
    Path(session_id): Path<String>,
) -> Response {
    let offer = match state.webrtc_offers.get(&session_id) {
        Some(o) => o,
        None => return (StatusCode::NOT_FOUND, "Offer not found").into_response(),
    };

    let answer = offer
        .ice_candidates
        .iter()
        .find(|c| c.starts_with("__ANSWER__"))
        .map(|c| c.strip_prefix("__ANSWER__").unwrap_or_default().to_string());

    match answer {
        Some(sdp) => {
            state.webrtc_offers.remove(&session_id);
            (
                StatusCode::OK,
                axum::Json(serde_json::json!({
                    "type": "answer",
                    "sdp": sdp,
                })),
            )
                .into_response()
        }
        None => (
            StatusCode::OK,
            axum::Json(serde_json::json!({
                "type": "pending",
            })),
        )
            .into_response(),
    }
}
