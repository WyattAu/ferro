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
}
