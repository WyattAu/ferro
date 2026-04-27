use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use axum::Extension;

use crate::AppState;

async fn caldav_state(state: &AppState) -> ferro_dav::caldav::CalDavState {
    ferro_dav::caldav::CalDavState {
        store: state.calendar_store.clone(),
        principal: "default".to_string(),
    }
}

async fn carddav_state(state: &AppState) -> ferro_dav::carddav::CardDavState {
    ferro_dav::carddav::CardDavState {
        store: state.address_book_store.clone(),
        principal: "default".to_string(),
    }
}

pub async fn caldav_options() -> impl IntoResponse {
    ferro_dav::caldav::options_handler().await
}

pub async fn caldav_list(State(state): State<AppState>) -> Response {
    ferro_dav::caldav::list_calendars(axum::extract::State(caldav_state(&state).await)).await
}

pub async fn caldav_create(State(state): State<AppState>) -> Response {
    ferro_dav::caldav::create_calendar_handler(axum::extract::State(caldav_state(&state).await)).await
}

pub async fn caldav_delete(
    State(state): State<AppState>,
    Path(calendar): Path<String>,
) -> Response {
    ferro_dav::caldav::delete_calendar_handler(
        axum::extract::State(caldav_state(&state).await),
        Path(calendar),
    )
    .await
}

pub async fn caldav_props(
    State(state): State<AppState>,
    Path(calendar): Path<String>,
) -> Response {
    ferro_dav::caldav::calendar_properties(
        axum::extract::State(caldav_state(&state).await),
        Path(calendar),
    )
    .await
}

pub async fn caldav_get_event(
    State(state): State<AppState>,
    Path((calendar, uid)): Path<(String, String)>,
) -> Response {
    ferro_dav::caldav::get_event(
        axum::extract::State(caldav_state(&state).await),
        Path((calendar, uid)),
    )
    .await
}

pub async fn caldav_put_event(
    State(state): State<AppState>,
    Path((calendar, uid)): Path<(String, String)>,
    Extension(body): Extension<Bytes>,
) -> Response {
    ferro_dav::caldav::put_event(
        axum::extract::State(caldav_state(&state).await),
        Path((calendar, uid)),
        Extension(body),
    )
    .await
}

pub async fn caldav_delete_event(
    State(state): State<AppState>,
    Path((calendar, uid)): Path<(String, String)>,
) -> Response {
    ferro_dav::caldav::delete_event(
        axum::extract::State(caldav_state(&state).await),
        Path((calendar, uid)),
    )
    .await
}

pub async fn carddav_options() -> impl IntoResponse {
    ferro_dav::carddav::options_handler().await
}

pub async fn carddav_list(State(state): State<AppState>) -> Response {
    ferro_dav::carddav::list_address_books(axum::extract::State(
        carddav_state(&state).await,
    ))
    .await
}

pub async fn carddav_create(State(state): State<AppState>) -> Response {
    ferro_dav::carddav::create_address_book_handler(axum::extract::State(
        carddav_state(&state).await,
    ))
    .await
}

pub async fn carddav_delete(
    State(state): State<AppState>,
    Path(book): Path<String>,
) -> Response {
    ferro_dav::carddav::delete_address_book_handler(
        axum::extract::State(carddav_state(&state).await),
        Path(book),
    )
    .await
}

pub async fn carddav_props(
    State(state): State<AppState>,
    Path(book): Path<String>,
) -> Response {
    ferro_dav::carddav::address_book_properties(
        axum::extract::State(carddav_state(&state).await),
        Path(book),
    )
    .await
}

pub async fn carddav_get_contact(
    State(state): State<AppState>,
    Path((book, uid)): Path<(String, String)>,
) -> Response {
    ferro_dav::carddav::get_contact(
        axum::extract::State(carddav_state(&state).await),
        Path((book, uid)),
    )
    .await
}

pub async fn carddav_put_contact(
    State(state): State<AppState>,
    Path((book, uid)): Path<(String, String)>,
    Extension(body): Extension<Bytes>,
) -> Response {
    ferro_dav::carddav::put_contact(
        axum::extract::State(carddav_state(&state).await),
        Path((book, uid)),
        Extension(body),
    )
    .await
}

pub async fn carddav_delete_contact(
    State(state): State<AppState>,
    Path((book, uid)): Path<(String, String)>,
) -> Response {
    ferro_dav::carddav::delete_contact(
        axum::extract::State(carddav_state(&state).await),
        Path((book, uid)),
    )
    .await
}
