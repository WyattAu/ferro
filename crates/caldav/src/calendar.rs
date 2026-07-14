use chrono::{DateTime, Utc};
use ferro_dav::store::{CalFilter, CalendarInfo, DynCalendarStore, EventInfo};

use crate::error::{CalDavError, Result};
use crate::ical::{self, CalendarEvent};

#[derive(Debug, Clone)]
pub struct Calendar {
    pub uid: String,
    pub display_name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub ctag: String,
    pub items: Vec<CalendarItem>,
}

#[derive(Debug, Clone)]
pub struct CalendarItem {
    pub uid: String,
    pub etag: String,
    pub data: Vec<u8>,
    pub last_modified: DateTime<Utc>,
}

impl From<CalendarInfo> for Calendar {
    fn from(info: CalendarInfo) -> Self {
        Calendar {
            uid: info.id,
            display_name: info.name,
            description: None,
            color: Some(info.color),
            ctag: info.ctag,
            items: Vec::new(),
        }
    }
}

impl From<EventInfo> for CalendarItem {
    fn from(event: EventInfo) -> Self {
        CalendarItem {
            uid: event.uid,
            etag: event.etag,
            data: event.ical_data.into_bytes(),
            last_modified: event.updated_at,
        }
    }
}

#[derive(Clone)]
pub struct CalendarManager {
    store: DynCalendarStore,
}

impl CalendarManager {
    pub fn new(store: DynCalendarStore) -> Self {
        Self { store }
    }

    pub async fn list_calendars(&self, principal: &str) -> Vec<Calendar> {
        self.store
            .list_calendars(principal)
            .await
            .into_iter()
            .map(Calendar::from)
            .collect()
    }

    pub async fn get_calendar(&self, principal: &str, calendar_id: &str) -> Option<Calendar> {
        self.store.get_calendar(principal, calendar_id).await.map(|info| {
            let mut cal = Calendar::from(info.clone());
            cal.uid = info.id;
            cal
        })
    }

    pub async fn create_calendar(&self, principal: &str, name: &str, color: &str) -> Result<Calendar> {
        self.store
            .create_calendar(principal, name, color)
            .await
            .map(Calendar::from)
            .map_err(|e| CalDavError::Store(e.to_string()))
    }

    pub async fn delete_calendar(&self, principal: &str, calendar_id: &str) -> Result<()> {
        self.store
            .delete_calendar(principal, calendar_id)
            .await
            .map_err(|e| CalDavError::Store(e.to_string()))
    }

    pub async fn list_events(&self, calendar_id: &str) -> Vec<CalendarItem> {
        self.store
            .list_events(calendar_id)
            .await
            .into_iter()
            .map(CalendarItem::from)
            .collect()
    }

    pub async fn get_event(&self, calendar_id: &str, event_uid: &str) -> Option<CalendarItem> {
        self.store
            .get_event(calendar_id, event_uid)
            .await
            .map(CalendarItem::from)
    }

    pub async fn create_event(&self, calendar_id: &str, ical_data: &str) -> Result<CalendarItem> {
        self.store
            .create_event(calendar_id, ical_data)
            .await
            .map(CalendarItem::from)
            .map_err(|e| CalDavError::Store(e.to_string()))
    }

    pub async fn update_event(&self, calendar_id: &str, event_uid: &str, ical_data: &str) -> Result<CalendarItem> {
        self.store
            .update_event(calendar_id, event_uid, ical_data)
            .await
            .map(CalendarItem::from)
            .map_err(|e| CalDavError::Store(e.to_string()))
    }

    pub async fn delete_event(&self, calendar_id: &str, event_uid: &str) -> Result<()> {
        self.store
            .delete_event(calendar_id, event_uid)
            .await
            .map_err(|e| CalDavError::Store(e.to_string()))
    }

    pub async fn query_events(&self, calendar_id: &str, filter: &CalFilter) -> Vec<CalendarItem> {
        self.store
            .query_events(calendar_id, filter)
            .await
            .into_iter()
            .map(CalendarItem::from)
            .collect()
    }

    pub fn parse_event_data(&self, ical: &str) -> Result<CalendarEvent> {
        ical::extract_event_from_ical(ical)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferro_dav::store::InMemoryCalendarStore;

    #[tokio::test]
    async fn test_create_and_list_calendars() {
        let store = std::sync::Arc::new(InMemoryCalendarStore::new());
        let manager = CalendarManager::new(store);

        let cal = manager
            .create_calendar("user1", "My Calendar", "#ff0000")
            .await
            .unwrap();
        assert_eq!(cal.display_name, "My Calendar");

        let calendars = manager.list_calendars("user1").await;
        assert_eq!(calendars.len(), 1);
        assert_eq!(calendars[0].uid, cal.uid);
    }

    #[tokio::test]
    async fn test_create_and_get_event() {
        let store = std::sync::Arc::new(InMemoryCalendarStore::new());
        let manager = CalendarManager::new(store);

        let cal = manager.create_calendar("user1", "Work", "#0000ff").await.unwrap();

        let ical = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:event-1\r\nSUMMARY:Meeting\r\nDTSTART:20240101T100000Z\r\nDTEND:20240101T110000Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";

        let item = manager.create_event(&cal.uid, ical).await.unwrap();
        assert_eq!(item.uid, "event-1");

        let retrieved = manager.get_event(&cal.uid, "event-1").await;
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_list_calendars_empty() {
        let store = std::sync::Arc::new(InMemoryCalendarStore::new());
        let manager = CalendarManager::new(store);
        let calendars = manager.list_calendars("user1").await;
        assert!(calendars.is_empty());
    }

    #[tokio::test]
    async fn test_get_calendar_not_found() {
        let store = std::sync::Arc::new(InMemoryCalendarStore::new());
        let manager = CalendarManager::new(store);
        let result = manager.get_calendar("user1", "nonexistent").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_delete_calendar() {
        let store = std::sync::Arc::new(InMemoryCalendarStore::new());
        let manager = CalendarManager::new(store);

        let cal = manager.create_calendar("user1", "Temp", "#000000").await.unwrap();
        manager.delete_calendar("user1", &cal.uid).await.unwrap();

        let calendars = manager.list_calendars("user1").await;
        assert!(calendars.is_empty());
    }

    #[tokio::test]
    async fn test_list_events_empty() {
        let store = std::sync::Arc::new(InMemoryCalendarStore::new());
        let manager = CalendarManager::new(store);

        let cal = manager.create_calendar("user1", "Empty", "#000000").await.unwrap();
        let events = manager.list_events(&cal.uid).await;
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn test_get_event_not_found() {
        let store = std::sync::Arc::new(InMemoryCalendarStore::new());
        let manager = CalendarManager::new(store);

        let cal = manager.create_calendar("user1", "Test", "#000000").await.unwrap();
        let result = manager.get_event(&cal.uid, "nonexistent").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_event() {
        let store = std::sync::Arc::new(InMemoryCalendarStore::new());
        let manager = CalendarManager::new(store);

        let cal = manager.create_calendar("user1", "Test", "#000000").await.unwrap();
        let ical = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:upd-1\r\nSUMMARY:Original\r\nDTSTART:20240101T100000Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        manager.create_event(&cal.uid, ical).await.unwrap();

        let updated_ical = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:upd-1\r\nSUMMARY:Updated\r\nDTSTART:20240101T100000Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let updated = manager.update_event(&cal.uid, "upd-1", updated_ical).await.unwrap();
        assert_eq!(updated.uid, "upd-1");
    }

    #[tokio::test]
    async fn test_delete_event() {
        let store = std::sync::Arc::new(InMemoryCalendarStore::new());
        let manager = CalendarManager::new(store);

        let cal = manager.create_calendar("user1", "Test", "#000000").await.unwrap();
        let ical = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:del-1\r\nSUMMARY:Delete Me\r\nDTSTART:20240101T100000Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        manager.create_event(&cal.uid, ical).await.unwrap();

        manager.delete_event(&cal.uid, "del-1").await.unwrap();
        let result = manager.get_event(&cal.uid, "del-1").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_create_multiple_calendars() {
        let store = std::sync::Arc::new(InMemoryCalendarStore::new());
        let manager = CalendarManager::new(store);

        manager.create_calendar("user1", "Cal1", "#ff0000").await.unwrap();
        manager.create_calendar("user1", "Cal2", "#00ff00").await.unwrap();

        let calendars = manager.list_calendars("user1").await;
        assert_eq!(calendars.len(), 2);
    }

    #[test]
    fn test_calendar_debug() {
        let cal = Calendar {
            uid: "test".to_string(),
            display_name: "Test".to_string(),
            description: None,
            color: None,
            ctag: "1".to_string(),
            items: vec![],
        };
        assert!(!format!("{:?}", cal).is_empty());
    }

    #[test]
    fn test_calendar_clone() {
        let cal = Calendar {
            uid: "clone".to_string(),
            display_name: "Clone".to_string(),
            description: Some("desc".to_string()),
            color: Some("#ff0000".to_string()),
            ctag: "1".to_string(),
            items: vec![],
        };
        let cloned = cal.clone();
        assert_eq!(cloned.uid, "clone");
    }

    #[test]
    fn test_calendar_item_debug() {
        let item = CalendarItem {
            uid: "item".to_string(),
            etag: "1".to_string(),
            data: vec![],
            last_modified: Utc::now(),
        };
        assert!(!format!("{:?}", item).is_empty());
    }

    #[test]
    fn test_calendar_item_clone() {
        let item = CalendarItem {
            uid: "item-cl".to_string(),
            etag: "2".to_string(),
            data: b"test".to_vec(),
            last_modified: Utc::now(),
        };
        let cloned = item.clone();
        assert_eq!(cloned.uid, "item-cl");
    }

    #[test]
    fn test_parse_event_data() {
        let store = std::sync::Arc::new(InMemoryCalendarStore::new());
        let manager = CalendarManager::new(store);

        let ical = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:parse-1\r\nSUMMARY:Parsed\r\nDTSTART:20240101T100000Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let event = manager.parse_event_data(ical).unwrap();
        assert_eq!(event.uid, "parse-1");
    }

    #[test]
    fn test_parse_event_data_invalid() {
        let store = std::sync::Arc::new(InMemoryCalendarStore::new());
        let manager = CalendarManager::new(store);
        let result = manager.parse_event_data("not valid");
        assert!(result.is_err());
    }
}
