use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// A group of users for organizing share permissions and access control.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub members: Vec<String>,
    pub created_by: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateGroupRequest {
    pub name: String,
    pub description: Option<String>,
    pub members: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateGroupRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[async_trait]
pub trait GroupStoreTrait: Send + Sync {
    async fn create(&self, req: CreateGroupRequest, created_by: String) -> Group;
    async fn get(&self, id: &str) -> Option<Group>;
    async fn get_by_name(&self, name: &str) -> Option<Group>;
    async fn update(&self, id: &str, req: UpdateGroupRequest) -> Option<Group>;
    async fn delete(&self, id: &str) -> bool;
    async fn list(&self) -> Vec<Group>;
    async fn add_member(&self, id: &str, username: &str) -> bool;
    async fn remove_member(&self, id: &str, username: &str) -> bool;
    async fn is_member(&self, id: &str, username: &str) -> bool;
    async fn list_user_groups(&self, username: &str) -> Vec<Group>;
}

pub struct GroupStore {
    groups: Arc<RwLock<Vec<Group>>>,
}

impl GroupStore {
    pub fn new() -> Self {
        Self {
            groups: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn load_group(&self, group: Group) {
        self.groups.write().await.push(group);
    }

    pub fn load_groups_blocking(&self, groups: Vec<Group>) {
        tokio::task::block_in_place(|| {
            let mut guard = self.groups.blocking_write();
            for group in groups {
                guard.push(group);
            }
        });
    }
}

impl Default for GroupStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl GroupStoreTrait for GroupStore {
    async fn create(&self, req: CreateGroupRequest, created_by: String) -> Group {
        let id = uuid::Uuid::new_v4().to_string();
        let members = req.members.unwrap_or_default();

        let group = Group {
            id: id.clone(),
            name: req.name,
            description: req.description,
            members,
            created_by,
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        self.groups.write().await.push(group.clone());
        group
    }

    async fn get(&self, id: &str) -> Option<Group> {
        let groups = self.groups.read().await;
        groups.iter().find(|g| g.id == id).cloned()
    }

    async fn get_by_name(&self, name: &str) -> Option<Group> {
        let groups = self.groups.read().await;
        groups.iter().find(|g| g.name == name).cloned()
    }

    async fn update(&self, id: &str, req: UpdateGroupRequest) -> Option<Group> {
        let mut groups = self.groups.write().await;
        if let Some(group) = groups.iter_mut().find(|g| g.id == id) {
            if let Some(name) = req.name {
                group.name = name;
            }
            if let Some(desc) = req.description {
                group.description = Some(desc);
            }
            Some(group.clone())
        } else {
            None
        }
    }

    async fn delete(&self, id: &str) -> bool {
        let mut groups = self.groups.write().await;
        if let Some(pos) = groups.iter().position(|g| g.id == id) {
            groups.remove(pos);
            true
        } else {
            false
        }
    }

    async fn list(&self) -> Vec<Group> {
        let groups = self.groups.read().await;
        groups.clone()
    }

    async fn add_member(&self, id: &str, username: &str) -> bool {
        let mut groups = self.groups.write().await;
        if let Some(group) = groups.iter_mut().find(|g| g.id == id) {
            if !group.members.contains(&username.to_string()) {
                group.members.push(username.to_string());
            }
            true
        } else {
            false
        }
    }

    async fn remove_member(&self, id: &str, username: &str) -> bool {
        let mut groups = self.groups.write().await;
        if let Some(group) = groups.iter_mut().find(|g| g.id == id)
            && let Some(pos) = group.members.iter().position(|m| m == username)
        {
            group.members.remove(pos);
            return true;
        }
        false
    }

    async fn is_member(&self, id: &str, username: &str) -> bool {
        let groups = self.groups.read().await;
        groups
            .iter()
            .find(|g| g.id == id)
            .map(|g| g.members.contains(&username.to_string()))
            .unwrap_or(false)
    }

    async fn list_user_groups(&self, username: &str) -> Vec<Group> {
        let groups = self.groups.read().await;
        groups
            .iter()
            .filter(|g| g.members.contains(&username.to_string()))
            .cloned()
            .collect()
    }
}

/// Check if a user has access to a share via group membership.
/// Returns true if the user is a direct member of any of the allowed groups.
pub async fn check_group_share_access(
    group_store: &dyn GroupStoreTrait,
    allowed_group_ids: &[String],
    username: &str,
) -> bool {
    for group_id in allowed_group_ids {
        if group_store.is_member(group_id, username).await {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_group() {
        let store = GroupStore::new();
        let req = CreateGroupRequest {
            name: "engineering".to_string(),
            description: Some("Engineering team".to_string()),
            members: Some(vec!["alice".to_string(), "bob".to_string()]),
        };
        let group = store.create(req, "admin".to_string()).await;
        assert_eq!(group.name, "engineering");
        assert_eq!(group.members.len(), 2);
        assert!(group.members.contains(&"alice".to_string()));
        assert!(group.members.contains(&"bob".to_string()));
    }

    #[tokio::test]
    async fn test_get_group() {
        let store = GroupStore::new();
        let req = CreateGroupRequest {
            name: "test".to_string(),
            description: None,
            members: None,
        };
        let group = store.create(req, "admin".to_string()).await;
        let found = store.get(&group.id).await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "test");
    }

    #[tokio::test]
    async fn test_get_by_name() {
        let store = GroupStore::new();
        let req = CreateGroupRequest {
            name: "mygroup".to_string(),
            description: None,
            members: None,
        };
        store.create(req, "admin".to_string()).await;
        let found = store.get_by_name("mygroup").await;
        assert!(found.is_some());
    }

    #[tokio::test]
    async fn test_update_group() {
        let store = GroupStore::new();
        let req = CreateGroupRequest {
            name: "old_name".to_string(),
            description: None,
            members: None,
        };
        let group = store.create(req, "admin".to_string()).await;
        let updated = store
            .update(
                &group.id,
                UpdateGroupRequest {
                    name: Some("new_name".to_string()),
                    description: Some("Updated".to_string()),
                },
            )
            .await;
        assert!(updated.is_some());
        let updated = updated.unwrap();
        assert_eq!(updated.name, "new_name");
        assert_eq!(updated.description, Some("Updated".to_string()));
    }

    #[tokio::test]
    async fn test_delete_group() {
        let store = GroupStore::new();
        let req = CreateGroupRequest {
            name: "to_delete".to_string(),
            description: None,
            members: None,
        };
        let group = store.create(req, "admin".to_string()).await;
        assert!(store.delete(&group.id).await);
        assert!(store.get(&group.id).await.is_none());
    }

    #[tokio::test]
    async fn test_add_remove_member() {
        let store = GroupStore::new();
        let req = CreateGroupRequest {
            name: "team".to_string(),
            description: None,
            members: None,
        };
        let group = store.create(req, "admin".to_string()).await;
        assert!(store.add_member(&group.id, "alice").await);
        assert!(store.is_member(&group.id, "alice").await);
        assert!(store.remove_member(&group.id, "alice").await);
        assert!(!store.is_member(&group.id, "alice").await);
    }

    #[tokio::test]
    async fn test_list_user_groups() {
        let store = GroupStore::new();
        store
            .create(
                CreateGroupRequest {
                    name: "group1".to_string(),
                    description: None,
                    members: Some(vec!["alice".to_string()]),
                },
                "admin".to_string(),
            )
            .await;
        store
            .create(
                CreateGroupRequest {
                    name: "group2".to_string(),
                    description: None,
                    members: Some(vec!["bob".to_string()]),
                },
                "admin".to_string(),
            )
            .await;
        let groups = store.list_user_groups("alice").await;
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "group1");
    }

    #[tokio::test]
    async fn test_check_group_share_access() {
        let store = GroupStore::new();
        let group = store
            .create(
                CreateGroupRequest {
                    name: "allowed".to_string(),
                    description: None,
                    members: Some(vec!["alice".to_string()]),
                },
                "admin".to_string(),
            )
            .await;
        assert!(check_group_share_access(&store, std::slice::from_ref(&group.id), "alice").await);
        assert!(!check_group_share_access(&store, &[group.id], "bob").await);
    }
}
