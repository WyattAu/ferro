use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncProfile {
    pub id: String,
    pub name: String,
    pub owner: String,
    pub rules: Vec<SyncRule>,
    pub path_prefix: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl SyncProfile {
    pub fn new(name: String, owner: String, rules: Vec<SyncRule>) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            owner,
            rules,
            path_prefix: None,
            enabled: true,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRule {
    pub pattern: String,
    pub direction: RuleDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleDirection {
    Include,
    Exclude,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictInfo {
    pub path: String,
    pub local_modified: String,
    pub remote_modified: String,
    pub resolution: ConflictResolution,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolution {
    KeepLocal,
    KeepRemote,
    KeepNewer,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterPreviewRequest {
    pub rules: Vec<SyncRule>,
    pub path_prefix: Option<String>,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterPreviewResponse {
    pub matched: Vec<String>,
    pub missed: Vec<String>,
}
