use crate::error::ScimError;
use crate::schema::*;
use dashmap::DashMap;

#[derive(Clone, Default)]
pub struct GroupStore {
    groups: DashMap<String, ScimGroup>,
}

impl GroupStore {
    pub fn new() -> Self {
        Self {
            groups: DashMap::new(),
        }
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
