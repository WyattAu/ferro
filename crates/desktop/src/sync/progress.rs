use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct SyncProgress {
    pub total_files: AtomicU64,
    pub completed_files: AtomicU64,
    pub total_bytes: AtomicU64,
    pub completed_bytes: AtomicU64,
    pub current_file: RwLock<Option<String>>,
    pub errors: AtomicU64,
    pub start_time: Instant,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncSummary {
    pub total_files: u64,
    pub completed_files: u64,
    pub total_bytes: u64,
    pub completed_bytes: u64,
    pub errors: u64,
    pub elapsed: Duration,
    pub bytes_per_second: f64,
}

impl SyncProgress {
    pub fn new() -> Self {
        Self {
            total_files: AtomicU64::new(0),
            completed_files: AtomicU64::new(0),
            total_bytes: AtomicU64::new(0),
            completed_bytes: AtomicU64::new(0),
            current_file: RwLock::new(None),
            errors: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    pub fn record_file(&self, size: u64) {
        self.completed_files.fetch_add(1, Ordering::SeqCst);
        self.completed_bytes.fetch_add(size, Ordering::SeqCst);
    }

    pub fn record_error(&self) {
        self.errors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn to_summary(&self) -> SyncSummary {
        let total_files = self.total_files.load(Ordering::SeqCst);
        let completed_files = self.completed_files.load(Ordering::SeqCst);
        let total_bytes = self.total_bytes.load(Ordering::SeqCst);
        let completed_bytes = self.completed_bytes.load(Ordering::SeqCst);
        let errors = self.errors.load(Ordering::SeqCst);
        let elapsed = self.start_time.elapsed();
        let bytes_per_second = self.bytes_per_second();

        SyncSummary {
            total_files,
            completed_files,
            total_bytes,
            completed_bytes,
            errors,
            elapsed,
            bytes_per_second,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.total_files.load(Ordering::SeqCst) == self.completed_files.load(Ordering::SeqCst)
            && self.total_files.load(Ordering::SeqCst) > 0
    }

    pub fn bytes_per_second(&self) -> f64 {
        let completed_bytes = self.completed_bytes.load(Ordering::SeqCst) as f64;
        let elapsed_secs = self.start_time.elapsed().as_secs_f64();
        if elapsed_secs > 0.0 {
            completed_bytes / elapsed_secs
        } else {
            0.0
        }
    }
}

impl Default for SyncProgress {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_initial_state() {
        let progress = SyncProgress::new();
        assert_eq!(progress.total_files.load(Ordering::SeqCst), 0);
        assert_eq!(progress.completed_files.load(Ordering::SeqCst), 0);
        assert_eq!(progress.errors.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_record_file() {
        let progress = SyncProgress::new();
        progress.record_file(1024);
        assert_eq!(progress.completed_files.load(Ordering::SeqCst), 1);
        assert_eq!(progress.completed_bytes.load(Ordering::SeqCst), 1024);

        progress.record_file(2048);
        assert_eq!(progress.completed_files.load(Ordering::SeqCst), 2);
        assert_eq!(progress.completed_bytes.load(Ordering::SeqCst), 3072);
    }

    #[test]
    fn test_record_error() {
        let progress = SyncProgress::new();
        progress.record_error();
        progress.record_error();
        assert_eq!(progress.errors.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_to_summary() {
        let progress = SyncProgress::new();
        progress.total_files.store(10, Ordering::SeqCst);
        progress.total_bytes.store(5000, Ordering::SeqCst);
        progress.record_file(1000);
        progress.record_file(2000);
        progress.record_error();

        let summary = progress.to_summary();
        assert_eq!(summary.total_files, 10);
        assert_eq!(summary.completed_files, 2);
        assert_eq!(summary.total_bytes, 5000);
        assert_eq!(summary.completed_bytes, 3000);
        assert_eq!(summary.errors, 1);
    }

    #[test]
    fn test_is_complete() {
        let progress = SyncProgress::new();

        progress.total_files.store(0, Ordering::SeqCst);
        assert!(!progress.is_complete());

        progress.total_files.store(3, Ordering::SeqCst);
        progress.completed_files.store(3, Ordering::SeqCst);
        assert!(progress.is_complete());

        progress.completed_files.store(2, Ordering::SeqCst);
        assert!(!progress.is_complete());
    }

    #[test]
    fn test_bytes_per_second_zero_elapsed() {
        let progress = SyncProgress::new();
        progress.record_file(1000);
        let bps = progress.bytes_per_second();
        assert!(bps >= 0.0);
    }

    #[test]
    fn test_summary_serialization() {
        let summary = SyncSummary {
            total_files: 10,
            completed_files: 5,
            total_bytes: 5000,
            completed_bytes: 2500,
            errors: 1,
            elapsed: Duration::from_secs(2),
            bytes_per_second: 1250.0,
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"total_files\":10"));
        assert!(json.contains("\"errors\":1"));
    }

    #[test]
    fn test_default_trait() {
        let progress = SyncProgress::default();
        assert_eq!(progress.total_files.load(Ordering::SeqCst), 0);
    }
}
