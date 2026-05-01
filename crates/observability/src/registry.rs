use std::sync::Arc;

use parking_lot::RwLock;

use crate::{Counter, Gauge, Histogram};

pub enum MetricEntry {
    Counter(Arc<Counter>),
    Gauge(Arc<Gauge>),
    Histogram(Arc<Histogram>),
}

pub struct MetricsRegistry {
    entries: RwLock<Vec<(String, String, MetricEntry)>>,
}

impl MetricsRegistry {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
        }
    }

    pub fn register_counter(&self, name: &str, help: &str) -> Arc<Counter> {
        let counter = Arc::new(Counter::new(name, help));
        self.entries.write().push((
            name.to_string(),
            help.to_string(),
            MetricEntry::Counter(Arc::clone(&counter)),
        ));
        counter
    }

    pub fn register_gauge(&self, name: &str, help: &str) -> Arc<Gauge> {
        let gauge = Arc::new(Gauge::new(name, help));
        self.entries.write().push((
            name.to_string(),
            help.to_string(),
            MetricEntry::Gauge(Arc::clone(&gauge)),
        ));
        gauge
    }

    pub fn register_histogram(&self, name: &str, help: &str, buckets: Vec<f64>) -> Arc<Histogram> {
        let histogram = Arc::new(Histogram::new(name, help, buckets));
        self.entries.write().push((
            name.to_string(),
            help.to_string(),
            MetricEntry::Histogram(Arc::clone(&histogram)),
        ));
        histogram
    }

    pub fn entries(&self) -> parking_lot::RwLockReadGuard<'_, Vec<(String, String, MetricEntry)>> {
        self.entries.read()
    }
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub fn global_registry() -> &'static MetricsRegistry {
    use std::sync::OnceLock;
    static REGISTRY: OnceLock<MetricsRegistry> = OnceLock::new();
    REGISTRY.get_or_init(MetricsRegistry::new)
}
