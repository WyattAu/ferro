use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use tracing::warn;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug)]
pub struct CircuitBreakerError<E> {
    pub state: CircuitState,
    pub inner: Option<E>,
}

impl<E: std::fmt::Display> std::fmt::Display for CircuitBreakerError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.inner {
            Some(e) => write!(f, "Circuit breaker is {:?}: {}", self.state, e),
            None => write!(f, "Circuit breaker is {:?}", self.state),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for CircuitBreakerError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.inner.as_ref().map(|e| e as &dyn std::error::Error)
    }
}

impl<E> CircuitBreakerError<E> {
    #[must_use]
    pub fn open() -> Self {
        Self {
            state: CircuitState::Open,
            inner: None,
        }
    }

    pub fn with_inner(inner: E) -> Self {
        Self {
            state: CircuitState::Open,
            inner: Some(inner),
        }
    }
}

pub struct CircuitBreaker {
    failure_threshold: u64,
    recovery_timeout: Duration,
    failure_count: AtomicU64,
    state: Mutex<CircuitState>,
    last_failure: Mutex<Option<Instant>>,
}

impl CircuitBreaker {
    #[must_use]
    pub fn new(failure_threshold: u64, recovery_timeout: Duration) -> Self {
        Self {
            failure_threshold,
            recovery_timeout,
            failure_count: AtomicU64::new(0),
            state: Mutex::new(CircuitState::Closed),
            last_failure: Mutex::new(None),
        }
    }

    pub fn state(&self) -> CircuitState {
        let current = {
            let guard = self.state.lock();
            *guard
        };
        if current == CircuitState::Open
            && let Some(last) = *self.last_failure.lock()
            && last.elapsed() >= self.recovery_timeout
        {
            let mut state = self.state.lock();
            if *state == CircuitState::Open {
                warn!("Circuit breaker transitioning from Open to HalfOpen");
                *state = CircuitState::HalfOpen;
                return CircuitState::HalfOpen;
            }
        }
        current
    }

    pub fn record_success(&self) {
        {
            let prev = self.state.lock();
            if *prev != CircuitState::Closed {
                warn!("Circuit breaker transitioning from {:?} to Closed", *prev);
            }
        }
        self.failure_count.store(0, Ordering::Relaxed);
        *self.state.lock() = CircuitState::Closed;
        *self.last_failure.lock() = None;
    }

    pub fn record_failure(&self) {
        let count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        *self.last_failure.lock() = Some(Instant::now());

        if count >= self.failure_threshold {
            let mut state = self.state.lock();
            if *state != CircuitState::Open {
                warn!(
                    "Circuit breaker transitioning from {:?} to Open after {} failures",
                    *state, count
                );
                *state = CircuitState::Open;
            }
        }
    }

    pub async fn call<F, Fut, T, E>(&self, f: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
    {
        let current_state = self.state();
        match current_state {
            CircuitState::Open => {
                return Err(CircuitBreakerError::open());
            }
            CircuitState::HalfOpen => {
                // Allow one probe request through
            }
            CircuitState::Closed => {}
        }

        match f().await {
            Ok(val) => {
                self.record_success();
                Ok(val)
            }
            Err(e) => {
                self.record_failure();
                Err(CircuitBreakerError::with_inner(e))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_opens_after_threshold() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(60));

        for _ in 0..3 {
            let _ = cb.call(|| async { Err::<(), &str>("error") }).await;
        }

        assert_eq!(cb.state(), CircuitState::Open);

        let result: Result<(), CircuitBreakerError<&str>> = cb.call(|| async { Ok(()) }).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_circuit_breaker_resets_on_success() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(60));

        let _ = cb.call(|| async { Err::<(), &str>("error") }).await;
        let _ = cb.call(|| async { Err::<(), &str>("error") }).await;
        let _ = cb.call(|| async { Ok::<(), &str>(()) }).await;

        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_after_timeout() {
        let cb = CircuitBreaker::new(2, Duration::from_millis(50));

        let _ = cb.call(|| async { Err::<(), &str>("error") }).await;
        let _ = cb.call(|| async { Err::<(), &str>("error") }).await;

        assert_eq!(cb.state(), CircuitState::Open);

        tokio::time::sleep(Duration::from_millis(60)).await;

        assert_eq!(cb.state(), CircuitState::HalfOpen);
    }

    #[tokio::test]
    async fn test_display_with_inner_error() {
        let err: CircuitBreakerError<&str> = CircuitBreakerError::with_inner("disk full");
        let msg = format!("{err}");
        assert!(msg.contains("Open"));
        assert!(msg.contains("disk full"));
    }

    #[tokio::test]
    async fn test_display_without_inner_error() {
        let err: CircuitBreakerError<&str> = CircuitBreakerError::open();
        let msg = format!("{err}");
        assert!(msg.contains("Open"));
    }

    #[tokio::test]
    async fn test_error_source() {
        use std::error::Error;
        let inner = std::io::Error::other("test");
        let err: CircuitBreakerError<std::io::Error> = CircuitBreakerError::with_inner(inner);
        let source = err.source().expect("should have source");
        assert!(source.to_string().contains("test"));
    }

    #[tokio::test]
    async fn test_error_source_none() {
        use std::error::Error;
        let err: CircuitBreakerError<std::io::Error> = CircuitBreakerError::open();
        assert!(err.source().is_none());
    }

    #[test]
    fn test_record_success_directly() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(60));
        // Push to Open state via direct manipulation
        cb.failure_count.store(5, Ordering::Relaxed);
        *cb.state.lock() = CircuitState::Open;
        assert_eq!(cb.state(), CircuitState::Open);

        // record_success should reset to Closed
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(cb.failure_count.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_record_success_from_half_open() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(60));
        *cb.state.lock() = CircuitState::HalfOpen;
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_half_open_success_closes() {
        let cb = CircuitBreaker::new(2, Duration::from_millis(50));
        let _ = cb.call(|| async { Err::<(), &str>("error") }).await;
        let _ = cb.call(|| async { Err::<(), &str>("error") }).await;
        assert_eq!(cb.state(), CircuitState::Open);

        tokio::time::sleep(Duration::from_millis(60)).await;
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // Success in HalfOpen should close
        let result = cb.call(|| async { Ok::<(), &str>(()) }).await;
        assert!(result.is_ok());
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_half_open_failure_reopens() {
        let cb = CircuitBreaker::new(2, Duration::from_millis(50));
        let _ = cb.call(|| async { Err::<(), &str>("error") }).await;
        let _ = cb.call(|| async { Err::<(), &str>("error") }).await;

        tokio::time::sleep(Duration::from_millis(60)).await;
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        let _ = cb.call(|| async { Err::<(), &str>("still broken") }).await;
        assert_eq!(cb.state(), CircuitState::Open);
    }
}
