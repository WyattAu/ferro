use crate::AppState;

#[derive(Clone, Debug)]
pub struct FileEvent {
    pub op_type: &'static str,
    pub path: String,
    pub new_path: Option<String>,
    pub size: Option<u64>,
    pub mime_type: Option<String>,
    pub owner: String,
    pub etag: Option<String>,
    pub already_existed: bool,
}

pub async fn dispatch_post_op(state: &AppState, event: FileEvent) {
    let ws_event = match event.op_type {
        "put" | "mkcol" => {
            if event.already_existed {
                Some(crate::ws::WsEvent::FileUpdated {
                    path: event.path.clone(),
                    size: event.size.unwrap_or(0),
                    owner: event.owner.clone(),
                })
            } else {
                Some(crate::ws::WsEvent::FileCreated {
                    path: event.path.clone(),
                    size: event.size.unwrap_or(0),
                    owner: event.owner.clone(),
                })
            }
        }
        "delete" => Some(crate::ws::WsEvent::FileDeleted {
            path: event.path.clone(),
            owner: event.owner.clone(),
        }),
        "move" => Some(crate::ws::WsEvent::FileMoved {
            from: event.path.clone(),
            to: event.new_path.clone().unwrap_or_default(),
            owner: event.owner.clone(),
        }),
        "copy" => Some(crate::ws::WsEvent::FileCreated {
            path: event.new_path.clone().unwrap_or_else(|| event.path.clone()),
            size: event.size.unwrap_or(0),
            owner: event.owner.clone(),
        }),
        _ => None,
    };
    if let Some(ws) = ws_event {
        state.ws_manager.broadcast(&ws);
    }

    state.read_cache.invalidate_path(&event.path);
    if let Some(np) = &event.new_path {
        state.read_cache.invalidate_path(np);
    }

    let webhook_event = match event.op_type {
        "put" => crate::webhooks::WebhookEvent {
            event: if event.already_existed {
                "file.modify".to_string()
            } else {
                "file.upload".to_string()
            },
            timestamp: chrono::Utc::now().to_rfc3339(),
            path: event.path.clone(),
            size: event.size,
            user: Some(event.owner.clone()),
            etag: event.etag.clone(),
        },
        "delete" => crate::webhooks::WebhookEvent {
            event: "file.delete".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            path: event.path.clone(),
            size: None,
            user: Some(event.owner.clone()),
            etag: None,
        },
        "mkcol" => crate::webhooks::WebhookEvent {
            event: "file.upload".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            path: event.path.clone(),
            size: None,
            user: Some(event.owner.clone()),
            etag: None,
        },
        "move" => crate::webhooks::WebhookEvent {
            event: "file.modify".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            path: event.path.clone(),
            size: event.size,
            user: Some(event.owner.clone()),
            etag: event.etag.clone(),
        },
        "copy" => crate::webhooks::WebhookEvent {
            event: "file.upload".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            path: event.new_path.clone().unwrap_or_else(|| event.path.clone()),
            size: event.size,
            user: Some(event.owner.clone()),
            etag: None,
        },
        _ => return,
    };
    crate::webhooks::fire_webhooks(
        state.webhooks.clone(),
        webhook_event.clone(),
        state.db.clone(),
    )
    .await;

    // Email notifications when enabled
    if state.email_config.enabled {
        let subject = format!("Ferro: {}", webhook_event.event);
        let body = format!(
            "Event: {}\nPath: {}\nTimestamp: {}",
            webhook_event.event, webhook_event.path, webhook_event.timestamp
        );
        let msg = crate::email::EmailMessage {
            to: webhook_event.user.clone().unwrap_or_default(),
            subject,
            body_text: body,
            body_html: None,
        };
        tokio::spawn(async move {
            if let Err(e) =
                crate::email::send_email(&crate::email::EmailConfig::default(), &msg).await
            {
                tracing::warn!("Email notification failed: {}", e);
            }
        });
    }
}
