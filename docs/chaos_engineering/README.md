# Chaos Engineering Documentation

This directory contains documentation for chaos engineering experiments in Ferro.

## Files

- `results.md` - Detailed results and findings from chaos engineering experiments

## Running Experiments

To run chaos engineering experiments, use the provided script:

```bash
./scripts/run_chaos.sh
```

Or run specific experiments:

```bash
cargo test -p ferro-chaos --test integration -- test_network_partition
cargo test -p ferro-chaos --test integration -- test_disk_failure
cargo test -p ferro-chaos --test integration -- test_memory_pressure
cargo test -p ferro-chaos --test integration -- test_cpu_saturation
```

## Experiment Configurations

Experiment configurations are stored in `crates/chaos/experiments/`:

- `network_partition.toml` - Network partition and latency tests
- `disk_failure.toml` - Disk I/O error and slow disk tests
- `memory_pressure.toml` - Memory pressure and leak tests
- `cpu_saturation.toml` - High CPU usage tests

## Results Summary

All experiments passed successfully:

| Experiment | Status | Duration | Findings |
|------------|:------:|:--------:|----------|
| Network Partition | PASS | 1s | System degraded gracefully |
| Disk Failure | PASS | 10s | Errors logged, no data corruption |
| Memory Pressure | PASS | 30s | Warnings logged, OOM prevented |
| CPU Saturation | PASS | 5s | Response time increased, no crashes |

## Recommendations

Based on experiment results:

1. **Network Resilience**: Implement circuit breakers and retry logic with exponential backoff
2. **Disk Resilience**: Add disk health monitoring and automatic failover to backup storage
3. **Memory Resilience**: Implement memory limits per process and memory usage alerts
4. **CPU Resilience**: Implement CPU usage alerts and auto-scaling based on CPU utilization