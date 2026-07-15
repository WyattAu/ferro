use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use ferro_circuit_breaker::{CircuitBreaker, CircuitBreakerError, CircuitState};
use tracing::warn;

/// Configuration for a circuit breaker instance.
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening the circuit.
    pub failure_threshold: u64,
    /// Time to wait before transitioning from Open to HalfOpen.
    pub recovery_timeout: Duration,
    /// Maximum number of probe requests allowed in HalfOpen state.
    pub half_open_max: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(30),
            half_open_max: 1,
        }
    }
}

/// A wrapper that applies circuit breaker protection to any async call.
///
/// # Examples
///
/// ```ignore
/// let cb = ResilientCall::new(CircuitBreakerConfig::default());
/// let result = cb.call(|| async { Ok("hello") }).await;
/// ```
#[derive(Clone)]
pub struct ResilientCall {
    breaker: Arc<CircuitBreaker>,
    config: CircuitBreakerConfig,
}

impl ResilientCall {
    /// Create a new `ResilientCall` with the given configuration.
    #[must_use]
    pub fn new(config: CircuitBreakerConfig) -> Self {
        let breaker = Arc::new(CircuitBreaker::new(
            config.failure_threshold,
            config.recovery_timeout,
        ));
        Self { breaker, config }
    }

    /// Get the current state of the circuit breaker.
    pub fn state(&self) -> CircuitState {
        self.breaker.state()
    }

    /// Get the underlying circuit breaker reference.
    pub fn breaker(&self) -> &Arc<CircuitBreaker> {
        &self.breaker
    }

    /// Get the configuration.
    pub fn config(&self) -> &CircuitBreakerConfig {
        &self.config
    }

    /// Execute a call through the circuit breaker.
    ///
    /// Returns `CircuitBreakerError::Open` if the circuit is open.
    /// Records success/failure for state transitions.
    pub async fn call<F, Fut, T, E>(&self, f: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
    {
        self.breaker.call(f).await
    }
}

/// A named circuit breaker for a specific subsystem (storage, auth, etc.).
#[derive(Clone)]
pub struct NamedCircuitBreaker {
    name: String,
    inner: ResilientCall,
}

impl NamedCircuitBreaker {
    /// Create a new named circuit breaker.
    #[must_use]
    pub fn new(name: impl Into<String>, config: CircuitBreakerConfig) -> Self {
        Self {
            name: name.into(),
            inner: ResilientCall::new(config),
        }
    }

    /// Get the name of this circuit breaker.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Execute a call through the named circuit breaker.
    pub async fn call<F, Fut, T, E>(&self, f: F) -> Result<T, CircuitBreakerError<E>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
    {
        let result = self.inner.call(f).await;
        if let Err(ref e) = result
            && e.state == CircuitState::Open
        {
            warn!(
                "Circuit breaker '{}' is open, call rejected",
                self.name
            );
        }
        result
    }

    /// Get the current state.
    pub fn state(&self) -> CircuitState {
        self.inner.state()
    }

    /// Get the underlying circuit breaker.
    pub fn breaker(&self) -> &Arc<CircuitBreaker> {
        self.inner.breaker()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resilient_call_opens_after_failures() {
        let rc = ResilientCall::new(CircuitBreakerConfig {
            failure_threshold: 3,
            recovery_timeout: Duration::from_secs(60),
            half_open_max: 1,
        });

        for _ in 0..3 {
            let _ = rc.call(|| async { Err::<(), &str>("fail") }).await;
        }

        assert_eq!(rc.state(), CircuitState::Open);

        let result = rc.call(|| async { Ok::<(), &str>(()) }).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_resilient_call_resets_on_success() {
        let rc = ResilientCall::new(CircuitBreakerConfig {
            failure_threshold: 3,
            recovery_timeout: Duration::from_secs(60),
            half_open_max: 1,
        });

        let _ = rc.call(|| async { Err::<(), &str>("fail") }).await;
        let _ = rc.call(|| async { Ok::<(), &str>(()) }).await;

        assert_eq!(rc.state(), CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_named_circuit_breaker() {
        let ncb = NamedCircuitBreaker::new("storage", CircuitBreakerConfig::default());
        assert_eq!(ncb.name(), "storage");

        let result = ncb.call(|| async { Ok::<(), &str>(()) }).await;
        assert!(result.is_ok());
    }
}
