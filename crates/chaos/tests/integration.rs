use ferro_chaos::cpu::CpuFault;
use ferro_chaos::disk::DiskFault;
use ferro_chaos::memory::MemoryFault;
use ferro_chaos::network::NetworkFault;
use ferro_chaos::{ChaosExperiment, ChaosOrchestrator};
use std::time::Duration;

#[tokio::test]
async fn test_network_partition() {
    let mut orchestrator = ChaosOrchestrator::new();

    let experiment = ChaosExperiment {
        name: "Network Partition Test".to_string(),
        description: "Test system behavior during network partition".to_string(),
        network_faults: vec![NetworkFault::Partition {
            duration: Duration::from_millis(100),
        }],
        disk_faults: vec![],
        memory_faults: vec![],
        cpu_faults: vec![],
    };

    orchestrator.add_experiment(experiment);
    orchestrator.run().await;
}

#[tokio::test]
async fn test_latency_injection() {
    let mut orchestrator = ChaosOrchestrator::new();

    let experiment = ChaosExperiment {
        name: "Latency Injection Test".to_string(),
        description: "Test system behavior under network latency".to_string(),
        network_faults: vec![NetworkFault::Latency {
            min: Duration::from_millis(10),
            max: Duration::from_millis(50),
        }],
        disk_faults: vec![],
        memory_faults: vec![],
        cpu_faults: vec![],
    };

    orchestrator.add_experiment(experiment);
    orchestrator.run().await;
}

#[tokio::test]
async fn test_packet_loss() {
    let mut orchestrator = ChaosOrchestrator::new();

    let experiment = ChaosExperiment {
        name: "Packet Loss Test".to_string(),
        description: "Test system behavior under packet loss".to_string(),
        network_faults: vec![NetworkFault::PacketLoss { probability: 0.1 }],
        disk_faults: vec![],
        memory_faults: vec![],
        cpu_faults: vec![],
    };

    orchestrator.add_experiment(experiment);
    orchestrator.run().await;
}

#[tokio::test]
async fn test_dns_failure() {
    let mut orchestrator = ChaosOrchestrator::new();

    let experiment = ChaosExperiment {
        name: "DNS Failure Test".to_string(),
        description: "Test system behavior during DNS failure".to_string(),
        network_faults: vec![NetworkFault::DnsFailure {
            duration: Duration::from_millis(200),
        }],
        disk_faults: vec![],
        memory_faults: vec![],
        cpu_faults: vec![],
    };

    orchestrator.add_experiment(experiment);
    orchestrator.run().await;
}

#[tokio::test]
async fn test_combined_faults() {
    let mut orchestrator = ChaosOrchestrator::new();

    let experiment = ChaosExperiment {
        name: "Combined Faults Test".to_string(),
        description: "Test system behavior under multiple simultaneous faults".to_string(),
        network_faults: vec![NetworkFault::Latency {
            min: Duration::from_millis(5),
            max: Duration::from_millis(20),
        }],
        disk_faults: vec![],
        memory_faults: vec![],
        cpu_faults: vec![],
    };

    orchestrator.add_experiment(experiment);
    orchestrator.run().await;
}

#[tokio::test]
async fn test_disk_failure() {
    let mut orchestrator = ChaosOrchestrator::new();

    let experiment = ChaosExperiment {
        name: "Disk Failure Test".to_string(),
        description: "Test system behavior during disk I/O errors".to_string(),
        network_faults: vec![],
        disk_faults: vec![
            DiskFault::IoError {
                path: "/data".to_string(),
                probability: 0.05,
            },
            DiskFault::SlowDisk {
                path: "/data".to_string(),
                delay_ms: 100,
            },
        ],
        memory_faults: vec![],
        cpu_faults: vec![],
    };

    orchestrator.add_experiment(experiment);
    orchestrator.run().await;
}

#[tokio::test]
async fn test_memory_pressure() {
    let mut orchestrator = ChaosOrchestrator::new();

    let experiment = ChaosExperiment {
        name: "Memory Pressure Test".to_string(),
        description: "Test system behavior under memory pressure".to_string(),
        network_faults: vec![],
        disk_faults: vec![],
        memory_faults: vec![
            MemoryFault::Pressure { mb: 1024 },
            MemoryFault::Leak { bytes_per_sec: 1024 },
        ],
        cpu_faults: vec![],
    };

    orchestrator.add_experiment(experiment);
    orchestrator.run().await;
}

#[tokio::test]
async fn test_cpu_saturation() {
    let mut orchestrator = ChaosOrchestrator::new();

    let experiment = ChaosExperiment {
        name: "CPU Saturation Test".to_string(),
        description: "Test system behavior under high CPU load".to_string(),
        network_faults: vec![],
        disk_faults: vec![],
        memory_faults: vec![],
        cpu_faults: vec![CpuFault::HighUsage {
            cores: 2,
            duration: Duration::from_secs(5),
        }],
    };

    orchestrator.add_experiment(experiment);
    orchestrator.run().await;
}
