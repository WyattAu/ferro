mod address_book;
mod calendar;
#[cfg(test)]
mod tests;

pub use address_book::InMemoryAddressBookStore;
pub use calendar::InMemoryCalendarStore;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use std::sync::Arc;

#[cfg(feature = "persistence")]
use tracing::warn;

/// Information about a calendar collection.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CalendarInfo {
    /// Unique identifier for this calendar.
    pub id: String,
    /// Owner principal of this calendar.
    pub principal: String,
    /// Display name shown to users.
    pub name: String,
    /// Hex color code for calendar clients.
    pub color: String,
    /// Synchronization token for change detection.
    pub ctag: String,
    /// When this calendar was created.
    pub created_at: DateTime<Utc>,
    /// When this calendar was last modified.
    pub updated_at: DateTime<Utc>,
}

/// Information about a calendar event.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EventInfo {
    /// Unique identifier for this event (UID from iCalendar data).
    pub uid: String,
    /// ID of the calendar this event belongs to.
    pub calendar_id: String,
    /// Raw iCalendar (RFC 5545) data.
    pub ical_data: String,
    /// Entity tag for conditional requests.
    pub etag: String,
    /// When this event was created.
    pub created_at: DateTime<Utc>,
    /// When this event was last modified.
    pub updated_at: DateTime<Utc>,
}

/// Time-range filter for calendar event queries.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CalFilter {
    /// Include events starting at or after this time.
    pub start: Option<DateTime<Utc>>,
    /// Include events ending at or before this time.
    pub end: Option<DateTime<Utc>>,
}

/// Information about an address book collection.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AddressBookInfo {
    /// Unique identifier for this address book.
    pub id: String,
    /// Owner principal of this address book.
    pub principal: String,
    /// Display name shown to users.
    pub name: String,
    /// Synchronization token for change detection.
    pub ctag: String,
    /// When this address book was created.
    pub created_at: DateTime<Utc>,
    /// When this address book was last modified.
    pub updated_at: DateTime<Utc>,
}

/// Information about a contact (vCard).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContactInfo {
    /// Unique identifier for this contact (UID from vCard data).
    pub uid: String,
    /// ID of the address book this contact belongs to.
    pub address_book_id: String,
    /// Raw vCard (RFC 6350) data.
    pub vcard_data: String,
    /// Entity tag for conditional requests.
    pub etag: String,
    /// When this contact was created.
    pub created_at: DateTime<Utc>,
    /// When this contact was last modified.
    pub updated_at: DateTime<Utc>,
}

/// Error type for store operations.
#[derive(Debug, Clone)]
pub struct StoreError(String);

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for StoreError {}

/// Result type for store operations.
pub type StoreResult<T> = Result<T, StoreError>;

/// Trait for calendar data storage backends.
#[async_trait]
pub trait CalendarStore: Send + Sync {
    async fn list_calendars(&self, principal: &str) -> Vec<CalendarInfo>;
    async fn get_calendar(&self, principal: &str, calendar_id: &str) -> Option<CalendarInfo>;
    async fn create_calendar(&self, principal: &str, name: &str, color: &str) -> StoreResult<CalendarInfo>;
    async fn delete_calendar(&self, principal: &str, calendar_id: &str) -> StoreResult<()>;
    async fn list_events(&self, calendar_id: &str) -> Vec<EventInfo>;
    async fn get_event(&self, calendar_id: &str, event_uid: &str) -> Option<EventInfo>;
    async fn create_event(&self, calendar_id: &str, ical: &str) -> StoreResult<EventInfo>;
    async fn update_event(&self, calendar_id: &str, event_uid: &str, ical: &str) -> StoreResult<EventInfo>;
    async fn delete_event(&self, calendar_id: &str, event_uid: &str) -> StoreResult<()>;
    async fn query_events(&self, calendar_id: &str, filter: &CalFilter) -> Vec<EventInfo>;
}

/// Trait for address book data storage backends.
#[async_trait]
pub trait AddressBookStore: Send + Sync {
    async fn list_address_books(&self, principal: &str) -> Vec<AddressBookInfo>;
    async fn get_address_book(&self, principal: &str, book_id: &str) -> Option<AddressBookInfo>;
    async fn create_address_book(&self, principal: &str, name: &str) -> StoreResult<AddressBookInfo>;
    async fn delete_address_book(&self, principal: &str, book_id: &str) -> StoreResult<()>;
    async fn list_contacts(&self, book_id: &str) -> Vec<ContactInfo>;
    async fn get_contact(&self, book_id: &str, contact_uid: &str) -> Option<ContactInfo>;
    async fn create_contact(&self, book_id: &str, vcard: &str) -> StoreResult<ContactInfo>;
    async fn update_contact(&self, book_id: &str, contact_uid: &str, vcard: &str) -> StoreResult<ContactInfo>;
    async fn delete_contact(&self, book_id: &str, contact_uid: &str) -> StoreResult<()>;
}

/// # Safety
/// The wrapped `rusqlite::Connection` is only accessed via short-lived lock guards
/// that never cross an `.await` point. SQLite operations are synchronous
/// and complete in microseconds, well below the threshold for async poisoning.
#[cfg(feature = "persistence")]
pub type DbHandle = Arc<std::sync::Mutex<rusqlite::Connection>>;

/// Type-erased calendar store reference for use in async contexts.
pub type DynCalendarStore = Arc<dyn CalendarStore>;
/// Type-erased address book store reference for use in async contexts.
pub type DynAddressBookStore = Arc<dyn AddressBookStore>;
