use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug)]
pub struct Histogram {
    name: String,
    help: String,
    buckets: Vec<f64>,
    counts: Vec<AtomicU64>,
    sum: AtomicU64,
    count: AtomicU64,
}

impl Histogram {
    pub fn new(name: &str, help: &str, buckets: Vec<f64>) -> Self {
        let counts = buckets.iter().map(|_| AtomicU64::new(0)).collect();
        Self {
            name: name.to_string(),
            help: help.to_string(),
            buckets,
            counts,
            sum: AtomicU64::new(0),
            count: AtomicU64::new(0),
        }
    }

    pub fn observe(&self, value: f64) {
        let v = value as u64;
        self.sum.fetch_add(v, Ordering::Relaxed);
        self.count.fetch_add(1, Ordering::Relaxed);
        for (i, bucket) in self.buckets.iter().enumerate() {
            if value <= *bucket {
                self.counts[i].fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    pub fn observe_duration(&self, duration: std::time::Duration) {
        self.observe(duration.as_secs_f64());
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn help(&self) -> &str {
        &self.help
    }

    pub fn buckets(&self) -> &[f64] {
        &self.buckets
    }

    pub fn bucket_count(&self, index: usize) -> u64 {
        self.counts[index].load(Ordering::Relaxed)
    }

    pub fn total_count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }

    pub fn total_sum(&self) -> u64 {
        self.sum.load(Ordering::Relaxed)
    }
}
