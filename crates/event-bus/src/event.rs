use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::EventBusError;

pub trait Event:
    Send + Sync + Clone + Serialize + serde::de::DeserializeOwned + 'static
{
    fn event_type(&self) -> &str;
    fn timestamp(&self) -> DateTime<Utc>;
    fn to_json(&self) -> Result<String, EventBusError> {
        serde_json::to_string(self).map_err(|e| EventBusError::Serialization(e.to_string()))
    }
    fn from_json(json: &str) -> Result<Self, EventBusError>
    where
        Self: Sized,
    {
        serde_json::from_str(json).map_err(|e| EventBusError::Deserialization(e.to_string()))
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FileEvent {
    pub id: String,
    pub event_type: String,
    pub timestamp: DateTime<Utc>,
    pub path: String,
    pub user_id: String,
    pub size: Option<u64>,
    pub content_type: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl FileEvent {
    pub fn new(event_type: impl Into<String>, path: impl Into<String>, user_id: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            event_type: event_type.into(),
            timestamp: Utc::now(),
            path: path.into(),
            user_id: user_id.into(),
            size: None,
            content_type: None,
            metadata: HashMap::new(),
        }
    }
}

impl Event for FileEvent {
    fn event_type(&self) -> &str {
        &self.event_type
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SystemEvent {
    pub id: String,
    pub event_type: String,
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub metadata: HashMap<String, String>,
}

impl SystemEvent {
    pub fn new(event_type: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            event_type: event_type.into(),
            timestamp: Utc::now(),
            source: source.into(),
            metadata: HashMap::new(),
        }
    }
}

impl Event for SystemEvent {
    fn event_type(&self) -> &str {
        &self.event_type
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }
}
