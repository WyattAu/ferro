use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    #[serde(rename = "@context")]
    pub context: String,
    pub id: String,
    pub r#type: String,
    pub name: String,
    pub preferred_username: String,
    pub summary: Option<String>,
    pub inbox: String,
    pub outbox: String,
    pub followers: String,
    pub following: String,
    pub public_key: PublicKey,
    #[serde(rename = "icon")]
    pub icon: Option<Image>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicKey {
    pub id: String,
    pub owner: String,
    pub public_key_pem: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    pub r#type: String,
    pub media_type: String,
    pub url: String,
}

impl Actor {
    pub fn new(base_url: &str, username: &str, display_name: &str) -> Self {
        let actor_id = format!("{}/fed/actor/{}", base_url, username);
        Self {
            context: "https://www.w3.org/ns/activitystreams".to_string(),
            id: actor_id.clone(),
            r#type: "Service".to_string(),
            name: display_name.to_string(),
            preferred_username: username.to_string(),
            summary: Some(format!("Ferro file server - {}", display_name)),
            inbox: format!("{}/fed/inbox", actor_id),
            outbox: format!("{}/fed/outbox", actor_id),
            followers: format!("{}/fed/followers", actor_id),
            following: format!("{}/fed/following", actor_id),
            public_key: PublicKey {
                id: format!("{}#main-key", actor_id),
                owner: actor_id,
                public_key_pem: "TODO: generate RSA key pair".to_string(),
            },
            icon: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_actor_serialization() {
        let actor = Actor::new("https://files.example.com", "admin", "Admin");
        let json = serde_json::to_string(&actor).unwrap();
        assert!(json.contains("\"type\":\"Service\""));
        assert!(json.contains("\"inbox\":"));
        assert!(json.contains("\"outbox\":"));
    }

    #[test]
    fn test_webfinger_resource_parsing() {
        let resource = "acct:alice@files.example.com";
        let stripped = resource.strip_prefix("acct:").unwrap();
        let parts: Vec<&str> = stripped.splitn(2, '@').collect();
        assert_eq!(parts[0], "alice");
        assert_eq!(parts[1], "files.example.com");
    }
}
