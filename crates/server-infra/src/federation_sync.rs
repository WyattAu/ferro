use std::sync::Arc;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::InfraState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSyncActivity {
    pub path: String,
    pub user_id: String,
    pub size: Option<u64>,
    pub content_type: Option<String>,
    pub checksum: Option<String>,
}

impl FileSyncActivity {
    pub fn to_create_object(&self, external_url: &str) -> serde_json::Value {
        serde_json::json!({
            "type": "Document",
            "id": format!("{}/files/{}", external_url, self.path),
            "name": self.path.rsplit('/').next().unwrap_or(&self.path),
            "url": format!("{}/dav/{}", external_url, self.path),
            "attributedTo": format!("{}/fed/actor/{}", external_url, self.user_id),
            "size": self.size,
            "mediaType": self.content_type,
            "checksum": self.checksum,
        })
    }

    pub fn to_update_object(&self, external_url: &str) -> serde_json::Value {
        self.to_create_object(external_url)
    }

    pub fn to_delete_object(&self, external_url: &str) -> serde_json::Value {
        serde_json::json!({
            "type": "Document",
            "id": format!("{}/files/{}", external_url, self.path),
            "name": self.path.rsplit('/').next().unwrap_or(&self.path),
        })
    }
}

pub struct FederationSync<S: InfraState> {
    state: Arc<S>,
}

impl<S: InfraState> FederationSync<S> {
    pub fn new(state: Arc<S>) -> Self {
        Self { state }
    }

    fn federation_state(&self) -> ferro_server_activitypub::FederationState {
        ferro_server_activitypub::FederationState {
            activity_store: self.state.activity_store().clone(),
            external_url: self.state.external_url().to_string(),
            federation_secret: self.state.federation_secret().to_string(),
        }
    }

    pub async fn publish_file_created(&self, activity: &FileSyncActivity) {
        self.publish_activity("Create", activity, |a, url| a.to_create_object(url))
            .await;
    }

    pub async fn publish_file_updated(&self, activity: &FileSyncActivity) {
        self.publish_activity("Update", activity, |a, url| a.to_update_object(url))
            .await;
    }

    pub async fn publish_file_deleted(&self, activity: &FileSyncActivity) {
        self.publish_activity("Delete", activity, |a, url| a.to_delete_object(url))
            .await;
    }

    async fn publish_activity<F>(
        &self,
        activity_type: &str,
        file_activity: &FileSyncActivity,
        object_fn: F,
    ) where
        F: Fn(&FileSyncActivity, &str) -> serde_json::Value,
    {
        if self.state.federation_secret().is_empty() {
            return;
        }

        let fed = self.federation_state();
        let followers = fed.activity_store.get_followers("admin");
        if followers.is_empty() {
            return;
        }

        let actor_id = format!("{}/fed/actor/admin", fed.external_url);
        let object = object_fn(file_activity, &fed.external_url);

        let as_activity = ferro_server_activitypub::activity::Activity {
            context: serde_json::json!("https://www.w3.org/ns/activitystreams"),
            id: format!("{}/activities/{}", fed.external_url, uuid::Uuid::new_v4()),
            r#type: match activity_type {
                "Create" => ferro_server_activitypub::activity::ActivityType::Create,
                "Update" => ferro_server_activitypub::activity::ActivityType::Update,
                "Delete" => ferro_server_activitypub::activity::ActivityType::Delete,
                _ => ferro_server_activitypub::activity::ActivityType::Create,
            },
            actor: actor_id.clone(),
            object,
            to: Some(
                followers
                    .iter()
                    .map(|f| format!("{}/inbox", f.trim_end_matches('/')))
                    .collect(),
            ),
            cc: None,
            published: Utc::now().to_rfc3339(),
            target: None,
        };

        let activity_value = match serde_json::to_value(&as_activity) {
            Ok(v) => v,
            Err(e) => {
                warn!("Failed to serialize {} activity: {}", activity_type, e);
                return;
            }
        };

        fed.activity_store.add_to_outbox(as_activity).ok();

        let results =
            ferro_server_activitypub::delivery::deliver_to_followers(&fed, &activity_value).await;
        let ok_count = results.iter().filter(|r| r.is_ok()).count();
        info!(
            "Published {} activity for {} to {}/{} followers",
            activity_type,
            file_activity.path,
            ok_count,
            results.len()
        );
    }

    pub async fn apply_inbound_activity(
        &self,
        activity: &ferro_server_activitypub::activity::Activity,
    ) -> Result<(), String> {
        match activity.r#type {
            ferro_server_activitypub::activity::ActivityType::Create
            | ferro_server_activitypub::activity::ActivityType::Update => {
                let file_sync: FileSyncActivity =
                    serde_json::from_value(activity.object.clone())
                        .map_err(|e| format!("Failed to parse file sync activity object: {}", e))?;

                if let Some(url) = file_sync
                    .to_create_object(self.state.external_url())
                    .get("url")
                    .and_then(|v| v.as_str())
                {
                    info!(
                        "Received activity for file {} from {}",
                        file_sync.path, activity.actor
                    );

                    if let Ok(meta) = self.state.storage().head(&file_sync.path).await {
                        let dominated = file_sync
                            .checksum
                            .as_ref()
                            .is_some_and(|cs| meta.content_hash.as_str() == cs)
                            || meta.size == file_sync.size.unwrap_or(0);
                        if dominated {
                            return Ok(());
                        }
                    }

                    info!(
                        "Applying remote file {} to local storage from {}",
                        file_sync.path, url
                    );
                }
            }
            ferro_server_activitypub::activity::ActivityType::Delete => {
                let file_sync: FileSyncActivity =
                    serde_json::from_value(activity.object.clone())
                        .map_err(|e| format!("Failed to parse file sync delete object: {}", e))?;

                info!(
                    "Received Delete for file {} from {}",
                    file_sync.path, activity.actor
                );

                match self.state.storage().delete(&file_sync.path).await {
                    Ok(()) => {
                        info!("Applied remote delete for {}", file_sync.path);
                    }
                    Err(e) => {
                        warn!(
                            "Failed to apply remote delete for {}: {}",
                            file_sync.path, e
                        );
                    }
                }
            }
            _ => {
                return Err(format!(
                    "Unhandled activity type for file sync: {:?}",
                    activity.r#type
                ));
            }
        }
        Ok(())
    }
}

pub async fn start_federation_sync<S: InfraState>(state: Arc<S>) {
    if state.federation_secret().is_empty() {
        info!("Federation sync disabled (no secret configured)");
        return;
    }

    let sync = FederationSync::new(state);
    let fed_state = sync.federation_state();

    let activity_store = fed_state.activity_store.clone();
    let recent_activities = activity_store.get_inbox(0, 10);
    for activity in &recent_activities {
        if matches!(
            activity.r#type,
            ferro_server_activitypub::activity::ActivityType::Create
                | ferro_server_activitypub::activity::ActivityType::Update
                | ferro_server_activitypub::activity::ActivityType::Delete
        ) && let Err(e) = sync.apply_inbound_activity(activity).await
        {
            warn!("Failed to apply inbound activity {}: {}", activity.id, e);
        }
    }

    info!("Federation sync started");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_sync_activity_to_create_object() {
        let activity = FileSyncActivity {
            path: "/docs/readme.md".to_string(),
            user_id: "alice".to_string(),
            size: Some(1024),
            content_type: Some("text/markdown".to_string()),
            checksum: Some("abc123".to_string()),
        };

        let obj = activity.to_create_object("https://files.example.com");
        assert_eq!(obj["type"], "Document");
        assert_eq!(obj["id"], "https://files.example.com/files//docs/readme.md");
        assert_eq!(obj["size"], 1024);
    }

    #[test]
    fn test_file_sync_activity_to_delete_object() {
        let activity = FileSyncActivity {
            path: "/docs/readme.md".to_string(),
            user_id: "alice".to_string(),
            size: None,
            content_type: None,
            checksum: None,
        };

        let obj = activity.to_delete_object("https://files.example.com");
        assert_eq!(obj["type"], "Document");
        assert_eq!(obj["id"], "https://files.example.com/files//docs/readme.md");
        assert!(obj.get("url").is_none());
    }
}
