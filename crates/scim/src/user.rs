use crate::schema::*;
use chrono::Utc;

pub fn to_scim_user(id: &str, username: &str, display_name: &str, email: &str, active: bool) -> ScimUser {
    ScimUser {
        schemas: vec!["urn:ietf:params:scim:schemas:core:2.0:User".into()],
        id: id.to_string(),
        external_id: None,
        user_name: username.to_string(),
        name: Some(ScimName {
            given_name: None,
            family_name: None,
            formatted: Some(display_name.to_string()),
        }),
        display_name: Some(display_name.to_string()),
        emails: if email.is_empty() {
            vec![]
        } else {
            vec![ScimEmail {
                value: email.to_string(),
                email_type: Some("work".into()),
                primary: true,
            }]
        },
        active,
        groups: vec![],
        meta: ScimMeta {
            resource_type: "User".into(),
            created: Utc::now(),
            last_modified: Utc::now(),
            location: format!("/scim/v2/Users/{}", id),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_scim_user_with_email() {
        let user = to_scim_user("u1", "testuser", "Test User", "test@example.com", true);
        assert_eq!(user.id, "u1");
        assert_eq!(user.user_name, "testuser");
        assert_eq!(user.display_name.as_deref(), Some("Test User"));
        assert!(user.active);
        assert_eq!(user.emails.len(), 1);
        assert_eq!(user.emails[0].value, "test@example.com");
        assert!(user.emails[0].primary);
        assert_eq!(user.emails[0].email_type.as_deref(), Some("work"));
    }

    #[test]
    fn test_to_scim_user_without_email() {
        let user = to_scim_user("u2", "nouser", "No Email", "", false);
        assert!(!user.active);
        assert!(user.emails.is_empty());
    }

    #[test]
    fn test_to_scim_user_schemas() {
        let user = to_scim_user("u1", "test", "Test", "a@b.com", true);
        assert_eq!(user.schemas.len(), 1);
        assert_eq!(user.schemas[0], "urn:ietf:params:scim:schemas:core:2.0:User");
    }

    #[test]
    fn test_to_scim_user_meta_location() {
        let user = to_scim_user("u1", "test", "Test", "a@b.com", true);
        assert_eq!(user.meta.location, "/scim/v2/Users/u1");
        assert_eq!(user.meta.resource_type, "User");
    }

    #[test]
    fn test_to_scim_user_name() {
        let user = to_scim_user("u1", "test", "Test User", "a@b.com", true);
        let name = user.name.unwrap();
        assert_eq!(name.formatted.as_deref(), Some("Test User"));
        assert!(name.given_name.is_none());
        assert!(name.family_name.is_none());
    }

    #[test]
    fn test_to_scim_user_external_id() {
        let user = to_scim_user("u1", "test", "Test", "a@b.com", true);
        assert!(user.external_id.is_none());
    }

    #[test]
    fn test_to_scim_user_groups_empty() {
        let user = to_scim_user("u1", "test", "Test", "a@b.com", true);
        assert!(user.groups.is_empty());
    }

    #[test]
    fn test_to_scim_user_serialize() {
        let user = to_scim_user("u1", "testuser", "Test User", "a@b.com", true);
        let json = serde_json::to_string(&user).unwrap();
        assert!(json.contains("testuser"));
        assert!(json.contains("Test User"));
    }
}
