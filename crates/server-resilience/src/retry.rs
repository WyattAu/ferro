use std::time::Duration;

use tracing::warn;

/// Policy for retrying failed operations with exponential backoff.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts (not counting the initial call).
    pub max_retries: u32,
    /// Base delay between retries.
    pub base_delay: Duration,
    /// Maximum delay cap (prevents unbounded growth).
    pub max_delay: Duration,
    /// Jitter factor (0.0 = no jitter, 1.0 = full jitter).
    pub jitter: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            jitter: 0.2,
        }
    }
}

impl RetryPolicy {
    /// No retries — fail immediately.
    #[must_use]
    pub fn none() -> Self {
        Self {
            max_retries: 0,
            ..Default::default()
        }
    }

    /// Calculate the delay for a given retry attempt (0-indexed).
    fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let base = self.base_delay.as_millis() as f64;
        let exponential = base * 2f64.powi(attempt as i32);
        let capped = exponential.min(self.max_delay.as_millis() as f64);

        if self.jitter > 0.0 {
            let jitter_range = capped * self.jitter;
            let jitter_value = (rand::random::<f64>() * 2.0 - 1.0) * jitter_range;
            let with_jitter = (capped + jitter_value).max(0.0);
            Duration::from_millis(with_jitter as u64)
        } else {
            Duration::from_millis(capped as u64)
        }
    }
}

/// Execute an async operation with retry and exponential backoff.
///
/// Retries on `Err` up to `policy.max_retries` times with exponential backoff.
/// Returns the first `Ok` or the last `Err` if all retries are exhausted.
///
/// # Examples
///
/// ```ignore
/// let result = retry_with_backoff(&RetryPolicy::default(), || async {
///     fetch_data().await
/// }).await;
/// ```
pub async fn retry_with_backoff<F, Fut, T, E>(policy: &RetryPolicy, mut f: F) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut last_err = None;

    for attempt in 0..=policy.max_retries {
        match f().await {
            Ok(val) => return Ok(val),
            Err(e) => {
                if attempt < policy.max_retries {
                    let delay = policy.delay_for_attempt(attempt);
                    warn!(
                        "Attempt {}/{} failed: {}. Retrying in {:?}",
                        attempt + 1,
                        policy.max_retries + 1,
                        e,
                        delay
                    );
                    tokio::time::sleep(delay).await;
                }
                last_err = Some(e);
            }
        }
    }

    Err(last_err.expect("retry loop always sets last_err"))
}

/// Calculate delay for a given attempt. Exposed for testing.
#[cfg(test)]
pub fn delay_for_attempt(policy: &RetryPolicy, attempt: u32) -> Duration {
    policy.delay_for_attempt(attempt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_policy_default() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_retries, 3);
        assert_eq!(policy.base_delay, Duration::from_millis(100));
        assert_eq!(policy.max_delay, Duration::from_secs(10));
        assert!((policy.jitter - 0.2).abs() < f64::EPSILON);
    }

    #[test]
    fn test_retry_policy_none() {
        let policy = RetryPolicy::none();
        assert_eq!(policy.max_retries, 0);
    }

    #[test]
    fn test_delay_exponential_growth() {
        let policy = RetryPolicy {
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            jitter: 0.0,
            ..Default::default()
        };

        let d0 = delay_for_attempt(&policy, 0);
        let d1 = delay_for_attempt(&policy, 1);
        let d2 = delay_for_attempt(&policy, 2);

        assert_eq!(d0, Duration::from_millis(100));
        assert_eq!(d1, Duration::from_millis(200));
        assert_eq!(d2, Duration::from_millis(400));
    }

    #[test]
    fn test_delay_capped_at_max() {
        let policy = RetryPolicy {
            base_delay: Duration::from_millis(1000),
            max_delay: Duration::from_secs(2),
            jitter: 0.0,
            ..Default::default()
        };

        let d10 = delay_for_attempt(&policy, 10);
        assert_eq!(d10, Duration::from_secs(2));
    }

    #[tokio::test]
    async fn test_retry_succeeds_on_second_attempt() {
        use std::sync::atomic::{AtomicU32, Ordering};

        let policy = RetryPolicy {
            max_retries: 3,
            base_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(10),
            jitter: 0.0,
        };

        let call_count = std::sync::Arc::new(AtomicU32::new(0));
        let cc = call_count.clone();
        let result = retry_with_backoff(&policy, || {
            let cc = cc.clone();
            async move {
                let count = cc.fetch_add(1, Ordering::Relaxed) + 1;
                if count < 2 { Err("not yet") } else { Ok(42) }
            }
        })
        .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count.load(Ordering::Relaxed), 2);
    }

    #[tokio::test]
    async fn test_retry_exhausted() {
        let policy = RetryPolicy {
            max_retries: 2,
            base_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(10),
            jitter: 0.0,
        };

        let result: Result<(), &str> = retry_with_backoff(&policy, || async { Err("always fail") }).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "always fail");
    }

    #[tokio::test]
    async fn test_retry_no_retries() {
        use std::sync::atomic::{AtomicU32, Ordering};

        let policy = RetryPolicy::none();
        let call_count = std::sync::Arc::new(AtomicU32::new(0));
        let cc = call_count.clone();

        let result: Result<(), &str> = retry_with_backoff(&policy, || {
            let cc = cc.clone();
            async move {
                cc.fetch_add(1, Ordering::Relaxed);
                Err("fail")
            }
        })
        .await;

        assert!(result.is_err());
        assert_eq!(call_count.load(Ordering::Relaxed), 1);
    }
}
