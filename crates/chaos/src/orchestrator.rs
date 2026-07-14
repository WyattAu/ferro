//! Chaos orchestration

use crate::{cpu, disk, memory, network};
use tracing::info;

/// Chaos experiment configuration
#[derive(Debug, Clone)]
pub struct ChaosExperiment {
    pub name: String,
    pub description: String,
    pub network_faults: Vec<network::NetworkFault>,
    pub disk_faults: Vec<disk::DiskFault>,
    pub memory_faults: Vec<memory::MemoryFault>,
    pub cpu_faults: Vec<cpu::CpuFault>,
}

/// Chaos orchestrator for running fault injection experiments
pub struct ChaosOrchestrator {
    experiments: Vec<ChaosExperiment>,
}

impl ChaosOrchestrator {
    pub fn new() -> Self {
        Self {
            experiments: Vec::new(),
        }
    }

    pub fn add_experiment(&mut self, experiment: ChaosExperiment) {
        self.experiments.push(experiment);
    }

    pub async fn run(&self) {
        for experiment in &self.experiments {
            info!("Running chaos experiment: {}", experiment.name);
            info!("Description: {}", experiment.description);

            let mut network_injector = network::NetworkFaultInjector::new();
            for fault in &experiment.network_faults {
                network_injector.add_fault(fault.clone());
            }
            network_injector.inject().await;

            let mut disk_injector = disk::DiskFaultInjector::new();
            for fault in &experiment.disk_faults {
                disk_injector.add_fault(fault.clone());
            }
            disk_injector.inject().await;

            let mut memory_injector = memory::MemoryFaultInjector::new();
            for fault in &experiment.memory_faults {
                memory_injector.add_fault(fault.clone());
            }
            memory_injector.inject().await;

            let mut cpu_injector = cpu::CpuFaultInjector::new();
            for fault in &experiment.cpu_faults {
                cpu_injector.add_fault(fault.clone());
            }
            cpu_injector.inject().await;

            info!("Completed chaos experiment: {}", experiment.name);
        }
    }
}

impl Default for ChaosOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}
