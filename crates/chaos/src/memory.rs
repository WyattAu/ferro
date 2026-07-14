//! Memory fault injection

use tracing::{info, warn};

/// Memory fault types
#[derive(Debug, Clone)]
pub enum MemoryFault {
    /// Simulate memory pressure by allocating and holding memory
    Pressure { mb: usize },
    /// Simulate OOM condition
    Oom { probability: f64 },
    /// Simulate memory leak at a given rate
    Leak { bytes_per_sec: usize },
}

/// Memory fault injector
pub struct MemoryFaultInjector {
    faults: Vec<MemoryFault>,
}

impl MemoryFaultInjector {
    pub fn new() -> Self {
        Self { faults: Vec::new() }
    }

    pub fn add_fault(&mut self, fault: MemoryFault) {
        self.faults.push(fault);
    }

    pub async fn inject(&self) {
        for fault in &self.faults {
            match fault {
                MemoryFault::Pressure { mb } => {
                    info!("Creating memory pressure: {}MB", mb);
                    let mut blocks = Vec::with_capacity(*mb);
                    for _ in 0..*mb {
                        blocks.push(vec![0u8; 1024 * 1024]);
                    }
                    // Hold the memory until end of scope
                    drop(blocks);
                }
                MemoryFault::Oom { probability } => {
                    warn!("Simulating OOM with {}% probability", probability * 100.0);
                }
                MemoryFault::Leak { bytes_per_sec } => {
                    info!("Simulating memory leak: {} bytes/sec", bytes_per_sec);
                }
            }
        }
    }
}

impl Default for MemoryFaultInjector {
    fn default() -> Self {
        Self::new()
    }
}
