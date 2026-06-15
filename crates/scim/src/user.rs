use crate::schema::*;
use chrono::Utc;

pub fn to_scim_user(
    id: &str,
    username: &str,
    display_name: &str,
    email: &str,
    active: bool,
) -> ScimUser {
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
