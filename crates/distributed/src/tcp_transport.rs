use crate::consensus::*;
use crate::error::DistributedError;
use crate::transport::{RaftMessage, RaftTransport, decode_frame, encode_frame};
use async_trait::async_trait;
use dashmap::DashMap;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};

const CONNECTION_TIMEOUT: Duration = Duration::from_secs(5);
const SEND_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_RECONNECT_ATTEMPTS: u32 = 5;

#[derive(Clone)]
pub struct IncomingMessage {
    pub from_addr: SocketAddr,
    pub message: RaftMessage,
}

#[derive(Clone)]
pub struct MessageHandler(Arc<dyn Fn(IncomingMessage) -> Option<RaftMessage> + Send + Sync>);

impl MessageHandler {
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(IncomingMessage) -> Option<RaftMessage> + Send + Sync + 'static,
    {
        Self(Arc::new(f))
    }
}

type SharedStream = Arc<tokio::sync::Mutex<Option<TcpStream>>>;

pub struct TcpRaftTransport {
    local_id: NodeId,
    bind_addr: SocketAddr,
    peers: HashMap<NodeId, SocketAddr>,
    connections: Arc<DashMap<NodeId, SharedStream>>,
    running: Arc<AtomicBool>,
    incoming_tx: Arc<tokio::sync::Mutex<Option<mpsc::Sender<IncomingMessage>>>>,
    handler: Arc<tokio::sync::Mutex<Option<MessageHandler>>>,
    shutdown_tx: Arc<tokio::sync::Mutex<Option<mpsc::Sender<()>>>>,
}

impl TcpRaftTransport {
    pub fn new(
        local_id: NodeId,
        bind_addr: SocketAddr,
        peers: HashMap<NodeId, SocketAddr>,
    ) -> Self {
        Self {
            local_id,
            bind_addr,
            peers,
            connections: Arc::new(DashMap::new()),
            running: Arc::new(AtomicBool::new(false)),
            incoming_tx: Arc::new(tokio::sync::Mutex::new(None)),
            handler: Arc::new(tokio::sync::Mutex::new(None)),
            shutdown_tx: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    pub fn set_handler(&self, handler: MessageHandler) {
        let h = self.handler.clone();
        tokio::spawn(async move {
            let mut guard = h.lock().await;
            *guard = Some(handler);
        });
    }

    async fn get_or_create_connection(
        &self,
        target: &NodeId,
    ) -> Result<TcpStream, DistributedError> {
        if let Some(entry) = self.connections.get(target) {
            let guard = entry.value().lock().await;
            if let Some(ref stream) = *guard
                && stream.peer_addr().is_ok()
            {
                let addr =
                    self.peers
                        .get(target)
                        .ok_or_else(|| DistributedError::NodeUnavailable {
                            node_id: target.0.clone(),
                        })?;
                let new_stream = TcpStream::connect(*addr).await.map_err(|_| {
                    DistributedError::NodeUnavailable {
                        node_id: target.0.clone(),
                    }
                })?;
                new_stream.set_nodelay(true).ok();
                return Ok(new_stream);
            }
            drop(guard);
        }

        let addr = self
            .peers
            .get(target)
            .ok_or_else(|| DistributedError::NodeUnavailable {
                node_id: target.0.clone(),
            })?;

        let mut backoff = Duration::from_millis(50);
        for _ in 0..MAX_RECONNECT_ATTEMPTS {
            match tokio::time::timeout(CONNECTION_TIMEOUT, TcpStream::connect(*addr)).await {
                Ok(Ok(stream)) => {
                    stream.set_nodelay(true).ok();
                    self.connections
                        .insert(target.clone(), Arc::new(tokio::sync::Mutex::new(None)));
                    return Ok(stream);
                }
                _ => {
                    sleep(backoff).await;
                    backoff = std::cmp::min(backoff * 2, Duration::from_secs(2));
                }
            }
        }

        Err(DistributedError::NodeUnavailable {
            node_id: target.0.clone(),
        })
    }

    async fn write_message(
        stream: &mut TcpStream,
        msg: &RaftMessage,
    ) -> Result<(), DistributedError> {
        let frame = encode_frame(msg)?;
        let timeout_fut = tokio::time::timeout(SEND_TIMEOUT, stream.write_all(&frame));
        timeout_fut
            .await
            .map_err(|_| DistributedError::Timeout {
                operation: "write message".into(),
            })?
            .map_err(|e| DistributedError::EncodingFailed {
                reason: e.to_string(),
            })?;
        stream
            .flush()
            .await
            .map_err(|e| DistributedError::EncodingFailed {
                reason: format!("flush: {}", e),
            })?;
        Ok(())
    }

    async fn read_response(stream: &mut TcpStream) -> Result<RaftMessage, DistributedError> {
        let mut len_buf = [0u8; 4];
        let timeout_fut = tokio::time::timeout(SEND_TIMEOUT, stream.read_exact(&mut len_buf));
        timeout_fut
            .await
            .map_err(|_| DistributedError::Timeout {
                operation: "read response length".into(),
            })?
            .map_err(|e| DistributedError::DecodingFailed {
                reason: format!("read len: {}", e),
            })?;

        let len = u32::from_be_bytes(len_buf) as usize;
        let mut payload = vec![0u8; len];
        let read_fut = stream.read_exact(&mut payload);
        let timeout_fut = tokio::time::timeout(SEND_TIMEOUT, read_fut);
        timeout_fut
            .await
            .map_err(|_| DistributedError::Timeout {
                operation: "read response payload".into(),
            })?
            .map_err(|e| DistributedError::DecodingFailed {
                reason: format!("read payload: {}", e),
            })?;

        let mut full_frame = Vec::with_capacity(4 + len);
        full_frame.extend_from_slice(&len_buf);
        full_frame.extend_from_slice(&payload);
        decode_frame(&full_frame)
    }

    async fn handle_connection(
        stream: TcpStream,
        from_addr: SocketAddr,
        handler: Arc<tokio::sync::Mutex<Option<MessageHandler>>>,
        local_id: NodeId,
    ) {
        let mut stream = stream;
        let mut len_buf = [0u8; 4];
        while stream.read_exact(&mut len_buf).await.is_ok() {
            let len = u32::from_be_bytes(len_buf) as usize;
            let mut payload = vec![0u8; len];
            if stream.read_exact(&mut payload).await.is_err() {
                break;
            }
            let mut full_frame = Vec::with_capacity(4 + len);
            full_frame.extend_from_slice(&len_buf);
            full_frame.extend_from_slice(&payload);
            match decode_frame(&full_frame) {
                Ok(msg) => {
                    if let RaftMessage::Ping { term, .. } = &msg {
                        let reply = RaftMessage::Pong {
                            from: local_id.clone(),
                            term: *term,
                        };
                        let _ = Self::write_message(&mut stream, &reply).await;
                        continue;
                    }
                    let guard = handler.lock().await;
                    if let Some(h) = guard.as_ref() {
                        let incoming = IncomingMessage {
                            from_addr,
                            message: msg.clone(),
                        };
                        if let Some(reply) = (h.0)(incoming) {
                            drop(guard);
                            let _ = Self::write_message(&mut stream, &reply).await;
                        }
                    }
                }
                Err(_) => break,
            }
        }
    }
}

#[async_trait]
impl RaftTransport for TcpRaftTransport {
    async fn send(
        &self,
        target: &NodeId,
        msg: RaftMessage,
    ) -> Result<RaftMessage, DistributedError> {
        let mut stream = self.get_or_create_connection(target).await?;
        Self::write_message(&mut stream, &msg).await?;
        Self::read_response(&mut stream).await
    }

    async fn broadcast(
        &self,
        msg: RaftMessage,
    ) -> Vec<(NodeId, Result<RaftMessage, DistributedError>)> {
        let mut results = Vec::new();
        for peer_id in self.peers.keys() {
            let result = self.send(peer_id, msg.clone()).await;
            results.push((peer_id.clone(), result));
        }
        results
    }

    async fn start(&self) -> Result<(), DistributedError> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }
        self.running.store(true, Ordering::SeqCst);

        let listener = TcpListener::bind(self.bind_addr).await.map_err(|e| {
            DistributedError::NodeUnavailable {
                node_id: format!("bind {}", e),
            }
        })?;

        let (incoming_tx, mut incoming_rx) = mpsc::channel::<IncomingMessage>(256);
        *self.incoming_tx.lock().await = Some(incoming_tx);

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        *self.shutdown_tx.lock().await = Some(shutdown_tx);

        let connections = self.connections.clone();
        let handler = self.handler.clone();
        let local_id = self.local_id.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    accept_result = listener.accept() => {
                        match accept_result {
                            Ok((stream, addr)) => {
                                let handler = handler.clone();
                                let lid = local_id.clone();
                                tokio::spawn(async move {
                                    Self::handle_connection(stream, addr, handler, lid).await;
                                });
                            }
                            Err(_) => {
                                if !running.load(Ordering::SeqCst) {
                                    break;
                                }
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                    Some(_msg) = incoming_rx.recv() => {
                    }
                }
            }
            connections.clear();
        });

        Ok(())
    }

    async fn stop(&self) -> Result<(), DistributedError> {
        if !self.running.load(Ordering::SeqCst) {
            return Ok(());
        }
        self.running.store(false, Ordering::SeqCst);

        let mut tx_guard = self.shutdown_tx.lock().await;
        if let Some(tx) = tx_guard.take() {
            let _ = tx.send(()).await;
        }

        self.connections.clear();

        let mut incoming_guard = self.incoming_tx.lock().await;
        *incoming_guard = None;

        Ok(())
    }
}

impl Drop for TcpRaftTransport {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener as TokioTcpListener;

    fn make_node(id: &str) -> NodeId {
        NodeId(id.into())
    }

    async fn find_free_port() -> u16 {
        let listener = TokioTcpListener::bind("127.0.0.1:0").await.unwrap();
        listener.local_addr().unwrap().port()
    }

    #[tokio::test]
    async fn test_tcp_send_append_entries() {
        let port1 = find_free_port().await;
        let port2 = find_free_port().await;

        let node1 = make_node("node1");
        let node2 = make_node("node2");

        let t1 = TcpRaftTransport::new(
            node1.clone(),
            format!("127.0.0.1:{}", port1).parse().unwrap(),
            HashMap::from([(
                node2.clone(),
                format!("127.0.0.1:{}", port2).parse().unwrap(),
            )]),
        );

        let t2 = TcpRaftTransport::new(
            node2.clone(),
            format!("127.0.0.1:{}", port2).parse().unwrap(),
            HashMap::from([(
                node1.clone(),
                format!("127.0.0.1:{}", port1).parse().unwrap(),
            )]),
        );

        t2.set_handler(MessageHandler::new(move |incoming| {
            if let RaftMessage::AppendEntriesRequest(_) = &incoming.message {
                Some(RaftMessage::AppendEntriesResponse(AppendEntriesResponse {
                    term: Term(1),
                    success: true,
                    match_index: Some(5),
                }))
            } else {
                None
            }
        }));

        t2.start().await.unwrap();
        t1.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        let msg = RaftMessage::AppendEntriesRequest(AppendEntriesRequest {
            term: Term(1),
            leader_id: node1.clone(),
            prev_log_index: 0,
            prev_log_term: Term(0),
            entries: vec![],
            leader_commit: 0,
        });

        let resp = t1.send(&node2, msg).await.unwrap();
        match resp {
            RaftMessage::AppendEntriesResponse(r) => {
                assert!(r.success);
                assert_eq!(r.match_index, Some(5));
            }
            _ => panic!("unexpected response"),
        }

        t1.stop().await.unwrap();
        t2.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_tcp_vote_flow() {
        let port1 = find_free_port().await;
        let port2 = find_free_port().await;

        let node1 = make_node("voter1");
        let node2 = make_node("voter2");

        let t1 = TcpRaftTransport::new(
            node1.clone(),
            format!("127.0.0.1:{}", port1).parse().unwrap(),
            HashMap::from([(
                node2.clone(),
                format!("127.0.0.1:{}", port2).parse().unwrap(),
            )]),
        );

        let t2 = TcpRaftTransport::new(
            node2.clone(),
            format!("127.0.0.1:{}", port2).parse().unwrap(),
            HashMap::from([(
                node1.clone(),
                format!("127.0.0.1:{}", port1).parse().unwrap(),
            )]),
        );

        let responder = node2.clone();
        t2.set_handler(MessageHandler::new(move |incoming| {
            if let RaftMessage::VoteRequest(_) = &incoming.message {
                Some(RaftMessage::VoteResponse(VoteResponse {
                    term: Term(1),
                    vote_granted: true,
                    voter_id: responder.clone(),
                }))
            } else {
                None
            }
        }));

        t2.start().await.unwrap();
        t1.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        let msg = RaftMessage::VoteRequest(VoteRequest {
            term: Term(1),
            candidate_id: node1.clone(),
            last_log_index: 0,
            last_log_term: Term(0),
        });

        let resp = t1.send(&node2, msg).await.unwrap();
        match resp {
            RaftMessage::VoteResponse(r) => {
                assert!(r.vote_granted);
                assert_eq!(r.voter_id.0, "voter2");
            }
            _ => panic!("unexpected response"),
        }

        t1.stop().await.unwrap();
        t2.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_tcp_broadcast() {
        let port1 = find_free_port().await;
        let port2 = find_free_port().await;
        let port3 = find_free_port().await;

        let node1 = make_node("broadcaster");
        let node2 = make_node("peer2");
        let node3 = make_node("peer3");

        let t1 = TcpRaftTransport::new(
            node1.clone(),
            format!("127.0.0.1:{}", port1).parse().unwrap(),
            HashMap::from([
                (
                    node2.clone(),
                    format!("127.0.0.1:{}", port2).parse().unwrap(),
                ),
                (
                    node3.clone(),
                    format!("127.0.0.1:{}", port3).parse().unwrap(),
                ),
            ]),
        );

        let t2 = TcpRaftTransport::new(
            node2.clone(),
            format!("127.0.0.1:{}", port2).parse().unwrap(),
            HashMap::from([(
                node1.clone(),
                format!("127.0.0.1:{}", port1).parse().unwrap(),
            )]),
        );

        let t3 = TcpRaftTransport::new(
            node3.clone(),
            format!("127.0.0.1:{}", port3).parse().unwrap(),
            HashMap::from([(
                node1.clone(),
                format!("127.0.0.1:{}", port1).parse().unwrap(),
            )]),
        );

        t2.set_handler(MessageHandler::new(move |incoming| {
            if let RaftMessage::Ping { .. } = &incoming.message {
                Some(RaftMessage::Pong {
                    from: make_node("peer2"),
                    term: Term(1),
                })
            } else {
                None
            }
        }));

        t3.set_handler(MessageHandler::new(move |incoming| {
            if let RaftMessage::Ping { .. } = &incoming.message {
                Some(RaftMessage::Pong {
                    from: make_node("peer3"),
                    term: Term(1),
                })
            } else {
                None
            }
        }));

        t2.start().await.unwrap();
        t3.start().await.unwrap();
        t1.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        let results = t1
            .broadcast(RaftMessage::Ping {
                from: node1.clone(),
                term: Term(1),
            })
            .await;

        assert_eq!(results.len(), 2);
        for (_id, result) in results {
            assert!(result.is_ok());
        }

        t1.stop().await.unwrap();
        t2.stop().await.unwrap();
        t3.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_connection_refused() {
        let port1 = find_free_port().await;
        let bad_port = find_free_port().await;

        let node1 = make_node("sender");
        let ghost = make_node("ghost");

        let t1 = TcpRaftTransport::new(
            node1.clone(),
            format!("127.0.0.1:{}", port1).parse().unwrap(),
            HashMap::from([(
                ghost.clone(),
                format!("127.0.0.1:{}", bad_port).parse().unwrap(),
            )]),
        );

        t1.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        let result = t1
            .send(
                &ghost,
                RaftMessage::Ping {
                    from: node1.clone(),
                    term: Term(1),
                },
            )
            .await;

        assert!(result.is_err());
        t1.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_large_message_tcp() {
        let port1 = find_free_port().await;
        let port2 = find_free_port().await;

        let node1 = make_node("big_sender");
        let node2 = make_node("big_receiver");

        let t1 = TcpRaftTransport::new(
            node1.clone(),
            format!("127.0.0.1:{}", port1).parse().unwrap(),
            HashMap::from([(
                node2.clone(),
                format!("127.0.0.1:{}", port2).parse().unwrap(),
            )]),
        );

        let t2 = TcpRaftTransport::new(
            node2.clone(),
            format!("127.0.0.1:{}", port2).parse().unwrap(),
            HashMap::from([(
                node1.clone(),
                format!("127.0.0.1:{}", port1).parse().unwrap(),
            )]),
        );

        t2.set_handler(MessageHandler::new(move |incoming| {
            if let RaftMessage::InstallSnapshot { .. } = incoming.message {
                Some(RaftMessage::InstallSnapshotResponse { term: Term(1) })
            } else {
                None
            }
        }));

        t2.start().await.unwrap();
        t1.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        let big_data = vec![0x42u8; 1024 * 1024];
        let msg = RaftMessage::InstallSnapshot {
            term: Term(1),
            leader_id: node1.clone(),
            last_included_index: 100,
            last_included_term: Term(1),
            data: big_data,
        };

        let resp = t1.send(&node2, msg).await.unwrap();
        assert!(matches!(resp, RaftMessage::InstallSnapshotResponse { .. }));

        t1.stop().await.unwrap();
        t2.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_ping_pong_heartbeat() {
        let port1 = find_free_port().await;
        let port2 = find_free_port().await;

        let node1 = make_node("pinger");
        let node2 = make_node("ponger");

        let t1 = TcpRaftTransport::new(
            node1.clone(),
            format!("127.0.0.1:{}", port1).parse().unwrap(),
            HashMap::from([(
                node2.clone(),
                format!("127.0.0.1:{}", port2).parse().unwrap(),
            )]),
        );

        let t2 = TcpRaftTransport::new(
            node2.clone(),
            format!("127.0.0.1:{}", port2).parse().unwrap(),
            HashMap::from([(
                node1.clone(),
                format!("127.0.0.1:{}", port1).parse().unwrap(),
            )]),
        );

        t2.start().await.unwrap();
        t1.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        let resp = t1
            .send(
                &node2,
                RaftMessage::Ping {
                    from: node1.clone(),
                    term: Term(1),
                },
            )
            .await
            .unwrap();

        assert!(matches!(resp, RaftMessage::Pong { .. }));

        t1.stop().await.unwrap();
        t2.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_reconnection_after_disconnect() {
        let port1 = find_free_port().await;
        let port2 = find_free_port().await;

        let node1 = make_node("recon1");
        let node2 = make_node("recon2");

        let t1 = TcpRaftTransport::new(
            node1.clone(),
            format!("127.0.0.1:{}", port1).parse().unwrap(),
            HashMap::from([(
                node2.clone(),
                format!("127.0.0.1:{}", port2).parse().unwrap(),
            )]),
        );

        let t2 = TcpRaftTransport::new(
            node2.clone(),
            format!("127.0.0.1:{}", port2).parse().unwrap(),
            HashMap::from([(
                node1.clone(),
                format!("127.0.0.1:{}", port1).parse().unwrap(),
            )]),
        );

        let responder = node2.clone();
        t2.set_handler(MessageHandler::new(move |incoming| {
            if let RaftMessage::Ping { .. } = &incoming.message {
                Some(RaftMessage::Pong {
                    from: responder.clone(),
                    term: Term(1),
                })
            } else {
                None
            }
        }));

        t2.start().await.unwrap();
        t1.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        let resp = t1
            .send(
                &node2,
                RaftMessage::Ping {
                    from: node1.clone(),
                    term: Term(1),
                },
            )
            .await
            .unwrap();
        assert!(matches!(resp, RaftMessage::Pong { .. }));

        t1.connections.clear();
        tokio::time::sleep(Duration::from_millis(50)).await;

        let resp2 = t1
            .send(
                &node2,
                RaftMessage::Ping {
                    from: node1.clone(),
                    term: Term(1),
                },
            )
            .await
            .unwrap();
        assert!(matches!(resp2, RaftMessage::Pong { .. }));

        t1.stop().await.unwrap();
        t2.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_stop_and_cleanup() {
        let port1 = find_free_port().await;
        let port2 = find_free_port().await;

        let node1 = make_node("stop1");
        let node2 = make_node("stop2");

        let t1 = TcpRaftTransport::new(
            node1.clone(),
            format!("127.0.0.1:{}", port1).parse().unwrap(),
            HashMap::from([(
                node2.clone(),
                format!("127.0.0.1:{}", port2).parse().unwrap(),
            )]),
        );

        let t2 = TcpRaftTransport::new(
            node2.clone(),
            format!("127.0.0.1:{}", port2).parse().unwrap(),
            HashMap::from([(
                node1.clone(),
                format!("127.0.0.1:{}", port1).parse().unwrap(),
            )]),
        );

        t2.set_handler(MessageHandler::new(move |incoming| {
            if let RaftMessage::Ping { .. } = &incoming.message {
                Some(RaftMessage::Pong {
                    from: make_node("stop2"),
                    term: Term(1),
                })
            } else {
                None
            }
        }));

        t2.start().await.unwrap();
        t1.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        let resp = t1
            .send(
                &node2,
                RaftMessage::Ping {
                    from: node1.clone(),
                    term: Term(1),
                },
            )
            .await
            .unwrap();
        assert!(matches!(resp, RaftMessage::Pong { .. }));

        t1.stop().await.unwrap();
        assert!(!t1.running.load(Ordering::SeqCst));
        assert!(t1.connections.is_empty());

        t2.stop().await.unwrap();
        assert!(!t2.running.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_double_start_is_ok() {
        let port = find_free_port().await;
        let node = make_node("double");

        let t = TcpRaftTransport::new(
            node,
            format!("127.0.0.1:{}", port).parse().unwrap(),
            HashMap::new(),
        );

        t.start().await.unwrap();
        t.start().await.unwrap();

        t.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_double_stop_is_ok() {
        let port = find_free_port().await;
        let node = make_node("doublestop");

        let t = TcpRaftTransport::new(
            node,
            format!("127.0.0.1:{}", port).parse().unwrap(),
            HashMap::new(),
        );

        t.start().await.unwrap();
        t.stop().await.unwrap();
        t.stop().await.unwrap();
    }
}
