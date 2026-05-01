pub mod counter;
pub mod exporter;
pub mod gauge;
pub mod histogram;
pub mod labels;
pub mod log_buffer;
#[cfg(feature = "endpoints")]
pub mod loki;
pub mod metrics;
pub mod registry;
#[cfg(feature = "endpoints")]
pub mod router;
#[cfg(feature = "endpoints")]
pub mod victoria_logs;
#[cfg(feature = "endpoints")]
pub mod victoria_metrics;

#[cfg(test)]
mod tests;

pub use counter::Counter;
pub use exporter::export_prometheus;
pub use gauge::Gauge;
pub use histogram::Histogram;
pub use labels::Labels;
pub use log_buffer::{LogBuffer, LogEntry};
pub use registry::{MetricsRegistry, global_registry};

#[cfg(feature = "endpoints")]
pub use router::{ObservabilityState, build_observability_router};
