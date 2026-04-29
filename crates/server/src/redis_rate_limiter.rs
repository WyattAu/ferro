use std::time::{Duration, SystemTime, UNIX_EPOCH};

use redis::aio::ConnectionManager;

const REDIS_OP_TIMEOUT: Duration = Duration::from_secs(5);

pub struct RedisRateLimiter {
    client: ConnectionManager,
}

impl RedisRateLimiter {
    pub async fn new(redis_url: &str) -> anyhow::Result<Self> {
        let client = redis::Client::open(redis_url)
            .map_err(|e| anyhow::anyhow!("Failed to create Redis client: {}", e))?;
        let mgr = redis::aio::ConnectionManager::new(client)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create Redis connection manager: {}", e))?;

        Ok(Self { client: mgr })
    }

    pub async fn check(&self, key: &str, limit: u32, window_secs: u64) -> bool {
        let window_start = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            / window_secs;
        let window_key = format!("ferro:rate:{}:{}", key, window_start);

        let count: u64 = match tokio::time::timeout(REDIS_OP_TIMEOUT, async {
            redis::cmd("INCR")
                .arg(&window_key)
                .query_async(&mut self.client.clone())
                .await
        })
        .await
        {
            Ok(Ok(c)) => c,
            _ => return true,
        };

        if count == 1 {
            let _ = tokio::time::timeout(REDIS_OP_TIMEOUT, async {
                redis::cmd("EXPIRE")
                    .arg(&window_key)
                    .arg(window_secs as i64)
                    .query_async::<()>(&mut self.client.clone())
                    .await
            })
            .await;
        }

        count <= limit as u64
    }

    pub async fn cleanup(&self, _window: Duration) {}
}
