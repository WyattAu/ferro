use crate::error::ScimError;
use crate::schema::*;
use dashmap::DashMap;

#[derive(Clone, Default)]
pub struct GroupStore {
    groups: DashMap<String, ScimGroup>,
}

impl GroupStore {
    pub fn new() -> Self {
        Self { groups: DashMap::new() }
    }

    pub fn list(&self, start: u32, count: u32) -> Vec<ScimGroup> {
        self.groups
            .iter()
            .skip(start as usize)
            .take(count as usize)
            .map(|e| e.value().clone())
            .collect()
    }

    pub fn get(&self, id: &str) -> Result<ScimGroup, ScimError> {
        self.groups
            .get(id)
            .map(|e| e.value().clone())
            .ok_or(ScimError::NotFound)
    }

    pub fn create(&self, group: ScimGroup) -> Result<ScimGroup, ScimError> {
        self.groups.insert(group.id.clone(), group.clone());
        Ok(group)
    }

    pub fn update(&self, id: &str, group: ScimGroup) -> Result<ScimGroup, ScimError> {
        self.groups.insert(id.to_string(), group.clone());
        Ok(group)
    }

    pub fn delete(&self, id: &str) -> Result<(), ScimError> {
        self.groups.remove(id).ok_or(ScimError::NotFound)?;
        Ok(())
    }

    pub fn count(&self) -> u32 {
        self.groups.len() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_group(id: &str, name: &str) -> ScimGroup {
        ScimGroup {
            schemas: vec!["urn:ietf:params:scim:schemas:core:2.0:Group".into()],
            id: id.to_string(),
            display_name: name.to_string(),
            members: vec![],
            meta: ScimMeta {
                resource_type: "Group".into(),
                created: chrono::Utc::now(),
                last_modified: chrono::Utc::now(),
                location: format!("/scim/v2/Groups/{}", id),
            },
        }
    }

    #[test]
    fn test_create_group() {
        let store = GroupStore::new();
        let group = make_group("g1", "Admins");
        let result = store.create(group).unwrap();
        assert_eq!(result.id, "g1");
        assert_eq!(result.display_name, "Admins");
    }

    #[test]
    fn test_get_group() {
        let store = GroupStore::new();
        store.create(make_group("g1", "Admins")).unwrap();
        let group = store.get("g1").unwrap();
        assert_eq!(group.id, "g1");
    }

    #[test]
    fn test_get_group_not_found() {
        let store = GroupStore::new();
        assert!(store.get("nonexistent").is_err());
    }

    #[test]
    fn test_list_groups() {
        let store = GroupStore::new();
        store.create(make_group("g1", "Admins")).unwrap();
        store.create(make_group("g2", "Users")).unwrap();
        let groups = store.list(0, 10);
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn test_list_groups_with_pagination() {
        let store = GroupStore::new();
        store.create(make_group("g1", "Admins")).unwrap();
        store.create(make_group("g2", "Users")).unwrap();
        let groups = store.list(0, 1);
        assert_eq!(groups.len(), 1);
    }

    #[test]
    fn test_update_group() {
        let store = GroupStore::new();
        store.create(make_group("g1", "Admins")).unwrap();
        let updated = store.update("g1", make_group("g1", "Super Admins")).unwrap();
        assert_eq!(updated.display_name, "Super Admins");
    }

    #[test]
    fn test_delete_group() {
        let store = GroupStore::new();
        store.create(make_group("g1", "Admins")).unwrap();
        store.delete("g1").unwrap();
        assert!(store.get("g1").is_err());
    }

    #[test]
    fn test_delete_group_not_found() {
        let store = GroupStore::new();
        assert!(store.delete("nonexistent").is_err());
    }

    #[test]
    fn test_count() {
        let store = GroupStore::new();
        assert_eq!(store.count(), 0);
        store.create(make_group("g1", "Admins")).unwrap();
        assert_eq!(store.count(), 1);
    }

    #[test]
    fn test_default() {
        let store = GroupStore::default();
        assert_eq!(store.count(), 0);
    }

    #[test]
    fn test_create_and_overwrite() {
        let store = GroupStore::new();
        store.create(make_group("g1", "Admins")).unwrap();
        store.create(make_group("g1", "Updated")).unwrap();
        let group = store.get("g1").unwrap();
        assert_eq!(group.display_name, "Updated");
    }

    #[test]
    fn test_scim_group_debug() {
        let group = make_group("g1", "Test");
        let debug = format!("{:?}", group);
        assert!(debug.contains("Test"));
    }

    #[test]
    fn test_scim_group_clone() {
        let group = make_group("g1", "Clone");
        let cloned = group.clone();
        assert_eq!(cloned.id, "g1");
    }
}
