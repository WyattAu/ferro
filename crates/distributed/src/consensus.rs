use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct NodeId(pub String);

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct Term(pub u64);

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LogEntry {
    pub term: Term,
    pub index: u64,
    pub command: Vec<u8>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VoteRequest {
    pub term: Term,
    pub candidate_id: NodeId,
    pub last_log_index: u64,
    pub last_log_term: Term,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VoteResponse {
    pub term: Term,
    pub vote_granted: bool,
    pub voter_id: NodeId,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppendEntriesRequest {
    pub term: Term,
    pub leader_id: NodeId,
    pub prev_log_index: u64,
    pub prev_log_term: Term,
    pub entries: Vec<LogEntry>,
    pub leader_commit: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppendEntriesResponse {
    pub term: Term,
    pub success: bool,
    pub match_index: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConsensusState {
    Follower,
    Candidate,
    Leader,
}

pub struct RaftNode {
    pub id: NodeId,
    pub current_term: Term,
    pub voted_for: Option<NodeId>,
    pub log: Vec<LogEntry>,
    pub commit_index: u64,
    pub state: ConsensusState,
    pub peers: Vec<NodeId>,
    pub election_timeout: Duration,
    pub heartbeat_interval: Duration,
    votes_received: Vec<NodeId>,
}

impl RaftNode {
    pub fn new(id: NodeId) -> Self {
        Self {
            id,
            current_term: Term(0),
            voted_for: None,
            log: Vec::new(),
            commit_index: 0,
            state: ConsensusState::Follower,
            peers: Vec::new(),
            election_timeout: Duration::from_millis(150),
            heartbeat_interval: Duration::from_millis(50),
            votes_received: Vec::new(),
        }
    }

    pub fn become_candidate(&mut self) {
        self.current_term = Term(self.current_term.0 + 1);
        self.voted_for = Some(self.id.clone());
        self.votes_received = vec![self.id.clone()];
        self.state = ConsensusState::Candidate;
    }

    pub fn become_leader(&mut self) {
        self.state = ConsensusState::Leader;
    }

    pub fn step_down(&mut self, new_term: Term) {
        self.current_term = new_term;
        self.voted_for = None;
        self.state = ConsensusState::Follower;
        self.votes_received.clear();
    }

    pub fn append_entry(&mut self, command: Vec<u8>) -> u64 {
        let index = (self.log.len() as u64) + 1;
        self.log.push(LogEntry {
            term: self.current_term,
            index,
            command,
        });
        index
    }

    pub fn request_vote(&self) -> VoteRequest {
        let last_log_index = self.log.len() as u64;
        let last_log_term = self.log.last().map(|e| e.term).unwrap_or(Term(0));
        VoteRequest {
            term: self.current_term,
            candidate_id: self.id.clone(),
            last_log_index,
            last_log_term,
        }
    }

    pub fn handle_vote_response(&mut self, resp: &VoteResponse) -> bool {
        if resp.term > self.current_term {
            self.step_down(resp.term);
            return false;
        }
        if resp.vote_granted
            && resp.term == self.current_term
            && self.state == ConsensusState::Candidate
        {
            if !self.votes_received.contains(&resp.voter_id) {
                self.votes_received.push(resp.voter_id.clone());
            }
            let total_nodes = self.peers.len() + 1;
            let majority = (total_nodes / 2) + 1;
            self.votes_received.len() >= majority
        } else {
            false
        }
    }

    pub fn handle_append_request(&mut self, req: &AppendEntriesRequest) -> AppendEntriesResponse {
        if req.term < self.current_term {
            return AppendEntriesResponse {
                term: self.current_term,
                success: false,
                match_index: None,
            };
        }

        if req.term > self.current_term {
            self.step_down(req.term);
        }

        if req.prev_log_index > 0 {
            let idx = (req.prev_log_index as usize).saturating_sub(1);
            if idx >= self.log.len() {
                return AppendEntriesResponse {
                    term: self.current_term,
                    success: false,
                    match_index: None,
                };
            }
            if self.log[idx].term != req.prev_log_term {
                return AppendEntriesResponse {
                    term: self.current_term,
                    success: false,
                    match_index: Some(self.log.len() as u64),
                };
            }
        }

        for entry in &req.entries {
            let idx = (entry.index as usize).saturating_sub(1);
            if idx < self.log.len() {
                if self.log[idx].term != entry.term {
                    self.log.truncate(idx);
                    self.log.push(entry.clone());
                }
            } else {
                self.log.push(entry.clone());
            }
        }

        if req.leader_commit > self.commit_index {
            self.commit_index = std::cmp::min(req.leader_commit, self.log.len() as u64);
        }

        AppendEntriesResponse {
            term: self.current_term,
            success: true,
            match_index: Some(self.log.len() as u64),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state_is_follower() {
        let node = RaftNode::new(NodeId("node-1".into()));
        assert_eq!(node.state, ConsensusState::Follower);
        assert_eq!(node.current_term, Term(0));
        assert!(node.voted_for.is_none());
        assert!(node.log.is_empty());
    }

    #[test]
    fn test_candidate_increments_term() {
        let mut node = RaftNode::new(NodeId("node-1".into()));
        node.become_candidate();
        assert_eq!(node.state, ConsensusState::Candidate);
        assert_eq!(node.current_term, Term(1));
        assert_eq!(node.voted_for, Some(NodeId("node-1".into())));
    }

    #[test]
    fn test_leader_transition() {
        let mut node = RaftNode::new(NodeId("node-1".into()));
        node.become_candidate();
        node.become_leader();
        assert_eq!(node.state, ConsensusState::Leader);
    }

    #[test]
    fn test_step_down_reverts_to_follower() {
        let mut node = RaftNode::new(NodeId("node-1".into()));
        node.become_candidate();
        node.step_down(Term(5));
        assert_eq!(node.state, ConsensusState::Follower);
        assert_eq!(node.current_term, Term(5));
        assert!(node.voted_for.is_none());
    }

    #[test]
    fn test_append_entry() {
        let mut node = RaftNode::new(NodeId("node-1".into()));
        let idx = node.append_entry(b"set x=1".to_vec());
        assert_eq!(idx, 1);
        assert_eq!(node.log.len(), 1);
        assert_eq!(node.log[0].index, 1);
        assert_eq!(node.log[0].command, b"set x=1");
    }

    #[test]
    fn test_vote_counting() {
        let mut node = RaftNode::new(NodeId("node-1".into()));
        node.peers = vec![NodeId("node-2".into()), NodeId("node-3".into())];
        node.become_candidate();

        let resp1 = VoteResponse {
            term: Term(1),
            vote_granted: true,
            voter_id: NodeId("node-2".into()),
        };
        let won = node.handle_vote_response(&resp1);
        assert!(won);

        let resp2 = VoteResponse {
            term: Term(1),
            vote_granted: true,
            voter_id: NodeId("node-2".into()),
        };
        let won_again = node.handle_vote_response(&resp2);
        assert!(won_again);
    }

    #[test]
    fn test_append_entries_log_matching() {
        let mut leader = RaftNode::new(NodeId("node-1".into()));
        leader.become_leader();
        leader.append_entry(b"cmd1".to_vec());
        leader.append_entry(b"cmd2".to_vec());

        let mut follower = RaftNode::new(NodeId("node-2".into()));

        let req = AppendEntriesRequest {
            term: Term(0),
            leader_id: NodeId("node-1".into()),
            prev_log_index: 0,
            prev_log_term: Term(0),
            entries: leader.log.clone(),
            leader_commit: 0,
        };

        let resp = follower.handle_append_request(&req);
        assert!(resp.success);
        assert_eq!(follower.log.len(), 2);
    }

    #[test]
    fn test_stale_term_rejected() {
        let mut node = RaftNode::new(NodeId("node-1".into()));
        node.current_term = Term(5);

        let req = AppendEntriesRequest {
            term: Term(3),
            leader_id: NodeId("node-0".into()),
            prev_log_index: 0,
            prev_log_term: Term(0),
            entries: vec![],
            leader_commit: 0,
        };

        let resp = node.handle_append_request(&req);
        assert!(!resp.success);
        assert_eq!(resp.term, Term(5));
    }
}
