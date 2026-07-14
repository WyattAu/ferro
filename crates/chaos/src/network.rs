//! Network fault injection

use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

/// Network fault types
#[derive(Debug, Clone)]
pub enum NetworkFault {
    /// Simulate packet loss
    PacketLoss { probability: f64 },
    /// Simulate latency
    Latency { min: Duration, max: Duration },
    /// Simulate partition
    Partition { duration: Duration },
    /// Simulate DNS failure
    DnsFailure { duration: Duration },
}

/// Network fault injector
pub struct NetworkFaultInjector {
    faults: Vec<NetworkFault>,
}

impl NetworkFaultInjector {
    pub fn new() -> Self {
        Self { faults: Vec::new() }
    }

    pub fn add_fault(&mut self, fault: NetworkFault) {
        self.faults.push(fault);
    }

    pub async fn inject(&self) {
        for fault in &self.faults {
            match fault {
                NetworkFault::PacketLoss { probability } => {
                    info!("Injecting packet loss: {}%", probability * 100.0);
                }
                NetworkFault::Latency { min, max } => {
                    info!("Injecting latency: {:?} to {:?}", min, max);
                    let min_ms = min.as_millis() as u64;
                    let max_ms = max.as_millis() as u64;
                    let range = max_ms.saturating_sub(min_ms);
                    let delay = if range > 0 {
                        min_ms + (rand::random::<u64>() % range)
                    } else {
                        min_ms
                    };
                    sleep(Duration::from_millis(delay)).await;
                }
                NetworkFault::Partition { duration } => {
                    warn!("Injecting network partition for {:?}", duration);
                    sleep(*duration).await;
                }
                NetworkFault::DnsFailure { duration } => {
                    warn!("Injecting DNS failure for {:?}", duration);
                    sleep(*duration).await;
                }
            }
        }
    }
}

impl Default for NetworkFaultInjector {
    fn default() -> Self {
        Self::new()
    }
}
