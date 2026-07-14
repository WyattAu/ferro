use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::time::{Duration, Instant};

/// Connection pool for database
#[derive(Clone)]
pub struct ConnectionPool {
    semaphore: Arc<Semaphore>,
    max_connections: usize,
    active_connections: Arc<tokio::sync::Mutex<usize>>,
    stats: Arc<tokio::sync::Mutex<PoolStats>>,
}

/// Pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_acquired: u64,
    pub total_released: u64,
    pub total_waited: u64,
    pub total_wait_time: Duration,
    pub active_connections: usize,
    pub idle_connections: usize,
}

impl ConnectionPool {
    pub fn new(max_connections: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_connections)),
            max_connections,
            active_connections: Arc::new(tokio::sync::Mutex::new(0)),
            stats: Arc::new(tokio::sync::Mutex::new(PoolStats {
                total_acquired: 0,
                total_released: 0,
                total_waited: 0,
                total_wait_time: Duration::ZERO,
                active_connections: 0,
                idle_connections: max_connections,
            })),
        }
    }

    /// Acquire a connection
    pub async fn acquire(&self) -> Result<Connection, PoolError> {
        let start = Instant::now();

        let permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| PoolError::PoolClosed)?;

        let wait_time = start.elapsed();

        let mut active = self.active_connections.lock().await;
        *active += 1;

        let mut stats = self.stats.lock().await;
        stats.total_acquired += 1;
        stats.total_waited += 1;
        stats.total_wait_time += wait_time;
        stats.active_connections = *active;
        stats.idle_connections = self.max_connections - *active;

        Ok(Connection {
            _permit: permit,
            active_connections: self.active_connections.clone(),
            stats: self.stats.clone(),
        })
    }

    /// Get pool statistics
    pub async fn stats(&self) -> PoolStats {
        self.stats.lock().await.clone()
    }
}

/// Database connection wrapper
pub struct Connection {
    _permit: OwnedSemaphorePermit,
    active_connections: Arc<tokio::sync::Mutex<usize>>,
    stats: Arc<tokio::sync::Mutex<PoolStats>>,
}

impl Drop for Connection {
    fn drop(&mut self) {
        let active = self.active_connections.clone();
        let stats = self.stats.clone();

        tokio::spawn(async move {
            let mut active = active.lock().await;
            *active -= 1;

            let mut stats = stats.lock().await;
            stats.total_released += 1;
            stats.active_connections = *active;
            stats.idle_connections = (stats.total_acquired - stats.total_released) as usize;
        });
    }
}

/// Pool error
#[derive(Debug)]
pub enum PoolError {
    PoolClosed,
    Timeout,
}

impl std::fmt::Display for PoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PoolError::PoolClosed => write!(f, "Pool is closed"),
            PoolError::Timeout => write!(f, "Acquire timeout"),
        }
    }
}

impl std::error::Error for PoolError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pool_acquire_release() {
        let pool = ConnectionPool::new(10);

        let conn = pool.acquire().await.unwrap();
        let stats = pool.stats().await;
        assert_eq!(stats.active_connections, 1);

        drop(conn);

        // Allow spawned Drop task to complete
        tokio::task::yield_now().await;

        let stats = pool.stats().await;
        assert_eq!(stats.active_connections, 0);
    }

    #[tokio::test]
    async fn test_pool_concurrent() {
        let pool = ConnectionPool::new(5);
        let mut handles = vec![];

        for _i in 0..10 {
            let pool = pool.clone();
            handles.push(tokio::spawn(async move {
                let _conn = pool.acquire().await.unwrap();
                tokio::time::sleep(Duration::from_millis(10)).await;
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }

        let stats = pool.stats().await;
        assert_eq!(stats.active_connections, 0);
    }
}
