use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};

pub use ferro_dav::ical::{IcalComponent, IcalProperty, get_all_props, get_first_prop, parse_ical, serialize_ical};

use crate::error::{CalDavError, Result};

#[derive(Debug, Clone)]
pub enum EventStatus {
    Tentative,
    Confirmed,
    Cancelled,
}

impl EventStatus {
    pub fn parse(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "TENTATIVE" => EventStatus::Tentative,
            "CONFIRMED" => EventStatus::Confirmed,
            "CANCELLED" => EventStatus::Cancelled,
            _ => EventStatus::Confirmed,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CalendarEvent {
    pub uid: String,
    pub summary: String,
    pub description: Option<String>,
    pub start: DateTime<Utc>,
    pub end: Option<DateTime<Utc>>,
    pub location: Option<String>,
    pub attendees: Vec<String>,
    pub recurrence: Option<String>,
    pub status: EventStatus,
}

pub fn parse_ical_datetime(value: &str, params: &hashbrown::HashMap<String, String>) -> Option<DateTime<Utc>> {
    let is_date = params.get("VALUE").map(|v| v.as_str()) == Some("DATE");
    let cleaned = value.trim();

    if is_date {
        let parsed = NaiveDate::parse_from_str(cleaned, "%Y%m%d").ok()?;
        Some(parsed.and_hms_opt(0, 0, 0)?.and_utc())
    } else if let Some(without_z) = cleaned.strip_suffix('Z') {
        NaiveDateTime::parse_from_str(without_z, "%Y%m%dT%H%M%S")
            .ok()
            .map(|dt| dt.and_utc())
    } else {
        NaiveDateTime::parse_from_str(cleaned, "%Y%m%dT%H%M%S")
            .ok()
            .map(|dt| dt.and_utc())
    }
}

pub fn parse_timestamp(s: &str) -> Option<DateTime<Utc>> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.ends_with('Z') {
        let without_z = trimmed.strip_suffix('Z')?;
        NaiveDateTime::parse_from_str(without_z, "%Y%m%dT%H%M%S")
            .ok()
            .map(|dt| dt.and_utc())
    } else {
        NaiveDateTime::parse_from_str(trimmed, "%Y%m%dT%H%M%S")
            .ok()
            .map(|dt| dt.and_utc())
    }
}

pub fn extract_event_from_ical(ical: &str) -> Result<CalendarEvent> {
    let components = parse_ical(ical).map_err(CalDavError::InvalidData)?;

    let vevent = components.iter().find_map(|c| {
        if c.name == "VCALENDAR" {
            c.children.iter().find(|ch| ch.name == "VEVENT" || ch.name == "VTODO")
        } else if c.name == "VEVENT" || c.name == "VTODO" {
            Some(c)
        } else {
            None
        }
    });

    let vevent = vevent.ok_or_else(|| CalDavError::InvalidData("No VEVENT or VTODO found".to_string()))?;

    let uid = get_first_prop(vevent, "UID")
        .map(|p| p.value.clone())
        .unwrap_or_default();

    let summary = get_first_prop(vevent, "SUMMARY")
        .map(|p| p.value.clone())
        .unwrap_or_default();

    let description = get_first_prop(vevent, "DESCRIPTION").map(|p| p.value.clone());

    let location = get_first_prop(vevent, "LOCATION").map(|p| p.value.clone());

    let start = get_first_prop(vevent, "DTSTART")
        .and_then(|p| parse_ical_datetime(&p.value, &p.params))
        .unwrap_or_else(Utc::now);

    let end = get_first_prop(vevent, "DTEND").and_then(|p| parse_ical_datetime(&p.value, &p.params));

    let attendees = get_all_props(vevent, "ATTENDEE")
        .iter()
        .map(|p| p.value.clone())
        .collect();

    let recurrence = get_first_prop(vevent, "RRULE").map(|p| p.value.clone());

    let status = get_first_prop(vevent, "STATUS")
        .map(|p| EventStatus::parse(&p.value))
        .unwrap_or(EventStatus::Confirmed);

    Ok(CalendarEvent {
        uid,
        summary,
        description,
        start,
        end,
        location,
        attendees,
        recurrence,
        status,
    })
}

pub fn generate_ical_event(event: &CalendarEvent) -> String {
    let mut ical = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Ferro//CalDAV Server//EN\r\n");

    ical.push_str("BEGIN:VEVENT\r\n");
    ical.push_str(&format!("UID:{}\r\n", event.uid));
    ical.push_str(&format!("SUMMARY:{}\r\n", event.summary));

    ical.push_str(&format!("DTSTART:{}\r\n", event.start.format("%Y%m%dT%H%M%SZ")));

    if let Some(ref end) = event.end {
        ical.push_str(&format!("DTEND:{}\r\n", end.format("%Y%m%dT%H%M%SZ")));
    }

    if let Some(ref desc) = event.description {
        ical.push_str(&format!("DESCRIPTION:{}\r\n", desc));
    }

    if let Some(ref loc) = event.location {
        ical.push_str(&format!("LOCATION:{}\r\n", loc));
    }

    for attendee in &event.attendees {
        ical.push_str(&format!("ATTENDEE:{}\r\n", attendee));
    }

    if let Some(ref rrule) = event.recurrence {
        ical.push_str(&format!("RRULE:{}\r\n", rrule));
    }

    let status_str = match event.status {
        EventStatus::Tentative => "TENTATIVE",
        EventStatus::Confirmed => "CONFIRMED",
        EventStatus::Cancelled => "CANCELLED",
    };
    ical.push_str(&format!("STATUS:{}\r\n", status_str));
    ical.push_str("END:VEVENT\r\n");
    ical.push_str("END:VCALENDAR\r\n");

    ical
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_timestamp() {
        assert!(parse_timestamp("20240101T120000Z").is_some());
        assert!(parse_timestamp("20240101T120000").is_some());
        assert!(parse_timestamp("").is_none());
    }

    #[test]
    fn test_event_status_from_str() {
        assert!(matches!(EventStatus::parse("TENTATIVE"), EventStatus::Tentative));
        assert!(matches!(EventStatus::parse("CONFIRMED"), EventStatus::Confirmed));
        assert!(matches!(EventStatus::parse("CANCELLED"), EventStatus::Cancelled));
        assert!(matches!(EventStatus::parse("UNKNOWN"), EventStatus::Confirmed));
    }

    #[test]
    fn test_extract_event_from_valid_ical() {
        let ical = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:test-123\r\nSUMMARY:Meeting\r\nDTSTART:20240101T100000Z\r\nDTEND:20240101T110000Z\r\nDESCRIPTION:Team standup\r\nLOCATION:Room A\r\nSTATUS:CONFIRMED\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";

        let event = extract_event_from_ical(ical).unwrap();
        assert_eq!(event.uid, "test-123");
        assert_eq!(event.summary, "Meeting");
        assert_eq!(event.description.as_deref(), Some("Team standup"));
        assert_eq!(event.location.as_deref(), Some("Room A"));
        assert!(matches!(event.status, EventStatus::Confirmed));
    }

    #[test]
    fn test_generate_ical_event() {
        let event = CalendarEvent {
            uid: "gen-123".to_string(),
            summary: "Birthday".to_string(),
            description: Some("Cake!".to_string()),
            start: Utc::now(),
            end: None,
            location: None,
            attendees: vec![],
            recurrence: None,
            status: EventStatus::Confirmed,
        };

        let ical = generate_ical_event(&event);
        assert!(ical.contains("BEGIN:VCALENDAR"));
        assert!(ical.contains("UID:gen-123"));
        assert!(ical.contains("SUMMARY:Birthday"));
        assert!(ical.contains("DESCRIPTION:Cake!"));
        assert!(ical.contains("END:VCALENDAR"));
    }

    #[test]
    fn test_event_status_debug() {
        assert!(!format!("{:?}", EventStatus::Tentative).is_empty());
        assert!(!format!("{:?}", EventStatus::Confirmed).is_empty());
        assert!(!format!("{:?}", EventStatus::Cancelled).is_empty());
    }

    #[test]
    fn test_event_status_clone() {
        let status = EventStatus::Tentative.clone();
        assert!(matches!(status, EventStatus::Tentative));
    }

    #[test]
    fn test_parse_ical_datetime_with_z() {
        let params = hashbrown::HashMap::new();
        let result = parse_ical_datetime("20240101T120000Z", &params);
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_ical_datetime_without_z() {
        let params = hashbrown::HashMap::new();
        let result = parse_ical_datetime("20240101T120000", &params);
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_ical_datetime_date_value() {
        let mut params = hashbrown::HashMap::new();
        params.insert("VALUE".to_string(), "DATE".to_string());
        let result = parse_ical_datetime("20240101", &params);
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_ical_datetime_invalid() {
        let params = hashbrown::HashMap::new();
        assert!(parse_ical_datetime("not-a-date", &params).is_none());
    }

    #[test]
    fn test_parse_timestamp_invalid() {
        assert!(parse_timestamp("invalid").is_none());
        assert!(parse_timestamp("20240101").is_none());
    }

    #[test]
    fn test_extract_event_no_vevent() {
        let ical = "BEGIN:VCALENDAR\r\nBEGIN:VTODO\r\nUID:todo-1\r\nSUMMARY:Task\r\nEND:VTODO\r\nEND:VCALENDAR\r\n";
        let event = extract_event_from_ical(ical).unwrap();
        assert_eq!(event.uid, "todo-1");
        assert_eq!(event.summary, "Task");
    }

    #[test]
    fn test_extract_event_missing_fields() {
        let ical = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nDTSTART:20240101T100000Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let event = extract_event_from_ical(ical).unwrap();
        assert!(event.uid.is_empty());
        assert!(event.summary.is_empty());
    }

    #[test]
    fn test_extract_event_with_attendees() {
        let ical = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:att-1\r\nSUMMARY:Meeting\r\nDTSTART:20240101T100000Z\r\nATTENDEE:mailto:alice@example.com\r\nATTENDEE:mailto:bob@example.com\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let event = extract_event_from_ical(ical).unwrap();
        assert_eq!(event.attendees.len(), 2);
    }

    #[test]
    fn test_extract_event_with_recurrence() {
        let ical = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:recur-1\r\nSUMMARY:Weekly\r\nDTSTART:20240101T100000Z\r\nRRULE:FREQ=WEEKLY;COUNT=5\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let event = extract_event_from_ical(ical).unwrap();
        assert_eq!(event.recurrence.as_deref(), Some("FREQ=WEEKLY;COUNT=5"));
    }

    #[test]
    fn test_extract_event_tentative_status() {
        let ical = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:tent-1\r\nSUMMARY:Tentative\r\nDTSTART:20240101T100000Z\r\nSTATUS:TENTATIVE\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let event = extract_event_from_ical(ical).unwrap();
        assert!(matches!(event.status, EventStatus::Tentative));
    }

    #[test]
    fn test_extract_event_cancelled_status() {
        let ical = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:cancel-1\r\nSUMMARY:Cancelled\r\nDTSTART:20240101T100000Z\r\nSTATUS:CANCELLED\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let event = extract_event_from_ical(ical).unwrap();
        assert!(matches!(event.status, EventStatus::Cancelled));
    }

    #[test]
    fn test_extract_event_invalid_ical() {
        let result = extract_event_from_ical("not valid ical");
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_ical_event_with_location() {
        let event = CalendarEvent {
            uid: "loc-1".to_string(),
            summary: "Location Test".to_string(),
            description: None,
            start: Utc::now(),
            end: None,
            location: Some("Room B".to_string()),
            attendees: vec![],
            recurrence: None,
            status: EventStatus::Confirmed,
        };
        let ical = generate_ical_event(&event);
        assert!(ical.contains("LOCATION:Room B"));
    }

    #[test]
    fn test_generate_ical_event_with_recurrence() {
        let event = CalendarEvent {
            uid: "rrule-1".to_string(),
            summary: "Recurring".to_string(),
            description: None,
            start: Utc::now(),
            end: None,
            location: None,
            attendees: vec![],
            recurrence: Some("FREQ=DAILY".to_string()),
            status: EventStatus::Confirmed,
        };
        let ical = generate_ical_event(&event);
        assert!(ical.contains("RRULE:FREQ=DAILY"));
    }

    #[test]
    fn test_generate_ical_event_with_attendees() {
        let event = CalendarEvent {
            uid: "att-gen-1".to_string(),
            summary: "With Attendees".to_string(),
            description: None,
            start: Utc::now(),
            end: None,
            location: None,
            attendees: vec!["mailto:alice@example.com".to_string()],
            recurrence: None,
            status: EventStatus::Confirmed,
        };
        let ical = generate_ical_event(&event);
        assert!(ical.contains("ATTENDEE:mailto:alice@example.com"));
    }

    #[test]
    fn test_generate_ical_event_tentative_status() {
        let event = CalendarEvent {
            uid: "tent-gen-1".to_string(),
            summary: "Tentative".to_string(),
            description: None,
            start: Utc::now(),
            end: None,
            location: None,
            attendees: vec![],
            recurrence: None,
            status: EventStatus::Tentative,
        };
        let ical = generate_ical_event(&event);
        assert!(ical.contains("STATUS:TENTATIVE"));
    }

    #[test]
    fn test_generate_ical_event_cancelled_status() {
        let event = CalendarEvent {
            uid: "cancel-gen-1".to_string(),
            summary: "Cancelled".to_string(),
            description: None,
            start: Utc::now(),
            end: None,
            location: None,
            attendees: vec![],
            recurrence: None,
            status: EventStatus::Cancelled,
        };
        let ical = generate_ical_event(&event);
        assert!(ical.contains("STATUS:CANCELLED"));
    }

    #[test]
    fn test_generate_ical_event_with_end() {
        let start = Utc::now();
        let end = start + chrono::Duration::hours(1);
        let event = CalendarEvent {
            uid: "end-1".to_string(),
            summary: "With End".to_string(),
            description: None,
            start,
            end: Some(end),
            location: None,
            attendees: vec![],
            recurrence: None,
            status: EventStatus::Confirmed,
        };
        let ical = generate_ical_event(&event);
        assert!(ical.contains("DTEND:"));
    }

    #[test]
    fn test_calendar_event_debug() {
        let event = CalendarEvent {
            uid: "debug-1".to_string(),
            summary: "Debug".to_string(),
            description: None,
            start: Utc::now(),
            end: None,
            location: None,
            attendees: vec![],
            recurrence: None,
            status: EventStatus::Confirmed,
        };
        assert!(!format!("{:?}", event).is_empty());
    }

    #[test]
    fn test_calendar_event_clone() {
        let event = CalendarEvent {
            uid: "clone-1".to_string(),
            summary: "Clone".to_string(),
            description: Some("desc".to_string()),
            start: Utc::now(),
            end: None,
            location: Some("loc".to_string()),
            attendees: vec!["a@b.com".to_string()],
            recurrence: None,
            status: EventStatus::Confirmed,
        };
        let cloned = event.clone();
        assert_eq!(cloned.uid, "clone-1");
        assert_eq!(cloned.summary, "Clone");
        assert_eq!(cloned.description.as_deref(), Some("desc"));
    }
}
