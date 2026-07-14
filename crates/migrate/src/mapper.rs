use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::db::{NcFileCache, NcShare, NcSystemTag, NcUser};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FerroUser {
    pub username: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FerroShare {
    pub path: String,
    pub share_type: FerroShareType,
    pub shared_with: Option<String>,
    pub owner: String,
    pub permissions: FerroPermissions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FerroShareType {
    User,
    Group,
    Link,
    Remote,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FerroPermissions {
    pub read: bool,
    pub write: bool,
    pub share: bool,
    pub delete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FerroTag {
    pub name: String,
    pub file_ids: Vec<i64>,
}

pub fn map_user(nc: &NcUser) -> FerroUser {
    FerroUser {
        username: nc.uid.clone(),
        email: nc.email.clone(),
        display_name: nc.display_name.clone(),
        role: "user".to_string(),
    }
}

pub fn map_share(nc: &NcShare, file_path: &str) -> FerroShare {
    FerroShare {
        path: file_path.to_string(),
        share_type: match nc.share_type {
            0 => FerroShareType::User,
            1 => FerroShareType::Group,
            3 => FerroShareType::Link,
            6 => FerroShareType::Remote,
            _ => FerroShareType::User,
        },
        shared_with: nc.share_with.clone(),
        owner: nc.uid_owner.clone(),
        permissions: map_permissions(nc.permissions),
    }
}

fn map_permissions(nc_perms: i64) -> FerroPermissions {
    FerroPermissions {
        read: (nc_perms & 1) != 0,
        write: (nc_perms & 2) != 0,
        share: (nc_perms & 16) != 0,
        delete: (nc_perms & 8) != 0,
    }
}

pub fn map_tags(tags: &[NcSystemTag], mappings: &[(i64, String, i64)]) -> Vec<FerroTag> {
    let mut tag_map: std::collections::HashMap<i64, String> = std::collections::HashMap::new();
    for t in tags {
        tag_map.insert(t.id, t.name.clone());
    }

    let mut result_map: std::collections::HashMap<String, Vec<i64>> = std::collections::HashMap::new();
    for (_, _, tag_id) in mappings {
        if let Some(name) = tag_map.get(tag_id) {
            result_map.entry(name.clone()).or_default().push(*tag_id);
        }
    }

    result_map
        .into_iter()
        .map(|(name, file_ids)| FerroTag { name, file_ids })
        .collect()
}

pub fn nc_path_to_ferro(nc_path: &str, username: &str) -> String {
    let stripped = nc_path
        .trim_start_matches('/')
        .trim_start_matches("files/")
        .trim_start_matches(&format!("files/{}/", username));

    format!("/{}", stripped)
}

pub fn nc_mtime_to_datetime(mtime: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(mtime, 0).unwrap_or_else(Utc::now)
}

pub fn should_skip_file(file: &NcFileCache, max_size: u64) -> bool {
    if file.path.is_empty() {
        return true;
    }
    if max_size > 0 && file.size as u64 > max_size {
        return true;
    }
    false
}
