use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    #[serde(rename = "@context")]
    pub context: serde_json::Value,
    pub id: String,
    pub r#type: ActivityType,
    pub actor: String,
    pub object: serde_json::Value,
    pub to: Option<Vec<String>>,
    pub cc: Option<Vec<String>>,
    pub published: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum ActivityType {
    Create,
    Update,
    Delete,
    Announce,
    Follow,
    Accept,
    Reject,
    Like,
    Undo,
}

impl Activity {
    pub fn create(actor: &str, object: serde_json::Value) -> Self {
        Self {
            context: serde_json::json!("https://www.w3.org/ns/activitystreams"),
            id: format!("{}/activities/{}", actor, uuid::Uuid::new_v4()),
            r#type: ActivityType::Create,
            actor: actor.to_string(),
            object,
            to: Some(vec!["https://www.w3.org/ns/activitystreams#Public".to_string()]),
            cc: None,
            published: Utc::now().to_rfc3339(),
            target: None,
        }
    }

    pub fn announce(actor: &str, object: serde_json::Value, target: &str) -> Self {
        Self {
            context: serde_json::json!("https://www.w3.org/ns/activitystreams"),
            id: format!("{}/activities/{}", actor, uuid::Uuid::new_v4()),
            r#type: ActivityType::Announce,
            actor: actor.to_string(),
            object,
            to: Some(vec!["https://www.w3.org/ns/activitystreams#Public".to_string()]),
            cc: None,
            published: Utc::now().to_rfc3339(),
            target: Some(serde_json::json!(target)),
        }
    }

    pub fn follow(actor: &str, target: &str) -> Self {
        Self {
            context: serde_json::json!("https://www.w3.org/ns/activitystreams"),
            id: format!("{}/activities/{}", actor, uuid::Uuid::new_v4()),
            r#type: ActivityType::Follow,
            actor: actor.to_string(),
            object: serde_json::json!(target),
            to: Some(vec![target.to_string()]),
            cc: None,
            published: Utc::now().to_rfc3339(),
            target: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activity_serialization() {
        let activity = Activity::create(
            "https://example.com/actor/alice",
            serde_json::json!({"type": "Note", "content": "hello"}),
        );
        let json = serde_json::to_string(&activity).unwrap();
        assert!(json.contains("\"type\":\"Create\""));
        assert!(json.contains("\"actor\":\"https://example.com/actor/alice\""));
    }

    #[test]
    fn test_activity_deserialization() {
        let json = r#"{
            "@context": "https://www.w3.org/ns/activitystreams",
            "id": "https://example.com/activities/1",
            "type": "Like",
            "actor": "https://example.com/actor/bob",
            "object": "https://example.com/notes/1",
            "to": ["https://www.w3.org/ns/activitystreams#Public"],
            "published": "2024-01-01T00:00:00+00:00"
        }"#;
        let activity: Activity = serde_json::from_str(json).unwrap();
        assert_eq!(activity.r#type, ActivityType::Like);
        assert_eq!(activity.actor, "https://example.com/actor/bob");
    }

    #[test]
    fn test_activity_create() {
        let activity = Activity::create(
            "https://example.com/actor/alice",
            serde_json::json!({"type": "Document"}),
        );
        assert_eq!(activity.r#type, ActivityType::Create);
        assert!(activity.id.starts_with("https://example.com/actor/alice/activities/"));
        assert!(activity.target.is_none());
        assert!(activity.to.is_some());
    }

    #[test]
    fn test_activity_announce() {
        let activity = Activity::announce(
            "https://example.com/actor/alice",
            serde_json::json!({"type": "Document", "id": "https://other.com/files/1"}),
            "https://example.com/actor/alice",
        );
        assert_eq!(activity.r#type, ActivityType::Announce);
        assert!(activity.target.is_some());
        let target = activity.target.unwrap();
        assert_eq!(target.as_str().unwrap(), "https://example.com/actor/alice");
    }

    #[test]
    fn test_activity_follow() {
        let activity = Activity::follow("https://example.com/actor/alice", "https://other.com/actor/bob");
        assert_eq!(activity.r#type, ActivityType::Follow);
        assert_eq!(activity.object.as_str().unwrap(), "https://other.com/actor/bob");
        let to = activity.to.unwrap();
        assert_eq!(to[0], "https://other.com/actor/bob");
    }

    #[test]
    fn test_activity_type_roundtrip() {
        let types = vec![
            ActivityType::Create,
            ActivityType::Update,
            ActivityType::Delete,
            ActivityType::Announce,
            ActivityType::Follow,
            ActivityType::Accept,
            ActivityType::Reject,
            ActivityType::Like,
            ActivityType::Undo,
        ];
        for t in types {
            let json = serde_json::to_string(&t).unwrap();
            let parsed: ActivityType = serde_json::from_str(&json).unwrap();
            assert_eq!(t, parsed);
        }
    }
}
