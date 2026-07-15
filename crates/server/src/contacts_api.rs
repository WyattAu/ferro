use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use crate::AppState;
use ferro_server_state::ServerState;

#[derive(Debug, Serialize)]
pub struct ContactResponse {
    pub uid: String,
    pub address_book_id: String,
    pub vcard_data: String,
    pub etag: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateContactRequest {
    pub address_book_id: String,
    pub vcard_data: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateContactRequest {
    pub vcard_data: String,
}

/// Core logic for listing contacts.
async fn list_contacts_impl<S: ferro_server_state::ServerState>(state: &S) -> impl IntoResponse {
    let books = state.address_book_store().list_address_books("default").await;
    let mut all_contacts = Vec::new();

    for book in &books {
        let contacts = state.address_book_store().list_contacts(&book.id).await;
        for contact in contacts {
            all_contacts.push(ContactResponse {
                uid: contact.uid,
                address_book_id: contact.address_book_id,
                vcard_data: contact.vcard_data,
                etag: contact.etag,
                created_at: contact.created_at.to_rfc3339(),
                updated_at: contact.updated_at.to_rfc3339(),
            });
        }
    }

    Json(serde_json::json!({
        "contacts": all_contacts,
        "address_books": books,
    }))
}

pub async fn list_contacts(State(state): State<AppState>) -> impl IntoResponse {
    list_contacts_impl(&state).await
}

/// Core logic for creating a contact.
async fn create_contact_impl<S: ferro_server_state::ServerState>(
    state: &S,
    req: CreateContactRequest,
) -> impl IntoResponse {
    let books = state.address_book_store().list_address_books("default").await;
    let book_id = if req.address_book_id.is_empty() {
        if let Some(book) = books.first() {
            book.id.clone()
        } else {
            match state
                .address_book_store()
                .create_address_book("default", "Contacts")
                .await
            {
                Ok(book) => book.id,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({"error": e.to_string()})),
                    )
                        .into_response();
                }
            }
        }
    } else {
        req.address_book_id
    };

    match state
        .address_book_store()
        .create_contact(&book_id, &req.vcard_data)
        .await
    {
        Ok(contact) => (
            StatusCode::CREATED,
            Json(ContactResponse {
                uid: contact.uid,
                address_book_id: contact.address_book_id,
                vcard_data: contact.vcard_data,
                etag: contact.etag,
                created_at: contact.created_at.to_rfc3339(),
                updated_at: contact.updated_at.to_rfc3339(),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

pub async fn create_contact(
    State(state): State<AppState>,
    Json(req): Json<CreateContactRequest>,
) -> impl IntoResponse {
    create_contact_impl(&state, req).await
}

/// Core logic for updating a contact.
async fn update_contact_impl<S: ferro_server_state::ServerState>(
    state: &S,
    uid: &str,
    req: UpdateContactRequest,
) -> impl IntoResponse {
    let books = state.address_book_store().list_address_books("default").await;

    for book in &books {
        if state
            .address_book_store()
            .get_contact(&book.id, uid)
            .await
            .is_some()
        {
            match state
                .address_book_store()
                .update_contact(&book.id, uid, &req.vcard_data)
                .await
            {
                Ok(updated) => {
                    return Json(ContactResponse {
                        uid: updated.uid,
                        address_book_id: updated.address_book_id,
                        vcard_data: updated.vcard_data,
                        etag: updated.etag,
                        created_at: updated.created_at.to_rfc3339(),
                        updated_at: updated.updated_at.to_rfc3339(),
                    })
                    .into_response();
                }
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({"error": e.to_string()})),
                    )
                        .into_response();
                }
            }
        }
    }

    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({"error": "Contact not found"})),
    )
        .into_response()
}

pub async fn update_contact(
    State(state): State<AppState>,
    Path(uid): Path<String>,
    Json(req): Json<UpdateContactRequest>,
) -> impl IntoResponse {
    update_contact_impl(&state, &uid, req).await
}

/// Core logic for deleting a contact.
async fn delete_contact_impl<S: ferro_server_state::ServerState>(state: &S, uid: &str) -> impl IntoResponse {
    let books = state.address_book_store().list_address_books("default").await;

    for book in &books {
        if state
            .address_book_store()
            .get_contact(&book.id, uid)
            .await
            .is_some()
        {
            match state
                .address_book_store()
                .delete_contact(&book.id, uid)
                .await
            {
                Ok(()) => return StatusCode::NO_CONTENT.into_response(),
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({"error": e.to_string()})),
                    )
                        .into_response();
                }
            }
        }
    }

    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({"error": "Contact not found"})),
    )
        .into_response()
}

pub async fn delete_contact(
    State(state): State<AppState>,
    Path(uid): Path<String>,
) -> impl IntoResponse {
    delete_contact_impl(&state, &uid).await
}

/// Core logic for exporting contacts.
async fn export_contacts_impl<S: ferro_server_state::ServerState>(state: &S) -> impl IntoResponse {
    let books = state.address_book_store().list_address_books("default").await;
    let mut vcard_data = String::from("BEGIN:VCARD\r\nVERSION:3.0\r\n");

    for book in &books {
        let contacts = state.address_book_store().list_contacts(&book.id).await;
        for contact in contacts {
            vcard_data.push_str(&contact.vcard_data);
            if !contact.vcard_data.ends_with("\r\n") {
                vcard_data.push_str("\r\n");
            }
        }
    }

    vcard_data.push_str("END:VCARD\r\n");

    (
        StatusCode::OK,
        [
            (
                axum::http::header::CONTENT_TYPE,
                "text/vcard; charset=utf-8",
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                "attachment; filename=\"contacts.vcf\"",
            ),
        ],
        vcard_data,
    )
        .into_response()
}

pub async fn export_contacts(State(state): State<AppState>) -> impl IntoResponse {
    export_contacts_impl(&state).await
}

/// Core logic for importing contacts.
async fn import_contacts_impl<S: ferro_server_state::ServerState>(state: &S, body: String) -> impl IntoResponse {
    let books = state.address_book_store().list_address_books("default").await;
    let book_id = if let Some(book) = books.first() {
        book.id.clone()
    } else {
        match state
            .address_book_store()
            .create_address_book("default", "Contacts")
            .await
        {
            Ok(book) => book.id,
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": e.to_string()})),
                )
                    .into_response();
            }
        }
    };

    let mut imported = 0;
    let mut errors = Vec::new();

    for vcard in body.split("BEGIN:VCARD") {
        let vcard = vcard.trim();
        if vcard.is_empty() {
            continue;
        }
        let full_vcard = format!("BEGIN:VCARD\r\n{}", vcard);
        if let Err(e) = state
            .address_book_store()
            .create_contact(&book_id, &full_vcard)
            .await
        {
            errors.push(e.to_string());
        } else {
            imported += 1;
        }
    }

    Json(serde_json::json!({
        "imported": imported,
        "errors": errors,
    }))
    .into_response()
}

pub async fn import_contacts(State(state): State<AppState>, body: String) -> impl IntoResponse {
    import_contacts_impl(&state, body).await
}
