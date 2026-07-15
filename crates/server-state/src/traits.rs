// ---------------------------------------------------------------------------
// SyncStoreTrait
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OpType {
    Create,
    Update,
    Delete,
    Rename,
    Share,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SyncOp {
    pub id: String,
    pub site_id: String,
    pub clock: VectorClock,
    pub r#type: OpType,
    pub path: String,
    pub new_path: Option<String>,
    pub size: u64,
    pub mime_type: Option<String>,
    pub owner: String,
    pub checksum: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
pub struct VectorClock {
    pub site_id: String,
    pub counter: u64,
}

impl VectorClock {
    pub fn new(site_id: &str) -> Self {
        Self {
            site_id: site_id.to_string(),
            counter: 0,
        }
    }

    pub fn with_counter(mut self, counter: u64) -> Self {
        self.counter = counter;
        self
    }
}

pub trait SyncStoreTrait: Send + Sync {
    fn record_op(&self, op: SyncOp);
    fn next_op_id(&self) -> (String, u64);
    fn pending_ops(&self) -> Vec<SyncOp>;
    fn current_clock(&self) -> u64;
    fn get_ops_since(&self, clock: u64) -> Vec<SyncOp>;
    fn total_ops(&self) -> usize;
}

// ---------------------------------------------------------------------------
// IdempotencyStoreTrait
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct IdempotentResponse {
    pub status: u16,
    pub body: bytes::Bytes,
    pub content_type: String,
    pub created_at: std::time::Instant,
}

pub trait IdempotencyStoreTrait: Send + Sync {
    fn get(&self, key: &str) -> Option<IdempotentResponse>;
    fn store(&self, key: &str, response: IdempotentResponse);
}

// ---------------------------------------------------------------------------
// NotificationPrefsStoreTrait
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NotificationPrefs {
    pub user_id: String,
    pub share_received_email: bool,
    pub share_received_push: bool,
    pub comment_added_email: bool,
    pub comment_added_push: bool,
    pub task_assigned_email: bool,
    pub task_assigned_push: bool,
    pub mention_push: bool,
    pub system_alert_push: bool,
    pub daily_digest_email: bool,
}

#[derive(Debug, serde::Deserialize)]
pub struct UpdateNotificationPrefsRequest {
    pub share_received_email: Option<bool>,
    pub share_received_push: Option<bool>,
    pub comment_added_email: Option<bool>,
    pub comment_added_push: Option<bool>,
    pub task_assigned_email: Option<bool>,
    pub task_assigned_push: Option<bool>,
    pub mention_push: Option<bool>,
    pub system_alert_push: Option<bool>,
    pub daily_digest_email: Option<bool>,
}

pub trait NotificationPrefsStoreTrait: Send + Sync {
    fn init_table(&self) -> Result<(), String>;
    fn get_prefs(&self, user_id: &str) -> Result<NotificationPrefs, String>;
    fn update_prefs(
        &self,
        user_id: &str,
        updates: &UpdateNotificationPrefsRequest,
    ) -> Result<NotificationPrefs, String>;
}

// ---------------------------------------------------------------------------
// RansomwareDetectorTrait
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
pub enum RansomwareCheckResult {
    Safe,
    Suspicious {
        user_id: String,
        mutation_count: u32,
        affected_paths: Vec<String>,
    },
}

pub trait RansomwareDetectorTrait: Send + Sync {
    fn record_mutation(
        &self,
        user_id: &str,
        path: &str,
        size: u64,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = RansomwareCheckResult> + Send + '_>>;
}

// ---------------------------------------------------------------------------
// UploadStoreTrait
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct UploadEntry {
    pub path: String,
    pub chunk_size: usize,
    pub total_chunks: Option<usize>,
    pub created_at: std::time::Instant,
}

pub trait UploadStoreTrait: Send + Sync {
    fn get(&self, key: &str) -> Option<UploadEntry>;
    fn insert(&self, key: String, entry: UploadEntry);
    fn remove(&self, key: &str) -> Option<UploadEntry>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn contains_key(&self, key: &str) -> bool;
    fn get_chunk(&self, key: &str, chunk_index: usize) -> Option<Vec<u8>>;
    fn insert_chunk(&self, key: &str, chunk_index: usize, data: Vec<u8>);
}
