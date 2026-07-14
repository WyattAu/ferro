# Audit Logging

## Overview

Ferro provides comprehensive audit logging for compliance and security.

## Log Types

### Authentication Logs
- Login attempts (success/failure)
- Logout events
- Password changes
- MFA events

### Data Access Logs
- Calendar reads
- Event creates/updates/deletes
- Contact reads
- Contact creates/updates/deletes

### Administrative Logs
- User creation/deletion
- Permission changes
- System configuration changes

## Log Format

```json
{
  "timestamp": "2024-01-01T00:00:00Z",
  "level": "INFO",
  "event": "authentication.login",
  "user_id": "user123",
  "ip_address": "192.168.1.1",
  "user_agent": "Mozilla/5.0",
  "resource": "/auth/login",
  "action": "POST",
  "status": "success",
  "details": {}
}
```

## Configuration

```toml
[audit]
enabled = true
log_path = "/var/log/ferro/audit.log"
retention_days = 365
include_ip_addresses = true
include_user_agents = true
```

## API Endpoints

### Query Audit Logs
```http
GET /api/audit/logs?start=2024-01-01&end=2024-01-31&event=authentication.login
```

### Export Audit Logs
```http
GET /api/audit/logs/export?format=csv&start=2024-01-01&end=2024-01-31
```

## Implementation

### Audit Logger

```rust
pub struct AuditLogger {
    enabled: bool,
    log_path: Option<PathBuf>,
    retention_days: u32,
}

impl AuditLogger {
    pub fn log_event(&self, event: AuditEvent) -> Result<(), AuditError> {
        if !self.enabled {
            return Ok(());
        }

        let json = serde_json::to_string(&event)?;
        
        if let Some(path) = &self.log_path {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .and_then(|mut file| {
                    use std::io::Write;
                    writeln!(file, "{}", json)
                })
                .map_err(|e| AuditError::IoError(e))?;
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct AuditEvent {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub event: String,
    pub user_id: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub resource: String,
    pub action: String,
    pub status: String,
    pub details: serde_json::Value,
}
```
