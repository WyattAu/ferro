use crate::store::DynAddressBookStore;
use crate::xml_ext::{self, DavProp, DavResponse, PropStat};
use axum::Extension;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};

/// Shared state for `CardDAV` Axum handlers.
#[derive(Clone)]
pub struct CardDavState {
    /// The address book store backend.
    pub store: DynAddressBookStore,
    /// The authenticated principal (user).
    pub principal: String,
}

/// Handle HTTP OPTIONS for `CardDAV` capability discovery.
pub async fn options_handler() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert("DAV", "1, 2, addressbook".parse().expect("static DAV header value"));
    headers.insert(
        "Allow",
        "OPTIONS, GET, PUT, DELETE, PROPFIND, REPORT"
            .parse()
            .expect("static Allow header value"),
    );
    (StatusCode::NO_CONTENT, headers)
}

fn dav_multistatus(body: Vec<u8>) -> Response {
    Response::builder()
        .status(StatusCode::MULTI_STATUS)
        .header("Content-Type", "application/xml; charset=utf-8")
        .body(body.into())
        .unwrap_or_else(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to build response: {e}"),
            )
                .into_response()
        })
}

fn dav_response_with_etag(status: StatusCode, etag: &str) -> Response {
    Response::builder()
        .status(status)
        .header("ETag", etag)
        .body(Bytes::new().into())
        .unwrap_or_else(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to build response: {e}"),
            )
                .into_response()
        })
}

fn dav_created(location: &str) -> Response {
    Response::builder()
        .status(StatusCode::CREATED)
        .header("Location", location)
        .body(Bytes::new().into())
        .unwrap_or_else(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to build response: {e}"),
            )
                .into_response()
        })
}

fn dav_ok_with_content_type(content_type: &str, etag: &str, body: String) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", content_type)
        .header("ETag", etag)
        .body(body.into())
        .unwrap_or_else(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to build response: {e}"),
            )
                .into_response()
        })
}

/// List all address books for the authenticated principal.
pub async fn list_address_books(State(state): State<CardDavState>) -> Response {
    let books = state.store.list_address_books(&state.principal).await;
    let responses: Vec<DavResponse> = books
        .iter()
        .map(|book| DavResponse {
            href: format!("/dav/card/{}/", book.id),
            propstats: vec![PropStat {
                status: 200,
                props: vec![
                    DavProp {
                        name: "D:resourcetype".to_string(),
                        namespace: None,
                        value: Some("<A:addressbook xmlns:A=\"urn:ietf:params:xml:ns:carddav\"/>".to_string()),
                    },
                    DavProp {
                        name: "D:displayname".to_string(),
                        namespace: None,
                        value: Some(xml_ext::escape_xml(&book.name).into_owned()),
                    },
                    DavProp {
                        name: "A:getctag".to_string(),
                        namespace: Some("urn:ietf:params:xml:ns:carddav".to_string()),
                        value: Some(book.ctag.clone()),
                    },
                ],
            }],
        })
        .collect();

    dav_multistatus(xml_ext::build_dav_multistatus(&responses))
}

/// Retrieve properties of a specific address book.
pub async fn address_book_properties(State(state): State<CardDavState>, Path(book): Path<String>) -> Response {
    let Some(book_info) = state.store.get_address_book(&state.principal, &book).await else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let response = DavResponse {
        href: format!("/dav/card/{book}/"),
        propstats: vec![PropStat {
            status: 200,
            props: vec![
                DavProp {
                    name: "D:resourcetype".to_string(),
                    namespace: None,
                    value: Some("<A:addressbook xmlns:A=\"urn:ietf:params:xml:ns:carddav\"/>".to_string()),
                },
                DavProp {
                    name: "D:displayname".to_string(),
                    namespace: None,
                    value: Some(xml_ext::escape_xml(&book_info.name).into_owned()),
                },
                DavProp {
                    name: "A:getctag".to_string(),
                    namespace: Some("urn:ietf:params:xml:ns:carddav".to_string()),
                    value: Some(book_info.ctag.clone()),
                },
            ],
        }],
    };

    let body = xml_ext::build_dav_multistatus(&[response]);
    dav_multistatus(body)
}

/// Create a new address book.
pub async fn create_address_book_handler(State(state): State<CardDavState>) -> Response {
    match state.store.create_address_book(&state.principal, "Contacts").await {
        Ok(book) => dav_created(&format!("/dav/card/{}/", book.id)),
        Err(_) => StatusCode::CONFLICT.into_response(),
    }
}

/// Delete an address book.
pub async fn delete_address_book_handler(State(state): State<CardDavState>, Path(book): Path<String>) -> Response {
    match state.store.delete_address_book(&state.principal, &book).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Retrieve a contact by address book ID and UID.
pub async fn get_contact(State(state): State<CardDavState>, Path((book, uid)): Path<(String, String)>) -> Response {
    let Some(contact) = state.store.get_contact(&book, &uid).await else {
        return StatusCode::NOT_FOUND.into_response();
    };

    dav_ok_with_content_type("text/vcard; charset=utf-8", &contact.etag, contact.vcard_data)
}

/// Create or update a contact (PUT).
pub async fn put_contact(
    State(state): State<CardDavState>,
    Path((book, uid)): Path<(String, String)>,
    Extension(body): Extension<Bytes>,
) -> Response {
    let vcard = String::from_utf8_lossy(&body).to_string();

    if state.store.get_contact(&book, &uid).await.is_some() {
        match state.store.update_contact(&book, &uid, &vcard).await {
            Ok(contact) => dav_response_with_etag(StatusCode::NO_CONTENT, &contact.etag),
            Err(_) => StatusCode::NOT_FOUND.into_response(),
        }
    } else {
        match state.store.create_contact(&book, &vcard).await {
            Ok(contact) => dav_response_with_etag(StatusCode::CREATED, &contact.etag),
            Err(_) => StatusCode::CONFLICT.into_response(),
        }
    }
}

/// Delete a contact.
pub async fn delete_contact(State(state): State<CardDavState>, Path((book, uid)): Path<(String, String)>) -> Response {
    match state.store.delete_contact(&book, &uid).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Handle a `CardDAV` REPORT request (addressbook-query).
pub async fn handle_report(State(state): State<CardDavState>, Extension(body): Extension<Bytes>) -> Response {
    let filter_text = xml_ext::parse_addressbook_query_filter(&body);

    let books = state.store.list_address_books(&state.principal).await;
    let mut responses = Vec::new();

    for book in &books {
        let contacts = state.store.list_contacts(&book.id).await;
        for contact in &contacts {
            let include = if let Some(ref query) = filter_text {
                let query_lower = query.to_lowercase();
                contact.vcard_data.to_lowercase().contains(&query_lower)
            } else {
                true
            };

            if include {
                responses.push(DavResponse {
                    href: format!("/dav/card/{}/{}.vcf", book.id, contact.uid),
                    propstats: vec![PropStat {
                        status: 200,
                        props: vec![
                            DavProp {
                                name: "D:getetag".to_string(),
                                namespace: None,
                                value: Some(contact.etag.clone()),
                            },
                            DavProp {
                                name: "A:address-data".to_string(),
                                namespace: Some("urn:ietf:params:xml:ns:carddav".to_string()),
                                value: Some(contact.vcard_data.clone()),
                            },
                        ],
                    }],
                });
            }
        }
    }

    let xml_body = xml_ext::build_dav_multistatus(&responses);
    dav_multistatus(xml_body)
}

/// Handle a `CardDAV` addressbook-multiget REPORT request (RFC 6352 Section 8.4).
/// Retrieves specific contacts by href.
pub async fn handle_multiget(State(state): State<CardDavState>, Extension(body): Extension<Bytes>) -> Response {
    let hrefs = xml_ext::parse_multiget_hrefs(&body);
    let mut responses = Vec::new();

    for href in &hrefs {
        // Parse href: expect "/dav/card/{book}/{uid}.vcf"
        let path = href.trim_matches('/').trim_start_matches("dav/card/");
        let parts: Vec<&str> = path.splitn(2, '/').collect();
        if parts.len() != 2 {
            continue;
        }
        let book = parts[0];
        let uid = parts[1].strip_suffix(".vcf").unwrap_or(parts[1]);

        if let Some(contact) = state.store.get_contact(book, uid).await {
            responses.push(DavResponse {
                href: href.clone(),
                propstats: vec![PropStat {
                    status: 200,
                    props: vec![
                        DavProp {
                            name: "D:getetag".to_string(),
                            namespace: None,
                            value: Some(contact.etag.clone()),
                        },
                        DavProp {
                            name: "A:address-data".to_string(),
                            namespace: Some("urn:ietf:params:xml:ns:carddav".to_string()),
                            value: Some(contact.vcard_data.clone()),
                        },
                    ],
                }],
            });
        } else {
            responses.push(DavResponse {
                href: href.clone(),
                propstats: vec![PropStat {
                    status: 404,
                    props: vec![
                        DavProp {
                            name: "D:getetag".to_string(),
                            namespace: None,
                            value: None,
                        },
                        DavProp {
                            name: "A:address-data".to_string(),
                            namespace: Some("urn:ietf:params:xml:ns:carddav".to_string()),
                            value: None,
                        },
                    ],
                }],
            });
        }
    }

    let xml_body = xml_ext::build_dav_multistatus(&responses);
    dav_multistatus(xml_body)
}
