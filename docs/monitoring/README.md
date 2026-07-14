# Monitoring

## Overview

Ferro uses Prometheus for metrics collection and Grafana for visualization.

## Metrics

### Application Metrics
- `http_requests_total` - Total HTTP requests
- `http_request_duration_seconds` - Request duration histogram
- `http_requests_by_status` - Requests by status code

### System Metrics
- `process_resident_memory_bytes` - Memory usage
- `process_cpu_seconds_total` - CPU usage
- `database_connections_active` - Active database connections

### Business Metrics
- `users_total` - Total users
- `calendars_total` - Total calendars
- `events_total` - Total events

## Dashboards

### Ferro Dashboard
- Request rate
- Latency (p50, p95, p99)
- Error rate
- Memory usage
- Database connections

## Alerts

### Critical
- High error rate (>1%)
- Database connection pool exhausted
- Pod crash looping

### Warning
- High latency (>1s p99)
- High memory usage (>80%)
- High disk usage (>80%)

## Access

### Grafana
- URL: http://grafana:3000
- Username: admin
- Password: admin

### Prometheus
- URL: http://prometheus:9090