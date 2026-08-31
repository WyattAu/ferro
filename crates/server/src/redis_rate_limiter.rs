use std::sync::LazyLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerError};
use redis::aio::ConnectionManager;

const REDIS_OP_TIMEOUT: Duration = Duration::from_secs(5);

static REDIS_CB: LazyLock<CircuitBreaker> = LazyLock::new(|| {
    CircuitBreaker::builder(CircuitBreakerConfig {
        failure_rate_threshold: 5,
        wait_duration: Duration::from_secs(30),
        ..CircuitBreakerConfig::standard()
    })
    .build()
});

pub struct RedisRateLimiter {
    client: ConnectionManager,
}

impl RedisRateLimiter {
    pub async fn new(redis_url: &str) -> anyhow::Result<Self> {
        let client =
            redis::Client::open(redis_url).map_err(|e| anyhow::anyhow!("Failed to create Redis client: {}", e))?;
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

        let count: u64 = match REDIS_CB
            .call(|| {
                let window_key = window_key.clone();
                let mut conn = self.client.clone();
                async move {
                    tokio::time::timeout(REDIS_OP_TIMEOUT, async {
                        redis::cmd("INCR").arg(&window_key).query_async::<u64>(&mut conn).await
                    })
                    .await
                    .map_err(|_| common::error::FerroError::Timeout)?
                    .map_err(|e| common::error::FerroError::Internal(format!("Redis INCR failed: {}", e)))
                }
            })
            .await
        {
            Ok(c) => c,
            Err(CircuitBreakerError::CircuitOpen) => return true,
            _ => return true,
        };

        if count == 1 {
            let _ = REDIS_CB
                .call(|| {
                    let window_key = window_key.clone();
                    let mut conn = self.client.clone();
                    async move {
                        tokio::time::timeout(REDIS_OP_TIMEOUT, async {
                            redis::cmd("EXPIRE")
                                .arg(&window_key)
                                .arg(window_secs as i64)
                                .query_async::<()>(&mut conn)
                                .await
                        })
                        .await
                        .map_err(|_| common::error::FerroError::Timeout)?
                        .map_err(|e| common::error::FerroError::Internal(format!("Redis EXPIRE failed: {}", e)))
                    }
                })
                .await;
        }

        count <= limit as u64
    }

    pub async fn cleanup(&self, _window: Duration) {}
}
