use crate::consensus::NodeId;
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum MemberState {
    Alive,
    Suspect,
    Dead,
    Left,
}

#[derive(Debug, Clone)]
pub struct ClusterMember {
    pub node_id: NodeId,
    pub address: String,
    pub state: MemberState,
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
    pub metadata: serde_json::Value,
}

pub trait MembershipStore: Send + Sync {
    fn add_member(&self, member: ClusterMember) -> Result<(), crate::error::DistributedError>;
    fn remove_member(&self, node_id: &NodeId) -> Result<(), crate::error::DistributedError>;
    fn get_member(&self, node_id: &NodeId) -> Result<Option<ClusterMember>, crate::error::DistributedError>;
    fn list_members(&self) -> Vec<ClusterMember>;
    fn update_heartbeat(&self, node_id: &NodeId) -> Result<(), crate::error::DistributedError>;
    fn mark_suspect(&self, node_id: &NodeId) -> Result<(), crate::error::DistributedError>;
    fn mark_dead(&self, node_id: &NodeId) -> Result<(), crate::error::DistributedError>;
}

pub struct InMemoryMembershipStore {
    members: DashMap<String, ClusterMember>,
}

impl InMemoryMembershipStore {
    pub fn new() -> Self {
        Self {
            members: DashMap::new(),
        }
    }
}

impl Default for InMemoryMembershipStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MembershipStore for InMemoryMembershipStore {
    fn add_member(&self, member: ClusterMember) -> Result<(), crate::error::DistributedError> {
        self.members.insert(member.node_id.0.clone(), member);
        Ok(())
    }

    fn remove_member(&self, node_id: &NodeId) -> Result<(), crate::error::DistributedError> {
        self.members.remove(&node_id.0);
        Ok(())
    }

    fn get_member(&self, node_id: &NodeId) -> Result<Option<ClusterMember>, crate::error::DistributedError> {
        Ok(self.members.get(&node_id.0).map(|m| m.value().clone()))
    }

    fn list_members(&self) -> Vec<ClusterMember> {
        self.members.iter().map(|m| m.value().clone()).collect()
    }

    fn update_heartbeat(&self, node_id: &NodeId) -> Result<(), crate::error::DistributedError> {
        if let Some(mut m) = self.members.get_mut(&node_id.0) {
            m.last_heartbeat = Utc::now();
            m.state = MemberState::Alive;
        }
        Ok(())
    }

    fn mark_suspect(&self, node_id: &NodeId) -> Result<(), crate::error::DistributedError> {
        if let Some(mut m) = self.members.get_mut(&node_id.0) {
            m.state = MemberState::Suspect;
        }
        Ok(())
    }

    fn mark_dead(&self, node_id: &NodeId) -> Result<(), crate::error::DistributedError> {
        if let Some(mut m) = self.members.get_mut(&node_id.0) {
            m.state = MemberState::Dead;
        }
        Ok(())
    }
}

pub struct FailureDetector {
    pub store: Arc<InMemoryMembershipStore>,
    pub suspect_timeout: Duration,
    pub dead_timeout: Duration,
}

impl FailureDetector {
    pub fn new(store: Arc<InMemoryMembershipStore>, suspect_timeout: Duration, dead_timeout: Duration) -> Self {
        Self {
            store,
            suspect_timeout,
            dead_timeout,
        }
    }

    pub fn check(&self) -> Vec<NodeId> {
        let now = Utc::now();
        let mut detected = Vec::new();

        for member in self.store.list_members() {
            if member.state == MemberState::Left {
                continue;
            }
            let elapsed = now.signed_duration_since(member.last_heartbeat);
            if elapsed >= chrono::Duration::from_std(self.dead_timeout).unwrap_or(chrono::Duration::zero()) {
                let _ = self.store.mark_dead(&member.node_id);
                detected.push(member.node_id.clone());
            } else if elapsed >= chrono::Duration::from_std(self.suspect_timeout).unwrap_or(chrono::Duration::zero())
                && member.state != MemberState::Suspect
            {
                let _ = self.store.mark_suspect(&member.node_id);
            }
        }

        detected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_member(id: &str) -> ClusterMember {
        ClusterMember {
            node_id: NodeId(id.into()),
            address: format!("http://{}:8080", id),
            state: MemberState::Alive,
            last_heartbeat: Utc::now(),
            metadata: serde_json::Value::Null,
        }
    }

    #[test]
    fn test_add_remove() {
        let store = InMemoryMembershipStore::new();
        let member = make_member("node-1");
        store.add_member(member.clone()).unwrap();

        let fetched = store.get_member(&NodeId("node-1".into())).unwrap().unwrap();
        assert_eq!(fetched.node_id.0, "node-1");

        store.remove_member(&NodeId("node-1".into())).unwrap();
        assert!(store.get_member(&NodeId("node-1".into())).unwrap().is_none());
    }

    #[test]
    fn test_heartbeat_update() {
        let store = InMemoryMembershipStore::new();
        let mut member = make_member("node-1");
        member.state = MemberState::Suspect;
        store.add_member(member).unwrap();

        store.update_heartbeat(&NodeId("node-1".into())).unwrap();
        let fetched = store.get_member(&NodeId("node-1".into())).unwrap().unwrap();
        assert_eq!(fetched.state, MemberState::Alive);
    }

    #[test]
    fn test_suspect_detection() {
        let store = Arc::new(InMemoryMembershipStore::new());
        let mut member = make_member("node-1");
        member.last_heartbeat = Utc::now() - chrono::Duration::seconds(30);
        store.add_member(member).unwrap();

        let detector = FailureDetector::new(store.clone(), Duration::from_secs(10), Duration::from_secs(60));

        let detected = detector.check();
        assert!(detected.is_empty());

        let fetched = store.get_member(&NodeId("node-1".into())).unwrap().unwrap();
        assert_eq!(fetched.state, MemberState::Suspect);
    }

    #[test]
    fn test_dead_detection() {
        let store = Arc::new(InMemoryMembershipStore::new());
        let mut member = make_member("node-1");
        member.last_heartbeat = Utc::now() - chrono::Duration::seconds(120);
        store.add_member(member).unwrap();

        let detector = FailureDetector::new(store.clone(), Duration::from_secs(10), Duration::from_secs(60));

        let detected = detector.check();
        assert_eq!(detected.len(), 1);
        assert_eq!(detected[0].0, "node-1");

        let fetched = store.get_member(&NodeId("node-1".into())).unwrap().unwrap();
        assert_eq!(fetched.state, MemberState::Dead);
    }

    #[test]
    fn test_list_filtering() {
        let store = InMemoryMembershipStore::new();
        store.add_member(make_member("node-1")).unwrap();
        store.add_member(make_member("node-2")).unwrap();

        let members = store.list_members();
        assert_eq!(members.len(), 2);

        store.remove_member(&NodeId("node-1".into())).unwrap();
        let members = store.list_members();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].node_id.0, "node-2");
    }
}
