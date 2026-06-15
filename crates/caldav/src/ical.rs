use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};

pub use ferro_dav::ical::{
    IcalComponent, IcalProperty, get_all_props, get_first_prop, parse_ical, serialize_ical,
};

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

pub fn parse_ical_datetime(
    value: &str,
    params: &std::collections::HashMap<String, String>,
) -> Option<DateTime<Utc>> {
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
            c.children
                .iter()
                .find(|ch| ch.name == "VEVENT" || ch.name == "VTODO")
        } else if c.name == "VEVENT" || c.name == "VTODO" {
            Some(c)
        } else {
            None
        }
    });

    let vevent =
        vevent.ok_or_else(|| CalDavError::InvalidData("No VEVENT or VTODO found".to_string()))?;

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

    let end =
        get_first_prop(vevent, "DTEND").and_then(|p| parse_ical_datetime(&p.value, &p.params));

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
    let mut ical =
        String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Ferro//CalDAV Server//EN\r\n");

    ical.push_str("BEGIN:VEVENT\r\n");
    ical.push_str(&format!("UID:{}\r\n", event.uid));
    ical.push_str(&format!("SUMMARY:{}\r\n", event.summary));

    ical.push_str(&format!(
        "DTSTART:{}\r\n",
        event.start.format("%Y%m%dT%H%M%SZ")
    ));

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
        assert!(matches!(
            EventStatus::parse("TENTATIVE"),
            EventStatus::Tentative
        ));
        assert!(matches!(
            EventStatus::parse("CONFIRMED"),
            EventStatus::Confirmed
        ));
        assert!(matches!(
            EventStatus::parse("CANCELLED"),
            EventStatus::Cancelled
        ));
        assert!(matches!(
            EventStatus::parse("UNKNOWN"),
            EventStatus::Confirmed
        ));
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
}
