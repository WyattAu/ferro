use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Deserialize;
use std::sync::Arc;

use crate::log_buffer::{LogBuffer, LogEntry};

#[derive(Debug, Deserialize)]
pub struct VictoriaLogsEntry {
    #[serde(rename = "_time")]
    pub time: Option<String>,
    #[serde(rename = "_stream")]
    pub stream: Option<String>,
    #[serde(rename = "_msg")]
    pub msg: Option<String>,
    pub level: Option<String>,
    pub source: Option<String>,
}

pub async fn vl_insert_handler(
    State(buffer): State<Arc<LogBuffer>>,
    Json(entries): Json<Vec<VictoriaLogsEntry>>,
) -> StatusCode {
    for entry in &entries {
        let timestamp = entry
            .time
            .as_ref()
            .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
            .map(|dt| dt.timestamp_millis() * 1_000_000)
            .unwrap_or_else(|| chrono::Utc::now().timestamp_millis() * 1_000_000);

        let mut labels = std::collections::HashMap::new();
        if let Some(stream) = &entry.stream {
            labels.insert("stream".to_string(), stream.clone());
        }
        if let Some(level) = &entry.level {
            labels.insert("level".to_string(), level.clone());
        }
        if let Some(source) = &entry.source {
            labels.insert("source".to_string(), source.clone());
        }

        buffer.push(LogEntry {
            timestamp,
            line: entry.msg.clone().unwrap_or_default(),
            labels,
            level: entry.level.clone().unwrap_or_else(|| "info".to_string()),
            source: entry.source.clone().unwrap_or_else(|| "victorialogs".to_string()),
        });
    }
    StatusCode::NO_CONTENT
}

pub async fn vl_promtail_handler(
    State(buffer): State<Arc<LogBuffer>>,
    Json(body): Json<serde_json::Value>,
) -> StatusCode {
    if let Some(streams) = body.get("streams").and_then(|s| s.as_array()) {
        for stream in streams {
            let stream_labels = stream
                .get("stream")
                .and_then(|s| s.as_object())
                .map(|o| {
                    o.iter()
                        .filter_map(|(k, v)| v.as_str().map(|v| (k.clone(), v.to_string())))
                        .collect::<std::collections::HashMap<_, _>>()
                })
                .unwrap_or_default();

            if let Some(values) = stream.get("values").and_then(|v| v.as_array()) {
                for value_pair in values {
                    if let Some(arr) = value_pair.as_array()
                        && arr.len() >= 2
                        && let Some(ts) = arr[0].as_str()
                    {
                        let line = arr[1].as_str().unwrap_or("");
                        let timestamp = ts
                            .parse::<i64>()
                            .unwrap_or_else(|_| chrono::Utc::now().timestamp_millis() * 1_000_000);
                        buffer.push(LogEntry {
                            timestamp,
                            line: line.to_string(),
                            labels: stream_labels.clone(),
                            level: stream_labels
                                .get("level")
                                .cloned()
                                .unwrap_or_else(|| "info".to_string()),
                            source: "promtail".to_string(),
                        });
                    }
                }
            }
        }
    }
    StatusCode::NO_CONTENT
}
