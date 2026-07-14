use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Lightweight performance profiler for critical paths.
///
/// Collects timing samples and computes percentiles. Thread-safe for
/// concurrent use across async tasks.
pub struct Profiler {
    samples: Mutex<Vec<Duration>>,
    max_samples: usize,
    total_count: AtomicU64,
    total_duration_ns: AtomicU64,
    enabled_flag: AtomicBool,
}

impl Profiler {
    /// Create a new profiler. When `enabled` is false, `record` is a no-op.
    pub fn new(enabled: bool) -> Self {
        Self {
            samples: Mutex::new(Vec::with_capacity(1024)),
            max_samples: 10_000,
            total_count: AtomicU64::new(0),
            total_duration_ns: AtomicU64::new(0),
            enabled_flag: AtomicBool::new(enabled),
        }
    }

    /// Record a timing sample. No-op when profiler is disabled or at capacity.
    pub fn record(&self, duration: Duration) {
        if !self.enabled_flag.load(Ordering::Relaxed) {
            return;
        }
        self.total_count.fetch_add(1, Ordering::Relaxed);
        self.total_duration_ns
            .fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
        if let Ok(mut samples) = self.samples.lock()
            && samples.len() < self.max_samples
        {
            samples.push(duration);
        }
    }

    /// Compute percentiles from collected samples.
    pub fn percentiles(&self) -> Vec<(String, Duration)> {
        let samples = match self.samples.lock() {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        if samples.is_empty() {
            return Vec::new();
        }

        let mut sorted = samples.clone();
        sorted.sort();

        let len = sorted.len();
        let p = |pct: f64| -> Duration {
            let idx = ((len as f64) * pct / 100.0).min((len - 1) as f64) as usize;
            sorted[idx]
        };

        vec![
            ("p50".into(), p(50.0)),
            ("p90".into(), p(90.0)),
            ("p95".into(), p(95.0)),
            ("p99".into(), p(99.0)),
            ("p99.9".into(), p(99.9)),
        ]
    }

    /// Return the total number of recorded samples.
    pub fn sample_count(&self) -> usize {
        self.samples.lock().map(|s| s.len()).unwrap_or(0)
    }

    /// Return the total number of calls (including dropped samples).
    pub fn total_count(&self) -> u64 {
        self.total_count.load(Ordering::Relaxed)
    }

    /// Return the mean duration across all recorded calls.
    pub fn mean(&self) -> Duration {
        let count = self.total_count.load(Ordering::Relaxed);
        if count == 0 {
            return Duration::ZERO;
        }
        let total_ns = self.total_duration_ns.load(Ordering::Relaxed);
        Duration::from_nanos(total_ns / count)
    }

    /// Return the minimum and maximum durations from collected samples.
    pub fn min_max(&self) -> (Duration, Duration) {
        let samples = match self.samples.lock() {
            Ok(s) => s,
            Err(_) => return (Duration::ZERO, Duration::ZERO),
        };
        if samples.is_empty() {
            return (Duration::ZERO, Duration::ZERO);
        }
        let mut min = Duration::MAX;
        let mut max = Duration::ZERO;
        for &d in samples.iter() {
            if d < min {
                min = d;
            }
            if d > max {
                max = d;
            }
        }
        (min, max)
    }

    /// Clear all collected samples and reset counters.
    pub fn reset(&self) {
        if let Ok(mut samples) = self.samples.lock() {
            samples.clear();
        }
        self.total_count.store(0, Ordering::Relaxed);
        self.total_duration_ns.store(0, Ordering::Relaxed);
    }

    /// Enable or disable the profiler at runtime.
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled_flag.store(enabled, Ordering::Relaxed);
    }

    /// Check if the profiler is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled_flag.load(Ordering::Relaxed)
    }
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new(false)
    }
}

/// Scoped guard that records elapsed time when dropped.
pub struct ProfileGuard<'a> {
    start: Instant,
    profiler: &'a Profiler,
}

impl<'a> ProfileGuard<'a> {
    pub fn new(profiler: &'a Profiler) -> Self {
        Self {
            start: Instant::now(),
            profiler,
        }
    }
}

impl Drop for ProfileGuard<'_> {
    fn drop(&mut self) {
        self.profiler.record(self.start.elapsed());
    }
}

/// Macro for profiling a code block with a profiler reference.
///
/// # Example
/// ```ignore
/// let result = profile!(profiler, {
///     expensive_operation().await
/// });
/// ```
#[macro_export]
macro_rules! profile {
    ($profiler:expr, $block:expr) => {{
        let _guard = $crate::profiler::ProfileGuard::new(&$profiler);
        $block
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiler_disabled_is_noop() {
        let p = Profiler::new(false);
        p.record(Duration::from_millis(10));
        assert_eq!(p.sample_count(), 0);
        assert_eq!(p.total_count(), 0);
    }

    #[test]
    fn test_profiler_records_samples() {
        let p = Profiler::new(true);
        p.record(Duration::from_millis(1));
        p.record(Duration::from_millis(5));
        p.record(Duration::from_millis(10));
        assert_eq!(p.sample_count(), 3);
        assert_eq!(p.total_count(), 3);
    }

    #[test]
    fn test_profiler_percentiles() {
        let p = Profiler::new(true);
        for i in 1..=100 {
            p.record(Duration::from_micros(i));
        }
        let pct = p.percentiles();
        assert!(!pct.is_empty());
        assert_eq!(pct[0].0, "p50");
    }

    #[test]
    fn test_profiler_mean() {
        let p = Profiler::new(true);
        p.record(Duration::from_millis(10));
        p.record(Duration::from_millis(20));
        let mean = p.mean();
        assert_eq!(mean, Duration::from_millis(15));
    }

    #[test]
    fn test_profiler_min_max() {
        let p = Profiler::new(true);
        p.record(Duration::from_millis(5));
        p.record(Duration::from_millis(50));
        p.record(Duration::from_millis(10));
        let (min, max) = p.min_max();
        assert_eq!(min, Duration::from_millis(5));
        assert_eq!(max, Duration::from_millis(50));
    }

    #[test]
    fn test_profiler_reset() {
        let p = Profiler::new(true);
        p.record(Duration::from_millis(10));
        p.reset();
        assert_eq!(p.sample_count(), 0);
        assert_eq!(p.total_count(), 0);
    }

    #[test]
    fn test_profiler_max_samples_cap() {
        let p = Profiler::new(true);
        // max_samples is 10_000 by default; add more to test cap
        for _ in 0..11_000 {
            p.record(Duration::from_millis(1));
        }
        assert_eq!(p.sample_count(), 10_000);
        assert_eq!(p.total_count(), 11_000);
    }

    #[test]
    fn test_profile_macro() {
        let p = Profiler::new(true);
        let result = profile!(p, 42);
        assert_eq!(result, 42);
        assert_eq!(p.total_count(), 1);
    }

    #[test]
    fn test_set_enabled() {
        let p = Profiler::new(false);
        assert!(!p.is_enabled());
        p.set_enabled(true);
        assert!(p.is_enabled());
        p.record(Duration::from_millis(1));
        assert_eq!(p.total_count(), 1);
    }
}
