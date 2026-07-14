//! CPU fault injection

use std::time::Duration;
use tracing::{info, warn};

/// CPU fault types
#[derive(Debug, Clone)]
pub enum CpuFault {
    /// Simulate high CPU usage by spinning on specified cores
    HighUsage { cores: usize, duration: Duration },
    /// Simulate CPU throttling
    Throttle { factor: f64 },
}

/// CPU fault injector
pub struct CpuFaultInjector {
    faults: Vec<CpuFault>,
}

impl CpuFaultInjector {
    pub fn new() -> Self {
        Self { faults: Vec::new() }
    }

    pub fn add_fault(&mut self, fault: CpuFault) {
        self.faults.push(fault);
    }

    pub async fn inject(&self) {
        for fault in &self.faults {
            match fault {
                CpuFault::HighUsage { cores, duration } => {
                    info!("Creating high CPU usage on {} cores for {:?}", cores, duration);
                    let handles: Vec<_> = (0..*cores)
                        .map(|_| {
                            let dur = *duration;
                            std::thread::spawn(move || {
                                let start = std::time::Instant::now();
                                while start.elapsed() < dur {
                                    std::hint::spin_loop();
                                }
                            })
                        })
                        .collect();

                    for handle in handles {
                        let _ = handle.join();
                    }
                }
                CpuFault::Throttle { factor } => {
                    warn!("Simulating CPU throttle with factor {}", factor);
                }
            }
        }
    }
}

impl Default for CpuFaultInjector {
    fn default() -> Self {
        Self::new()
    }
}
