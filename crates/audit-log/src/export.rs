use crate::AuditEntry;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExportFormat {
    Json,
    Csv,
}

pub fn export_json(entries: &[AuditEntry]) -> Result<String, crate::AuditError> {
    serde_json::to_string_pretty(entries).map_err(|e| crate::AuditError::Export(e.to_string()))
}

pub fn export_csv(entries: &[AuditEntry]) -> Result<String, crate::AuditError> {
    if entries.is_empty() {
        return Ok(String::new());
    }

    let mut w = csv::StringRecord::new();
    w.push_field("id");
    w.push_field("timestamp");
    w.push_field("action");
    w.push_field("actor_id");
    w.push_field("resource_type");
    w.push_field("resource_id");
    w.push_field("details");
    w.push_field("ip_address");
    w.push_field("user_agent");
    w.push_field("previous_hash");
    w.push_field("hash");

    let mut output = w.to_line();
    let mut action_buf = String::new();
    let mut resource_type_buf = String::new();

    for entry in entries {
        action_buf.clear();
        resource_type_buf.clear();
        std::fmt::write(&mut action_buf, format_args!("{:?}", entry.action)).ok();
        std::fmt::write(&mut resource_type_buf, format_args!("{:?}", entry.resource_type)).ok();

        let mut record = csv::StringRecord::new();
        record.push_field(&entry.id);
        record.push_field(&entry.timestamp.to_rfc3339());
        record.push_field(&action_buf);
        record.push_field(&entry.actor_id);
        record.push_field(&resource_type_buf);
        record.push_field(&entry.resource_id);
        record.push_field(&serde_json::to_string(&entry.details).unwrap_or_default());
        record.push_field(entry.ip_address.as_deref().unwrap_or(""));
        record.push_field(entry.user_agent.as_deref().unwrap_or(""));
        record.push_field(&entry.previous_hash);
        record.push_field(&entry.hash);
        output.push_str(&record.to_line());
    }

    Ok(output)
}

mod csv {
    pub struct StringRecord {
        fields: Vec<String>,
    }

    impl StringRecord {
        pub fn new() -> Self {
            Self { fields: Vec::new() }
        }

        pub fn push_field(&mut self, s: &str) {
            self.fields.push(escape_csv_field(s));
        }

        pub fn to_line(&self) -> String {
            let line = self.fields.join(",");
            format!("{line}\n")
        }
    }

    fn escape_csv_field(s: &str) -> String {
        if s.contains(',') || s.contains('"') || s.contains('\n') {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuditAction, ResourceType};
    use chrono::Utc;
    use std::collections::HashMap;

    fn make_entry(id: &str) -> AuditEntry {
        AuditEntry {
            id: id.to_string(),
            timestamp: Utc::now(),
            action: AuditAction::FileCreate,
            actor_id: "user-1".to_string(),
            resource_type: ResourceType::File,
            resource_id: format!("file-{id}"),
            details: HashMap::new(),
            ip_address: Some("1.2.3.4".to_string()),
            user_agent: None,
            previous_hash: String::new(),
            hash: "abc123".to_string(),
        }
    }

    #[test]
    fn test_export_json_valid() {
        let entries = vec![make_entry("1"), make_entry("2")];
        let json = export_json(&entries).unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0]["id"], "1");
    }

    #[test]
    fn test_export_json_empty() {
        let json = export_json(&[]).unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_empty());
    }

    #[test]
    fn test_export_csv_valid() {
        let entries = vec![make_entry("1")];
        let csv = export_csv(&entries).unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("id,timestamp"));
        assert!(lines[1].contains("1"));
    }

    #[test]
    fn test_export_csv_empty() {
        let csv = export_csv(&[]).unwrap();
        assert!(csv.is_empty());
    }

    #[test]
    fn test_export_csv_with_comma_in_field() {
        let mut entry = make_entry("1");
        entry.user_agent = Some("Mozilla, Firefox".to_string());
        let csv = export_csv(&[entry]).unwrap();
        assert!(csv.contains("\"Mozilla, Firefox\""));
    }
}
