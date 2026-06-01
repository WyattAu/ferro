use crate::consensus::*;
use crate::error::DistributedError;
use async_trait::async_trait;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum RaftMessage {
    AppendEntriesRequest(AppendEntriesRequest),
    AppendEntriesResponse(AppendEntriesResponse),
    VoteRequest(VoteRequest),
    VoteResponse(VoteResponse),
    InstallSnapshot {
        term: Term,
        leader_id: NodeId,
        last_included_index: u64,
        last_included_term: Term,
        data: Vec<u8>,
    },
    InstallSnapshotResponse {
        term: Term,
    },
    Ping {
        from: NodeId,
        term: Term,
    },
    Pong {
        from: NodeId,
        term: Term,
    },
}

#[async_trait]
pub trait RaftTransport: Send + Sync {
    async fn send(
        &self,
        target: &NodeId,
        msg: RaftMessage,
    ) -> Result<RaftMessage, DistributedError>;
    async fn broadcast(
        &self,
        msg: RaftMessage,
    ) -> Vec<(NodeId, Result<RaftMessage, DistributedError>)>;
    async fn start(&self) -> Result<(), DistributedError>;
    async fn stop(&self) -> Result<(), DistributedError>;
}

const MAX_MESSAGE_SIZE: usize = 64 * 1024 * 1024;

pub fn encode_frame(msg: &RaftMessage) -> Result<Vec<u8>, DistributedError> {
    let payload = serde_json::to_vec(msg).map_err(|e| DistributedError::EncodingFailed {
        reason: e.to_string(),
    })?;
    if payload.len() > MAX_MESSAGE_SIZE {
        return Err(DistributedError::EncodingFailed {
            reason: format!(
                "message too large: {} bytes (max {})",
                payload.len(),
                MAX_MESSAGE_SIZE
            ),
        });
    }
    let len = payload.len() as u32;
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&len.to_be_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

pub fn decode_frame(data: &[u8]) -> Result<RaftMessage, DistributedError> {
    if data.len() < 4 {
        return Err(DistributedError::DecodingFailed {
            reason: "frame too short".into(),
        });
    }
    let len = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
    if data.len() < 4 + len {
        return Err(DistributedError::DecodingFailed {
            reason: format!(
                "incomplete frame: expected {} bytes, got {}",
                len,
                data.len() - 4
            ),
        });
    }
    if len > MAX_MESSAGE_SIZE {
        return Err(DistributedError::DecodingFailed {
            reason: format!(
                "message too large: {} bytes (max {})",
                len, MAX_MESSAGE_SIZE
            ),
        });
    }
    serde_json::from_slice(&data[4..4 + len]).map_err(|e| DistributedError::DecodingFailed {
        reason: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(id: &str) -> NodeId {
        NodeId(id.into())
    }

    #[test]
    fn test_roundtrip_append_entries_request() {
        let msg = RaftMessage::AppendEntriesRequest(AppendEntriesRequest {
            term: Term(1),
            leader_id: make_node("leader"),
            prev_log_index: 5,
            prev_log_term: Term(1),
            entries: vec![LogEntry {
                term: Term(1),
                index: 6,
                command: b"hello".to_vec(),
            }],
            leader_commit: 3,
        });
        let json = serde_json::to_string(&msg).unwrap();
        let decoded: RaftMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(decoded, RaftMessage::AppendEntriesRequest(_)));
    }

    #[test]
    fn test_roundtrip_append_entries_response() {
        let msg = RaftMessage::AppendEntriesResponse(AppendEntriesResponse {
            term: Term(2),
            success: true,
            match_index: Some(10),
        });
        let json = serde_json::to_string(&msg).unwrap();
        let decoded: RaftMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(decoded, RaftMessage::AppendEntriesResponse(_)));
    }

    #[test]
    fn test_roundtrip_vote_request() {
        let msg = RaftMessage::VoteRequest(VoteRequest {
            term: Term(3),
            candidate_id: make_node("cand"),
            last_log_index: 7,
            last_log_term: Term(2),
        });
        let json = serde_json::to_string(&msg).unwrap();
        let decoded: RaftMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(decoded, RaftMessage::VoteRequest(_)));
    }

    #[test]
    fn test_roundtrip_vote_response() {
        let msg = RaftMessage::VoteResponse(VoteResponse {
            term: Term(3),
            vote_granted: true,
            voter_id: make_node("voter1"),
        });
        let json = serde_json::to_string(&msg).unwrap();
        let decoded: RaftMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(decoded, RaftMessage::VoteResponse(_)));
    }

    #[test]
    fn test_roundtrip_install_snapshot() {
        let msg = RaftMessage::InstallSnapshot {
            term: Term(4),
            leader_id: make_node("leader"),
            last_included_index: 100,
            last_included_term: Term(3),
            data: vec![0, 1, 2, 3, 4],
        };
        let json = serde_json::to_string(&msg).unwrap();
        let decoded: RaftMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(decoded, RaftMessage::InstallSnapshot { .. }));
    }

    #[test]
    fn test_roundtrip_install_snapshot_response() {
        let msg = RaftMessage::InstallSnapshotResponse { term: Term(4) };
        let json = serde_json::to_string(&msg).unwrap();
        let decoded: RaftMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(
            decoded,
            RaftMessage::InstallSnapshotResponse { .. }
        ));
    }

    #[test]
    fn test_roundtrip_ping() {
        let msg = RaftMessage::Ping {
            from: make_node("sender"),
            term: Term(5),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let decoded: RaftMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(decoded, RaftMessage::Ping { .. }));
    }

    #[test]
    fn test_roundtrip_pong() {
        let msg = RaftMessage::Pong {
            from: make_node("responder"),
            term: Term(5),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let decoded: RaftMessage = serde_json::from_str(&json).unwrap();
        assert!(matches!(decoded, RaftMessage::Pong { .. }));
    }

    #[test]
    fn test_frame_encode_decode() {
        let msg = RaftMessage::Ping {
            from: make_node("a"),
            term: Term(1),
        };
        let frame = encode_frame(&msg).unwrap();
        let decoded = decode_frame(&frame).unwrap();
        assert!(matches!(decoded, RaftMessage::Ping { .. }));
    }

    #[test]
    fn test_decode_frame_too_short() {
        let result = decode_frame(&[0, 1]);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_frame_incomplete() {
        let mut data = vec![0, 0, 0, 10];
        data.extend_from_slice(b"short");
        let result = decode_frame(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_large_message_serialization() {
        let big_data = vec![0xABu8; 1024 * 1024];
        let msg = RaftMessage::InstallSnapshot {
            term: Term(1),
            leader_id: make_node("leader"),
            last_included_index: 1000,
            last_included_term: Term(1),
            data: big_data,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let decoded: RaftMessage = serde_json::from_str(&json).unwrap();
        match decoded {
            RaftMessage::InstallSnapshot { ref data, .. } => {
                assert_eq!(data.len(), 1024 * 1024);
                assert_eq!(data[0], 0xAB);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_encode_frame_too_large() {
        let msg = RaftMessage::InstallSnapshot {
            term: Term(1),
            leader_id: make_node("l"),
            last_included_index: 1,
            last_included_term: Term(1),
            data: vec![0u8; MAX_MESSAGE_SIZE + 1],
        };
        let result = encode_frame(&msg);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_frame_too_large() {
        let huge_len = (MAX_MESSAGE_SIZE + 1) as u32;
        let mut data = huge_len.to_be_bytes().to_vec();
        data.resize(8, 0);
        let result = decode_frame(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_frames_in_sequence() {
        let msg1 = RaftMessage::Ping {
            from: make_node("a"),
            term: Term(1),
        };
        let msg2 = RaftMessage::Pong {
            from: make_node("b"),
            term: Term(2),
        };
        let f1 = encode_frame(&msg1).unwrap();
        let f2 = encode_frame(&msg2).unwrap();
        let d1 = decode_frame(&f1).unwrap();
        let d2 = decode_frame(&f2).unwrap();
        assert!(matches!(d1, RaftMessage::Ping { .. }));
        assert!(matches!(d2, RaftMessage::Pong { .. }));
    }
}
