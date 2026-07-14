use crossbeam::queue::ArrayQueue;

/// A lock-free event queue for high-throughput publishing.
pub struct EventQueue {
    queue: ArrayQueue<QueuedEvent>,
}

#[derive(Debug, Clone)]
pub struct QueuedEvent {
    pub event_type: String,
    pub event_json: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl EventQueue {
    pub fn new(capacity: usize) -> Self {
        Self {
            queue: ArrayQueue::new(capacity),
        }
    }

    pub fn push(&self, event: QueuedEvent) -> bool {
        self.queue.push(event).is_ok()
    }

    pub fn pop(&self) -> Option<QueuedEvent> {
        self.queue.pop()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.queue.capacity()
    }
}
