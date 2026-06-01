//! Distributed consensus integration.
//!
//! Provides helpers for creating and managing Raft nodes.

use ferro_distributed::consensus::{NodeId, RaftNode};

pub fn create_raft_node(node_id: &str) -> RaftNode {
    RaftNode::new(NodeId(node_id.to_string()))
}

pub fn append_command(node: &mut RaftNode, command: Vec<u8>) -> u64 {
    node.append_entry(command)
}

pub fn start_election(node: &mut RaftNode) {
    node.become_candidate()
}

pub fn promote_to_leader(node: &mut RaftNode) {
    node.become_leader()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferro_distributed::consensus::ConsensusState;

    #[test]
    fn test_create_node() {
        let node = create_raft_node("node-1");
        assert_eq!(node.id.0, "node-1");
        assert_eq!(node.state, ConsensusState::Follower);
        assert_eq!(node.current_term.0, 0);
        assert!(node.log.is_empty());
    }

    #[test]
    fn test_append_command() {
        let mut node = create_raft_node("node-1");
        let idx = append_command(&mut node, b"set-key".to_vec());
        assert_eq!(idx, 1);
        assert_eq!(node.log.len(), 1);
    }

    #[test]
    fn test_election_cycle() {
        let mut node = create_raft_node("node-1");
        start_election(&mut node);
        assert_eq!(node.state, ConsensusState::Candidate);
        assert_eq!(node.current_term.0, 1);
        promote_to_leader(&mut node);
        assert_eq!(node.state, ConsensusState::Leader);
    }
}
