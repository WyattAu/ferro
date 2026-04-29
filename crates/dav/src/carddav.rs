use crate::store::DynAddressBookStore;
use crate::xml_ext::{self, DavProp, DavResponse, PropStat};
use axum::Extension;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};

#[derive(Clone)]
pub struct CardDavState {
    pub store: DynAddressBookStore,
    pub principal: String,
}

pub async fn options_handler() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert("DAV", "1, 2, addressbook".parse().unwrap());
    headers.insert(
        "Allow",
        "OPTIONS, GET, PUT, DELETE, PROPFIND, REPORT"
            .parse()
            .unwrap(),
    );
    (StatusCode::NO_CONTENT, headers)
}

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
                        value: Some(
                            "<A:addressbook xmlns:A=\"urn:ietf:params:xml:ns:carddav\"/>"
                                .to_string(),
                        ),
                    },
                    DavProp {
                        name: "D:displayname".to_string(),
                        namespace: None,
                        value: Some(xml_ext::escape_xml(&book.name)),
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

    let body = xml_ext::build_dav_multistatus(&responses);
    Response::builder()
        .status(StatusCode::MULTI_STATUS)
        .header("Content-Type", "application/xml; charset=utf-8")
        .body(body.into())
        .unwrap()
}

pub async fn address_book_properties(
    State(state): State<CardDavState>,
    Path(book): Path<String>,
) -> Response {
    let Some(book_info) = state.store.get_address_book(&state.principal, &book).await else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let response = DavResponse {
        href: format!("/dav/card/{}/", book),
        propstats: vec![PropStat {
            status: 200,
            props: vec![
                DavProp {
                    name: "D:resourcetype".to_string(),
                    namespace: None,
                    value: Some(
                        "<A:addressbook xmlns:A=\"urn:ietf:params:xml:ns:carddav\"/>".to_string(),
                    ),
                },
                DavProp {
                    name: "D:displayname".to_string(),
                    namespace: None,
                    value: Some(xml_ext::escape_xml(&book_info.name)),
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
    Response::builder()
        .status(StatusCode::MULTI_STATUS)
        .header("Content-Type", "application/xml; charset=utf-8")
        .body(body.into())
        .unwrap()
}

pub async fn create_address_book_handler(State(state): State<CardDavState>) -> Response {
    match state
        .store
        .create_address_book(&state.principal, "Contacts")
        .await
    {
        Ok(book) => Response::builder()
            .status(StatusCode::CREATED)
            .header("Location", format!("/dav/card/{}/", book.id))
            .body(Bytes::new().into())
            .unwrap(),
        Err(_) => StatusCode::CONFLICT.into_response(),
    }
}

pub async fn delete_address_book_handler(
    State(state): State<CardDavState>,
    Path(book): Path<String>,
) -> Response {
    match state
        .store
        .delete_address_book(&state.principal, &book)
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

pub async fn get_contact(
    State(state): State<CardDavState>,
    Path((book, uid)): Path<(String, String)>,
) -> Response {
    let Some(contact) = state.store.get_contact(&book, &uid).await else {
        return StatusCode::NOT_FOUND.into_response();
    };

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/vcard; charset=utf-8")
        .header("ETag", &contact.etag)
        .body(contact.vcard_data.into())
        .unwrap()
}

pub async fn put_contact(
    State(state): State<CardDavState>,
    Path((book, uid)): Path<(String, String)>,
    Extension(body): Extension<Bytes>,
) -> Response {
    let vcard = String::from_utf8_lossy(&body).to_string();

    if state.store.get_contact(&book, &uid).await.is_some() {
        match state.store.update_contact(&book, &uid, &vcard).await {
            Ok(contact) => Response::builder()
                .status(StatusCode::NO_CONTENT)
                .header("ETag", &contact.etag)
                .body(Bytes::new().into())
                .unwrap(),
            Err(_) => StatusCode::NOT_FOUND.into_response(),
        }
    } else {
        match state.store.create_contact(&book, &vcard).await {
            Ok(contact) => Response::builder()
                .status(StatusCode::CREATED)
                .header("ETag", &contact.etag)
                .body(Bytes::new().into())
                .unwrap(),
            Err(_) => StatusCode::CONFLICT.into_response(),
        }
    }
}

pub async fn delete_contact(
    State(state): State<CardDavState>,
    Path((book, uid)): Path<(String, String)>,
) -> Response {
    match state.store.delete_contact(&book, &uid).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

pub async fn handle_report(
    State(state): State<CardDavState>,
    Extension(body): Extension<Bytes>,
) -> Response {
    let _filter_prop = xml_ext::parse_addressbook_query_filter(&body);

    let books = state.store.list_address_books(&state.principal).await;
    let mut responses = Vec::new();

    for book in &books {
        let contacts = state.store.list_contacts(&book.id).await;
        for contact in &contacts {
            let include = if let Some(ref prop) = _filter_prop {
                contact.vcard_data.contains(prop)
                    || contact
                        .vcard_data
                        .to_uppercase()
                        .contains(&prop.to_uppercase())
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
    Response::builder()
        .status(StatusCode::MULTI_STATUS)
        .header("Content-Type", "application/xml; charset=utf-8")
        .body(xml_body.into())
        .unwrap()
}
