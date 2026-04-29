use crate::store::*;

fn sample_event_ical(uid: &str, summary: &str, dtstart: &str, dtend: &str) -> String {
    format!(
        "BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
BEGIN:VEVENT\r\n\
UID:{}\r\n\
SUMMARY:{}\r\n\
DTSTART:{}\r\n\
DTEND:{}\r\n\
END:VEVENT\r\n\
END:VCALENDAR\r\n",
        uid, summary, dtstart, dtend
    )
}

#[tokio::test]
async fn test_create_and_list_calendars() {
    let store = InMemoryCalendarStore::new();
    let cal = store
        .create_calendar("user1", "Personal", "#ff0000")
        .await
        .unwrap();
    assert_eq!(cal.name, "Personal");
    assert_eq!(cal.principal, "user1");

    let cals = store.list_calendars("user1").await;
    assert_eq!(cals.len(), 1);
    assert_eq!(cals[0].id, cal.id);
}

#[tokio::test]
async fn test_delete_calendar() {
    let store = InMemoryCalendarStore::new();
    let cal = store
        .create_calendar("user1", "To Delete", "#000000")
        .await
        .unwrap();

    store.delete_calendar("user1", &cal.id).await.unwrap();
    let cals = store.list_calendars("user1").await;
    assert!(cals.is_empty());
}

#[tokio::test]
async fn test_get_calendar() {
    let store = InMemoryCalendarStore::new();
    let cal = store
        .create_calendar("user1", "My Cal", "#00ff00")
        .await
        .unwrap();

    let fetched = store.get_calendar("user1", &cal.id).await;
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().name, "My Cal");

    let not_found = store.get_calendar("user1", "nonexistent").await;
    assert!(not_found.is_none());
}

#[tokio::test]
async fn test_create_and_list_events() {
    let store = InMemoryCalendarStore::new();
    let cal = store
        .create_calendar("user1", "Events Cal", "#0000ff")
        .await
        .unwrap();

    let ical = sample_event_ical("evt-1", "Meeting", "20260427T140000Z", "20260427T150000Z");
    store.create_event(&cal.id, &ical).await.unwrap();

    let events = store.list_events(&cal.id).await;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].uid, "evt-1");
}

#[tokio::test]
async fn test_get_event() {
    let store = InMemoryCalendarStore::new();
    let cal = store
        .create_calendar("user1", "Cal", "#ffffff")
        .await
        .unwrap();

    let ical = sample_event_ical("evt-2", "Get Test", "20260427T100000Z", "20260427T110000Z");
    store.create_event(&cal.id, &ical).await.unwrap();

    let event = store.get_event(&cal.id, "evt-2").await;
    assert!(event.is_some());
    assert_eq!(event.unwrap().uid, "evt-2");

    let not_found = store.get_event(&cal.id, "nonexistent").await;
    assert!(not_found.is_none());
}

#[tokio::test]
async fn test_update_event() {
    let store = InMemoryCalendarStore::new();
    let cal = store
        .create_calendar("user1", "Cal", "#ffffff")
        .await
        .unwrap();

    let ical = sample_event_ical("evt-3", "Original", "20260427T100000Z", "20260427T110000Z");
    store.create_event(&cal.id, &ical).await.unwrap();

    let updated_ical = sample_event_ical(
        "evt-3",
        "Updated Title",
        "20260427T100000Z",
        "20260427T120000Z",
    );
    let event = store
        .update_event(&cal.id, "evt-3", &updated_ical)
        .await
        .unwrap();
    assert!(event.ical_data.contains("Updated Title"));
}

#[tokio::test]
async fn test_delete_event() {
    let store = InMemoryCalendarStore::new();
    let cal = store
        .create_calendar("user1", "Cal", "#ffffff")
        .await
        .unwrap();

    let ical = sample_event_ical("evt-4", "Delete Me", "20260427T100000Z", "20260427T110000Z");
    store.create_event(&cal.id, &ical).await.unwrap();

    store.delete_event(&cal.id, "evt-4").await.unwrap();
    let events = store.list_events(&cal.id).await;
    assert!(events.is_empty());
}

#[tokio::test]
async fn test_query_events_time_range() {
    let store = InMemoryCalendarStore::new();
    let cal = store
        .create_calendar("user1", "Cal", "#ffffff")
        .await
        .unwrap();

    let ical1 = sample_event_ical(
        "evt-a",
        "April Event",
        "20260401T100000Z",
        "20260402T110000Z",
    );
    let ical2 = sample_event_ical("evt-b", "May Event", "20260501T100000Z", "20260502T110000Z");
    store.create_event(&cal.id, &ical1).await.unwrap();
    store.create_event(&cal.id, &ical2).await.unwrap();

    let filter = CalFilter {
        start: Some(
            chrono::NaiveDate::from_ymd_opt(2026, 4, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc(),
        ),
        end: Some(
            chrono::NaiveDate::from_ymd_opt(2026, 4, 30)
                .unwrap()
                .and_hms_opt(23, 59, 59)
                .unwrap()
                .and_utc(),
        ),
    };

    let results = store.query_events(&cal.id, &filter).await;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].uid, "evt-a");
}

#[tokio::test]
async fn test_query_events_no_filter() {
    let store = InMemoryCalendarStore::new();
    let cal = store
        .create_calendar("user1", "Cal", "#ffffff")
        .await
        .unwrap();

    let ical1 = sample_event_ical("evt-x", "Event X", "20260101T100000Z", "20260102T110000Z");
    let ical2 = sample_event_ical("evt-y", "Event Y", "20261201T100000Z", "20261202T110000Z");
    store.create_event(&cal.id, &ical1).await.unwrap();
    store.create_event(&cal.id, &ical2).await.unwrap();

    let filter = CalFilter {
        start: None,
        end: None,
    };

    let results = store.query_events(&cal.id, &filter).await;
    assert_eq!(results.len(), 2);
}

#[tokio::test]
async fn test_calendar_isolation() {
    let store = InMemoryCalendarStore::new();
    let cal1 = store
        .create_calendar("user1", "Cal 1", "#ff0000")
        .await
        .unwrap();
    let cal2 = store
        .create_calendar("user1", "Cal 2", "#00ff00")
        .await
        .unwrap();

    let ical = sample_event_ical("iso-1", "Isolated", "20260427T100000Z", "20260427T110000Z");
    store.create_event(&cal1.id, &ical).await.unwrap();

    let events1 = store.list_events(&cal1.id).await;
    let events2 = store.list_events(&cal2.id).await;
    assert_eq!(events1.len(), 1);
    assert_eq!(events2.len(), 0);
}
