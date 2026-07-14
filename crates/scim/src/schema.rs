use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScimMeta {
    #[serde(rename = "resourceType")]
    pub resource_type: String,
    pub created: DateTime<Utc>,
    #[serde(rename = "lastModified")]
    pub last_modified: DateTime<Utc>,
    pub location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScimName {
    #[serde(rename = "givenName", skip_serializing_if = "Option::is_none")]
    pub given_name: Option<String>,
    #[serde(rename = "familyName", skip_serializing_if = "Option::is_none")]
    pub family_name: Option<String>,
    #[serde(rename = "formatted", skip_serializing_if = "Option::is_none")]
    pub formatted: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScimEmail {
    pub value: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub email_type: Option<String>,
    pub primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScimUserRef {
    pub value: String,
    #[serde(rename = "$ref", skip_serializing_if = "Option::is_none")]
    pub ref_url: Option<String>,
    pub display: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScimUser {
    pub schemas: Vec<String>,
    pub id: String,
    #[serde(rename = "externalId", skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    #[serde(rename = "userName")]
    pub user_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<ScimName>,
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub emails: Vec<ScimEmail>,
    pub active: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<ScimUserRef>,
    pub meta: ScimMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScimGroup {
    pub schemas: Vec<String>,
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<ScimUserRef>,
    pub meta: ScimMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScimListResponse<T> {
    pub schemas: Vec<String>,
    #[serde(rename = "totalResults")]
    pub total_results: u32,
    #[serde(rename = "startIndex")]
    pub start_index: u32,
    #[serde(rename = "itemsPerPage")]
    pub items_per_page: u32,
    pub resources: Vec<T>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_meta() -> ScimMeta {
        ScimMeta {
            resource_type: "User".into(),
            created: Utc::now(),
            last_modified: Utc::now(),
            location: "/scim/v2/Users/test".into(),
        }
    }

    #[test]
    fn test_scim_user_serialize() {
        let user = ScimUser {
            schemas: vec!["urn:ietf:params:scim:schemas:core:2.0:User".into()],
            id: "u1".into(),
            external_id: None,
            user_name: "testuser".into(),
            name: None,
            display_name: Some("Test User".into()),
            emails: vec![],
            active: true,
            groups: vec![],
            meta: make_meta(),
        };
        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("userName"));
        assert!(json.contains("testuser"));
    }

    #[test]
    fn test_scim_user_deserialize() {
        let json = r#"{
            "schemas": ["urn:ietf:params:scim:schemas:core:2.0:User"],
            "id": "u1",
            "userName": "testuser",
            "active": true,
            "emails": [],
            "groups": [],
            "meta": {
                "resourceType": "User",
                "created": "2024-01-01T00:00:00Z",
                "lastModified": "2024-01-01T00:00:00Z",
                "location": "/scim/v2/Users/u1"
            }
        }"#;
        let user: ScimUser = serde_json::from_str(json).unwrap();
        assert_eq!(user.user_name, "testuser");
        assert!(user.active);
    }

    #[test]
    fn test_scim_group_serialize() {
        let group = ScimGroup {
            schemas: vec!["urn:ietf:params:scim:schemas:core:2.0:Group".into()],
            id: "g1".into(),
            display_name: "Admins".into(),
            members: vec![],
            meta: ScimMeta {
                resource_type: "Group".into(),
                created: Utc::now(),
                last_modified: Utc::now(),
                location: "/scim/v2/Groups/g1".into(),
            },
        };
        let json = serde_json::to_string(&group).unwrap();
        assert!(json.contains("displayName"));
        assert!(json.contains("Admins"));
    }

    #[test]
    fn test_scim_list_response() {
        let response = ScimListResponse {
            schemas: vec!["urn:ietf:params:scim:api:messages:2.0:ListResponse".into()],
            total_results: 2,
            start_index: 1,
            items_per_page: 10,
            resources: vec!["item1".to_string(), "item2".to_string()],
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("totalResults"));
        assert!(json.contains("startIndex"));
        assert!(json.contains("itemsPerPage"));
    }

    #[test]
    fn test_scim_name_serialize() {
        let name = ScimName {
            given_name: Some("John".into()),
            family_name: Some("Doe".into()),
            formatted: Some("John Doe".into()),
        };
        let json = serde_json::to_string(&name).unwrap();
        assert!(json.contains("givenName"));
        assert!(json.contains("familyName"));
    }

    #[test]
    fn test_scim_email_serialize() {
        let email = ScimEmail {
            value: "test@example.com".into(),
            email_type: Some("work".into()),
            primary: true,
        };
        let json = serde_json::to_string(&email).unwrap();
        assert!(json.contains("test@example.com"));
        assert!(json.contains("work"));
    }

    #[test]
    fn test_scim_user_ref_serialize() {
        let user_ref = ScimUserRef {
            value: "u1".into(),
            ref_url: Some("/scim/v2/Users/u1".into()),
            display: Some("Test".into()),
        };
        let json = serde_json::to_string(&user_ref).unwrap();
        assert!(json.contains("$ref"));
    }

    #[test]
    fn test_scim_meta_debug() {
        let meta = make_meta();
        let debug = format!("{:?}", meta);
        assert!(debug.contains("ScimMeta"));
    }

    #[test]
    fn test_scim_user_debug() {
        let user = ScimUser {
            schemas: vec![],
            id: "u1".into(),
            external_id: None,
            user_name: "test".into(),
            name: None,
            display_name: None,
            emails: vec![],
            active: true,
            groups: vec![],
            meta: make_meta(),
        };
        let debug = format!("{:?}", user);
        assert!(debug.contains("ScimUser"));
    }
}
