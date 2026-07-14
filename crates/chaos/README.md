# Ferro Chaos Engineering

Netflix-style chaos engineering framework for resilience testing in Ferro.

## Overview

This crate provides fault injection capabilities for testing system resilience under various failure conditions. Inspired by Netflix's Chaos Monkey, it enables controlled experiments to validate system behavior during network partitions, disk failures, memory pressure, and CPU saturation.

## Features

- **Network Fault Injection**: Simulate packet loss, latency, partitions, and DNS failures
- **Disk Fault Injection**: Simulate I/O errors, slow disks, and disk full conditions
- **Memory Fault Injection**: Simulate memory pressure, OOM conditions, and memory leaks
- **CPU Fault Injection**: Simulate high CPU usage and throttling
- **Orchestration**: Run multiple fault experiments in sequence

## Usage

### Running Experiments

```bash
# Run all chaos experiments
./scripts/run_chaos.sh

# Run specific experiment
cargo test -p ferro-chaos --test integration -- test_network_partition
```

### Experiment Configurations

Experiment configurations are stored in `experiments/` directory:

- `network_partition.toml` - Network partition and latency tests
- `disk_failure.toml` - Disk I/O error and slow disk tests
- `memory_pressure.toml` - Memory pressure and leak tests
- `cpu_saturation.toml` - High CPU usage tests

### Programmatic Usage

```rust
use ferro_chaos::{ChaosExperiment, ChaosOrchestrator};
use ferro_chaos::network::NetworkFault;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let mut orchestrator = ChaosOrchestrator::new();
    
    let experiment = ChaosExperiment {
        name: "Network Partition Test".to_string(),
        description: "Test system behavior during network partition".to_string(),
        network_faults: vec![NetworkFault::Partition {
            duration: Duration::from_secs(1),
        }],
        disk_faults: vec![],
        memory_faults: vec![],
        cpu_faults: vec![],
    };
    
    orchestrator.add_experiment(experiment);
    orchestrator.run().await;
}
```

## Fault Types

### Network Faults

- `PacketLoss { probability }` - Simulate packet loss with given probability
- `Latency { min, max }` - Inject random latency between min and max
- `Partition { duration }` - Simulate network partition for specified duration
- `DnsFailure { duration }` - Simulate DNS resolution failure

### Disk Faults

- `DiskFull { path }` - Simulate disk full condition
- `IoError { path, probability }` - Simulate I/O errors with given probability
- `SlowDisk { path, delay_ms }` - Simulate slow disk with specified delay

### Memory Faults

- `Pressure { mb }` - Allocate and hold specified MB of memory
- `Oom { probability }` - Simulate OOM condition with given probability
- `Leak { bytes_per_sec }` - Simulate memory leak at specified rate

### CPU Faults

- `HighUsage { cores, duration }` - Spin on specified cores for duration
- `Throttle { factor }` - Simulate CPU throttling

## Results

Chaos engineering experiment results are documented in `docs/chaos_engineering/results.md`.

## Integration with CI

The chaos experiments can be integrated into CI pipelines to ensure system resilience is continuously validated. Add the following to your CI configuration:

```yaml
- name: Run Chaos Experiments
  run: ./scripts/run_chaos.sh
```