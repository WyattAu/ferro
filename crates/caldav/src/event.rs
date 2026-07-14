use crate::calendar::CalendarItem;
use crate::ical::CalendarEvent;

#[derive(Debug, Clone)]
pub struct EventWithParsed {
    pub item: CalendarItem,
    pub parsed: Option<CalendarEvent>,
}

impl EventWithParsed {
    pub fn from_item(item: CalendarItem) -> Self {
        let ical_str = String::from_utf8_lossy(&item.data);
        let parsed = crate::ical::extract_event_from_ical(&ical_str).ok();
        Self { item, parsed }
    }
}

pub fn events_overlap(a: &CalendarEvent, b: &CalendarEvent) -> bool {
    match (&a.end, &b.end) {
        (Some(a_end), Some(b_end)) => a.start < *b_end && b.start < *a_end,
        (Some(a_end), None) => b.start < *a_end,
        (None, Some(b_end)) => a.start < *b_end,
        (None, None) => a.start == b.start,
    }
}

pub fn event_in_time_range(
    event: &CalendarEvent,
    range_start: Option<&chrono::DateTime<chrono::Utc>>,
    range_end: Option<&chrono::DateTime<chrono::Utc>>,
) -> bool {
    match (range_start, range_end, &event.end) {
        (Some(start), Some(end), Some(event_end)) => event.start < *end && *event_end > *start,
        (Some(start), Some(end), None) => event.start >= *start && event.start < *end,
        (Some(start), None, _) => event.start >= *start,
        (None, Some(end), Some(event_end)) => *event_end <= *end,
        (None, Some(end), None) => event.start < *end,
        (None, None, _) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ical::EventStatus;
    use chrono::{Duration, Utc};

    fn make_event(start_offset_hours: i64, duration_hours: i64) -> CalendarEvent {
        let start = Utc::now() + Duration::hours(start_offset_hours);
        let end = start + Duration::hours(duration_hours);
        CalendarEvent {
            uid: "test".to_string(),
            summary: "Test".to_string(),
            description: None,
            start,
            end: Some(end),
            location: None,
            attendees: vec![],
            recurrence: None,
            status: EventStatus::Confirmed,
        }
    }

    #[test]
    fn test_events_overlap() {
        let a = make_event(0, 2);
        let b = make_event(1, 2);
        let c = make_event(5, 1);

        assert!(events_overlap(&a, &b));
        assert!(!events_overlap(&a, &c));
    }

    #[test]
    fn test_event_in_time_range() {
        let event = make_event(1, 2);
        let range_start = Utc::now();
        let range_end = Utc::now() + Duration::hours(10);

        assert!(event_in_time_range(&event, Some(&range_start), Some(&range_end)));

        let range_start2 = Utc::now() + Duration::hours(100);
        assert!(!event_in_time_range(&event, Some(&range_start2), None));
    }

    #[test]
    fn test_events_overlap_no_end_a() {
        let a = CalendarEvent {
            uid: "no-end-a".to_string(),
            summary: "No End A".to_string(),
            description: None,
            start: Utc::now(),
            end: None,
            location: None,
            attendees: vec![],
            recurrence: None,
            status: EventStatus::Confirmed,
        };
        let b = make_event(0, 2);
        assert!(events_overlap(&a, &b));
    }

    #[test]
    fn test_events_overlap_no_end_b() {
        let a = make_event(0, 2);
        let b = CalendarEvent {
            uid: "no-end-b".to_string(),
            summary: "No End B".to_string(),
            description: None,
            start: Utc::now(),
            end: None,
            location: None,
            attendees: vec![],
            recurrence: None,
            status: EventStatus::Confirmed,
        };
        assert!(events_overlap(&a, &b));
    }

    #[test]
    fn test_events_overlap_both_no_end() {
        let base = Utc::now();
        let a = CalendarEvent {
            uid: "both-no-end-a".to_string(),
            summary: "Both No End A".to_string(),
            description: None,
            start: base,
            end: None,
            location: None,
            attendees: vec![],
            recurrence: None,
            status: EventStatus::Confirmed,
        };
        let b = CalendarEvent {
            uid: "both-no-end-b".to_string(),
            summary: "Both No End B".to_string(),
            description: None,
            start: base,
            end: None,
            location: None,
            attendees: vec![],
            recurrence: None,
            status: EventStatus::Confirmed,
        };
        assert!(events_overlap(&a, &b));

        let c = CalendarEvent {
            uid: "both-no-end-c".to_string(),
            summary: "Both No End C".to_string(),
            description: None,
            start: base + Duration::hours(1),
            end: None,
            location: None,
            attendees: vec![],
            recurrence: None,
            status: EventStatus::Confirmed,
        };
        assert!(!events_overlap(&a, &c));
    }

    #[test]
    fn test_event_in_time_range_no_range() {
        let event = make_event(1, 2);
        assert!(event_in_time_range(&event, None, None));
    }

    #[test]
    fn test_event_in_time_range_start_only() {
        let event = make_event(1, 2);
        let range_start = Utc::now();
        assert!(event_in_time_range(&event, Some(&range_start), None));

        let range_start2 = Utc::now() + Duration::hours(100);
        assert!(!event_in_time_range(&event, Some(&range_start2), None));
    }

    #[test]
    fn test_event_in_time_range_end_only_no_event_end() {
        let event = CalendarEvent {
            uid: "end-only".to_string(),
            summary: "End Only".to_string(),
            description: None,
            start: Utc::now(),
            end: None,
            location: None,
            attendees: vec![],
            recurrence: None,
            status: EventStatus::Confirmed,
        };
        let range_end = Utc::now() + Duration::hours(10);
        assert!(event_in_time_range(&event, None, Some(&range_end)));

        let range_end2 = Utc::now() - Duration::hours(1);
        assert!(!event_in_time_range(&event, None, Some(&range_end2)));
    }

    #[test]
    fn test_event_in_time_range_end_only_with_event_end() {
        let event = make_event(-5, 2); // ended 3 hours ago
        let range_end = Utc::now();
        assert!(event_in_time_range(&event, None, Some(&range_end)));

        let range_end2 = Utc::now() - Duration::hours(10);
        assert!(!event_in_time_range(&event, None, Some(&range_end2)));
    }

    #[test]
    fn test_event_with_parsed_from_item() {
        let item = CalendarItem {
            uid: "parsed-test".to_string(),
            etag: "1".to_string(),
            data: b"BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:parsed-test\r\nSUMMARY:Parsed Event\r\nDTSTART:20240101T100000Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n".to_vec(),
            last_modified: Utc::now(),
        };
        let parsed = EventWithParsed::from_item(item);
        assert!(parsed.parsed.is_some());
        assert_eq!(parsed.parsed.as_ref().unwrap().uid, "parsed-test");
    }

    #[test]
    fn test_event_with_parsed_invalid_data() {
        let item = CalendarItem {
            uid: "bad-data".to_string(),
            etag: "1".to_string(),
            data: b"not valid ical".to_vec(),
            last_modified: Utc::now(),
        };
        let parsed = EventWithParsed::from_item(item);
        assert!(parsed.parsed.is_none());
    }

    #[test]
    fn test_event_in_time_range_before_range() {
        let event = make_event(-10, 1); // entirely before the range
        let range_start = Utc::now();
        let range_end = Utc::now() + Duration::hours(5);
        assert!(!event_in_time_range(&event, Some(&range_start), Some(&range_end)));
    }

    #[test]
    fn test_event_in_time_range_after_range() {
        let event = make_event(20, 1); // entirely after the range
        let range_start = Utc::now();
        let range_end = Utc::now() + Duration::hours(5);
        assert!(!event_in_time_range(&event, Some(&range_start), Some(&range_end)));
    }
}
