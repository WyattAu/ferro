use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::log_buffer::{LogBuffer, LogEntry};

#[derive(Debug, Deserialize)]
pub struct LokiPushRequest {
    pub streams: Vec<LokiStream>,
}

#[derive(Debug, Deserialize)]
pub struct LokiStream {
    pub stream: HashMap<String, String>,
    pub values: Vec<(String, String)>,
}

pub async fn loki_push_handler(State(buffer): State<Arc<LogBuffer>>, Json(body): Json<LokiPushRequest>) -> StatusCode {
    for stream in &body.streams {
        for (ts, line) in &stream.values {
            let timestamp = ts
                .parse::<i64>()
                .unwrap_or_else(|_| Utc::now().timestamp_millis() * 1_000_000);
            buffer.push(LogEntry {
                timestamp,
                line: line.clone(),
                labels: stream.stream.clone(),
                level: stream
                    .stream
                    .get("level")
                    .cloned()
                    .unwrap_or_else(|| "info".to_string()),
                source: "loki".to_string(),
            });
        }
    }
    StatusCode::NO_CONTENT
}

#[derive(Deserialize)]
pub struct LokiQueryParams {
    pub limit: Option<usize>,
    pub level: Option<String>,
}

#[derive(Serialize)]
pub struct LokiQueryResponse {
    pub status: String,
    pub data: LokiQueryData,
}

#[derive(Serialize)]
pub struct LokiQueryData {
    pub result_type: String,
    pub result: Vec<LokiStreamResult>,
}

#[derive(Serialize)]
pub struct LokiStreamResult {
    pub stream: HashMap<String, String>,
    pub values: Vec<(String, String)>,
}

pub async fn loki_query_handler(
    State(buffer): State<Arc<LogBuffer>>,
    axum::extract::Query(params): axum::extract::Query<LokiQueryParams>,
) -> Json<LokiQueryResponse> {
    let entries = buffer.query(params.level.as_deref(), params.limit.unwrap_or(100));
    let mut streams: HashMap<String, LokiStreamResult> = HashMap::new();

    for entry in &entries {
        let key = entry
            .labels
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(",");
        let stream = streams.entry(key).or_insert_with(|| LokiStreamResult {
            stream: entry.labels.clone(),
            values: Vec::new(),
        });
        stream.values.push((entry.timestamp.to_string(), entry.line.clone()));
    }

    Json(LokiQueryResponse {
        status: "success".to_string(),
        data: LokiQueryData {
            result_type: "streams".to_string(),
            result: streams.into_values().collect(),
        },
    })
}

pub async fn loki_labels_handler(State(buffer): State<Arc<LogBuffer>>) -> Json<serde_json::Value> {
    let entries = buffer.query(None, 10000);
    let mut labels = HashSet::new();
    for entry in &entries {
        for key in entry.labels.keys() {
            labels.insert(key.clone());
        }
    }
    Json(serde_json::json!({
        "status": "success",
        "data": labels.into_iter().collect::<Vec<_>>()
    }))
}
