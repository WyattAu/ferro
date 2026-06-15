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
        self.store
            .get_calendar(principal, calendar_id)
            .await
            .map(|info| {
                let mut cal = Calendar::from(info.clone());
                cal.uid = info.id;
                cal
            })
    }

    pub async fn create_calendar(
        &self,
        principal: &str,
        name: &str,
        color: &str,
    ) -> Result<Calendar> {
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

    pub async fn update_event(
        &self,
        calendar_id: &str,
        event_uid: &str,
        ical_data: &str,
    ) -> Result<CalendarItem> {
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

        let cal = manager
            .create_calendar("user1", "Work", "#0000ff")
            .await
            .unwrap();

        let ical = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:event-1\r\nSUMMARY:Meeting\r\nDTSTART:20240101T100000Z\r\nDTEND:20240101T110000Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";

        let item = manager.create_event(&cal.uid, ical).await.unwrap();
        assert_eq!(item.uid, "event-1");

        let retrieved = manager.get_event(&cal.uid, "event-1").await;
        assert!(retrieved.is_some());
    }
}
