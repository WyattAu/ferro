use thiserror::Error;

#[derive(Debug, Error)]
pub enum DistributedError {
    #[error("node {node_id} unavailable")]
    NodeUnavailable { node_id: String },
    #[error("quorum lost: required {required}, available {available}")]
    QuorumLost { required: usize, available: usize },
    #[error("encoding failed: {reason}")]
    EncodingFailed { reason: String },
    #[error("decoding failed: {reason}")]
    DecodingFailed { reason: String },
    #[error("replication lagged on {node_id}: {behind_bytes} bytes behind")]
    ReplicationLagged { node_id: String, behind_bytes: u64 },
    #[error("not leader (leader_id: {leader_id:?})")]
    NotLeader { leader_id: Option<String> },
    #[error("term expired: term {term}, current {current}")]
    TermExpired { term: u64, current: u64 },
    #[error("timeout during operation: {operation}")]
    Timeout { operation: String },
}
