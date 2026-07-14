use super::*;

#[tokio::test]
async fn test_calendar_create_and_list() {
    let store = InMemoryCalendarStore::new();
    let info = store.create_calendar("user1", "Personal", "#ff0000").await.unwrap();
    assert_eq!(info.name, "Personal");
    assert_eq!(info.color, "#ff0000");

    let calendars = store.list_calendars("user1").await;
    assert_eq!(calendars.len(), 1);
}

#[tokio::test]
async fn test_address_book_create_and_list() {
    let store = InMemoryAddressBookStore::new();
    let info = store.create_address_book("user1", "Contacts").await.unwrap();
    assert_eq!(info.name, "Contacts");

    let books = store.list_address_books("user1").await;
    assert_eq!(books.len(), 1);
}

#[tokio::test]
async fn test_calendar_get() {
    let store = InMemoryCalendarStore::new();
    let info = store.create_calendar("user1", "Work", "#0000ff").await.unwrap();

    let retrieved = store.get_calendar("user1", &info.id).await;
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, "Work");
}

#[tokio::test]
async fn test_calendar_get_wrong_principal() {
    let store = InMemoryCalendarStore::new();
    let info = store.create_calendar("user1", "Work", "#0000ff").await.unwrap();

    let retrieved = store.get_calendar("user2", &info.id).await;
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_calendar_delete() {
    let store = InMemoryCalendarStore::new();
    let info = store.create_calendar("user1", "Temp", "#ff0000").await.unwrap();
    store.delete_calendar("user1", &info.id).await.unwrap();

    let calendars = store.list_calendars("user1").await;
    assert!(calendars.is_empty());
}

#[tokio::test]
async fn test_calendar_delete_not_found() {
    let store = InMemoryCalendarStore::new();
    let result = store.delete_calendar("user1", "nonexistent").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_calendar_create_and_get_event() {
    let store = InMemoryCalendarStore::new();
    let cal = store.create_calendar("user1", "Work", "#0000ff").await.unwrap();

    let ical = "BEGIN:VCALENDAR\r\n\
BEGIN:VEVENT\r\n\
UID:event-1\r\n\
SUMMARY:Meeting\r\n\
DTSTART:20260427T100000Z\r\n\
DTEND:20260427T110000Z\r\n\
END:VEVENT\r\n\
END:VCALENDAR\r\n";

    let event = store.create_event(&cal.id, ical).await.unwrap();
    assert_eq!(event.uid, "event-1");

    let retrieved = store.get_event(&cal.id, "event-1").await;
    assert!(retrieved.is_some());
}

#[tokio::test]
async fn test_calendar_list_events() {
    let store = InMemoryCalendarStore::new();
    let cal = store.create_calendar("user1", "Work", "#0000ff").await.unwrap();

    let ical1 = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:evt-1\r\nSUMMARY:Event 1\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    let ical2 = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:evt-2\r\nSUMMARY:Event 2\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";

    store.create_event(&cal.id, ical1).await.unwrap();
    store.create_event(&cal.id, ical2).await.unwrap();

    let events = store.list_events(&cal.id).await;
    assert_eq!(events.len(), 2);
}

#[tokio::test]
async fn test_calendar_update_event() {
    let store = InMemoryCalendarStore::new();
    let cal = store.create_calendar("user1", "Work", "#0000ff").await.unwrap();

    let ical = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:evt-1\r\nSUMMARY:Original\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    store.create_event(&cal.id, ical).await.unwrap();

    let updated_ical =
        "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:evt-1\r\nSUMMARY:Updated\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    let updated = store.update_event(&cal.id, "evt-1", updated_ical).await.unwrap();
    assert_eq!(updated.ical_data, updated_ical);
}

#[tokio::test]
async fn test_calendar_update_event_not_found() {
    let store = InMemoryCalendarStore::new();
    let cal = store.create_calendar("user1", "Work", "#0000ff").await.unwrap();

    let result = store.update_event(&cal.id, "nonexistent", "data").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_calendar_delete_event() {
    let store = InMemoryCalendarStore::new();
    let cal = store.create_calendar("user1", "Work", "#0000ff").await.unwrap();

    let ical = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:evt-1\r\nSUMMARY:Event\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    store.create_event(&cal.id, ical).await.unwrap();

    store.delete_event(&cal.id, "evt-1").await.unwrap();
    let events = store.list_events(&cal.id).await;
    assert!(events.is_empty());
}

#[tokio::test]
async fn test_calendar_delete_event_not_found() {
    let store = InMemoryCalendarStore::new();
    let cal = store.create_calendar("user1", "Work", "#0000ff").await.unwrap();

    let result = store.delete_event(&cal.id, "nonexistent").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_calendar_query_events() {
    let store = InMemoryCalendarStore::new();
    let cal = store.create_calendar("user1", "Work", "#0000ff").await.unwrap();

    let ical = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:evt-1\r\nSUMMARY:Event\r\nDTSTART:20260427T100000Z\r\nDTEND:20260427T110000Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    store.create_event(&cal.id, ical).await.unwrap();

    let filter = CalFilter { start: None, end: None };
    let events = store.query_events(&cal.id, &filter).await;
    assert_eq!(events.len(), 1);
}

#[tokio::test]
async fn test_address_book_get() {
    let store = InMemoryAddressBookStore::new();
    let info = store.create_address_book("user1", "Personal").await.unwrap();

    let retrieved = store.get_address_book("user1", &info.id).await;
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, "Personal");
}

#[tokio::test]
async fn test_address_book_delete() {
    let store = InMemoryAddressBookStore::new();
    let info = store.create_address_book("user1", "Temp").await.unwrap();
    store.delete_address_book("user1", &info.id).await.unwrap();

    let books = store.list_address_books("user1").await;
    assert!(books.is_empty());
}

#[tokio::test]
async fn test_address_book_delete_not_found() {
    let store = InMemoryAddressBookStore::new();
    let result = store.delete_address_book("user1", "nonexistent").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_address_book_create_and_get_contact() {
    let store = InMemoryAddressBookStore::new();
    let book = store.create_address_book("user1", "Contacts").await.unwrap();

    let vcard = "BEGIN:VCARD\r\nVERSION:3.0\r\nUID:contact-1\r\nFN:John Doe\r\nEND:VCARD\r\n";
    let contact = store.create_contact(&book.id, vcard).await.unwrap();
    assert_eq!(contact.uid, "contact-1");

    let retrieved = store.get_contact(&book.id, "contact-1").await;
    assert!(retrieved.is_some());
}

#[tokio::test]
async fn test_address_book_list_contacts() {
    let store = InMemoryAddressBookStore::new();
    let book = store.create_address_book("user1", "Contacts").await.unwrap();

    let vcard1 = "BEGIN:VCARD\r\nVERSION:3.0\r\nUID:c-1\r\nFN:Contact 1\r\nEND:VCARD\r\n";
    let vcard2 = "BEGIN:VCARD\r\nVERSION:3.0\r\nUID:c-2\r\nFN:Contact 2\r\nEND:VCARD\r\n";

    store.create_contact(&book.id, vcard1).await.unwrap();
    store.create_contact(&book.id, vcard2).await.unwrap();

    let contacts = store.list_contacts(&book.id).await;
    assert_eq!(contacts.len(), 2);
}

#[tokio::test]
async fn test_address_book_update_contact() {
    let store = InMemoryAddressBookStore::new();
    let book = store.create_address_book("user1", "Contacts").await.unwrap();

    let vcard = "BEGIN:VCARD\r\nVERSION:3.0\r\nUID:c-1\r\nFN:Original\r\nEND:VCARD\r\n";
    store.create_contact(&book.id, vcard).await.unwrap();

    let updated_vcard = "BEGIN:VCARD\r\nVERSION:3.0\r\nUID:c-1\r\nFN:Updated\r\nEND:VCARD\r\n";
    let updated = store.update_contact(&book.id, "c-1", updated_vcard).await.unwrap();
    assert_eq!(updated.vcard_data, updated_vcard);
}

#[tokio::test]
async fn test_address_book_update_contact_not_found() {
    let store = InMemoryAddressBookStore::new();
    let book = store.create_address_book("user1", "Contacts").await.unwrap();

    let result = store.update_contact(&book.id, "nonexistent", "data").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_address_book_delete_contact() {
    let store = InMemoryAddressBookStore::new();
    let book = store.create_address_book("user1", "Contacts").await.unwrap();

    let vcard = "BEGIN:VCARD\r\nVERSION:3.0\r\nUID:c-1\r\nFN:Contact\r\nEND:VCARD\r\n";
    store.create_contact(&book.id, vcard).await.unwrap();

    store.delete_contact(&book.id, "c-1").await.unwrap();
    let contacts = store.list_contacts(&book.id).await;
    assert!(contacts.is_empty());
}

#[tokio::test]
async fn test_address_book_delete_contact_not_found() {
    let store = InMemoryAddressBookStore::new();
    let book = store.create_address_book("user1", "Contacts").await.unwrap();

    let result = store.delete_contact(&book.id, "nonexistent").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_calendar_list_wrong_principal() {
    let store = InMemoryCalendarStore::new();
    store.create_calendar("user1", "Work", "#0000ff").await.unwrap();

    let calendars = store.list_calendars("user2").await;
    assert!(calendars.is_empty());
}

#[tokio::test]
async fn test_address_book_list_wrong_principal() {
    let store = InMemoryAddressBookStore::new();
    store.create_address_book("user1", "Contacts").await.unwrap();

    let books = store.list_address_books("user2").await;
    assert!(books.is_empty());
}

#[tokio::test]
async fn test_calendar_create_event_not_found() {
    let store = InMemoryCalendarStore::new();
    let result = store
        .create_event(
            "nonexistent",
            "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:test\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n",
        )
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_address_book_create_contact_not_found() {
    let store = InMemoryAddressBookStore::new();
    let result = store
        .create_contact("nonexistent", "BEGIN:VCARD\r\nVERSION:3.0\r\nFN:Test\r\nEND:VCARD\r\n")
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_store_error_display() {
    let err = StoreError("test error".to_string());
    assert_eq!(format!("{}", err), "test error");
}

#[tokio::test]
async fn test_store_error_is_error() {
    let err = StoreError("test error".to_string());
    let _: &dyn std::error::Error = &err;
}

#[tokio::test]
async fn test_calendar_store_default() {
    let store = InMemoryCalendarStore::default();
    let calendars = store.list_calendars("user1").await;
    assert!(calendars.is_empty());
}

#[tokio::test]
async fn test_address_book_store_default() {
    let store = InMemoryAddressBookStore::default();
    let books = store.list_address_books("user1").await;
    assert!(books.is_empty());
}

#[cfg(feature = "persistence")]
#[tokio::test]
async fn test_calendar_persistence_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let db: DbHandle = Arc::new(std::sync::Mutex::new(conn));

    let store1 = InMemoryCalendarStore::with_db(db.clone());
    let info = store1.create_calendar("user1", "Work", "#0000ff").await.unwrap();
    let cal_id = info.id.clone();

    let ical = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:test-event-1\r\nSUMMARY:Meeting\r\nDTSTART:20240101T100000Z\r\nDTEND:20240101T110000Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    let event = store1.create_event(&cal_id, ical).await.unwrap();

    drop(store1);

    let store2 = InMemoryCalendarStore::with_db(db.clone());
    let calendars = store2.list_calendars("user1").await;
    assert_eq!(calendars.len(), 1);
    assert_eq!(calendars[0].name, "Work");
    assert_eq!(calendars[0].color, "#0000ff");

    let events = store2.list_events(&cal_id).await;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].uid, event.uid);
}

#[cfg(feature = "persistence")]
#[tokio::test]
async fn test_address_book_persistence_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let db: DbHandle = Arc::new(std::sync::Mutex::new(conn));

    let store1 = InMemoryAddressBookStore::with_db(db.clone());
    let info = store1.create_address_book("user1", "Personal").await.unwrap();
    let book_id = info.id.clone();

    let vcard = "BEGIN:VCARD\r\nVERSION:3.0\r\nFN:John Doe\r\nUID:contact-1\r\nEND:VCARD\r\n";
    let contact = store1.create_contact(&book_id, vcard).await.unwrap();

    drop(store1);

    let store2 = InMemoryAddressBookStore::with_db(db.clone());
    let books = store2.list_address_books("user1").await;
    assert_eq!(books.len(), 1);
    assert_eq!(books[0].name, "Personal");

    let contacts = store2.list_contacts(&book_id).await;
    assert_eq!(contacts.len(), 1);
    assert_eq!(contacts[0].uid, contact.uid);
}

#[cfg(feature = "persistence")]
#[tokio::test]
async fn test_calendar_delete_persistence() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let db: DbHandle = Arc::new(std::sync::Mutex::new(conn));

    let store1 = InMemoryCalendarStore::with_db(db.clone());
    let info = store1.create_calendar("user1", "Temp", "#ff0000").await.unwrap();
    store1.delete_calendar("user1", &info.id).await.unwrap();
    drop(store1);

    let store2 = InMemoryCalendarStore::with_db(db);
    let calendars = store2.list_calendars("user1").await;
    assert!(calendars.is_empty());
}

#[cfg(feature = "persistence")]
#[tokio::test]
async fn test_contact_delete_persistence() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let db: DbHandle = Arc::new(std::sync::Mutex::new(conn));

    let store1 = InMemoryAddressBookStore::with_db(db.clone());
    let info = store1.create_address_book("user1", "Book").await.unwrap();
    let vcard = "BEGIN:VCARD\r\nVERSION:3.0\r\nFN:Temp\r\nUID:temp-1\r\nEND:VCARD\r\n";
    let contact = store1.create_contact(&info.id, vcard).await.unwrap();
    store1.delete_contact(&info.id, &contact.uid).await.unwrap();
    drop(store1);

    let store2 = InMemoryAddressBookStore::with_db(db);
    let contacts = store2.list_contacts(&info.id).await;
    assert!(contacts.is_empty());
}
