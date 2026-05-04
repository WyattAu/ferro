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
    async fn create_calendar(
        &self,
        principal: &str,
        name: &str,
        color: &str,
    ) -> StoreResult<CalendarInfo>;
    async fn delete_calendar(&self, principal: &str, calendar_id: &str) -> StoreResult<()>;
    async fn list_events(&self, calendar_id: &str) -> Vec<EventInfo>;
    async fn get_event(&self, calendar_id: &str, event_uid: &str) -> Option<EventInfo>;
    async fn create_event(&self, calendar_id: &str, ical: &str) -> StoreResult<EventInfo>;
    async fn update_event(
        &self,
        calendar_id: &str,
        event_uid: &str,
        ical: &str,
    ) -> StoreResult<EventInfo>;
    async fn delete_event(&self, calendar_id: &str, event_uid: &str) -> StoreResult<()>;
    async fn query_events(&self, calendar_id: &str, filter: &CalFilter) -> Vec<EventInfo>;
}

/// Trait for address book data storage backends.
#[async_trait]
pub trait AddressBookStore: Send + Sync {
    async fn list_address_books(&self, principal: &str) -> Vec<AddressBookInfo>;
    async fn get_address_book(&self, principal: &str, book_id: &str) -> Option<AddressBookInfo>;
    async fn create_address_book(
        &self,
        principal: &str,
        name: &str,
    ) -> StoreResult<AddressBookInfo>;
    async fn delete_address_book(&self, principal: &str, book_id: &str) -> StoreResult<()>;
    async fn list_contacts(&self, book_id: &str) -> Vec<ContactInfo>;
    async fn get_contact(&self, book_id: &str, contact_uid: &str) -> Option<ContactInfo>;
    async fn create_contact(&self, book_id: &str, vcard: &str) -> StoreResult<ContactInfo>;
    async fn update_contact(
        &self,
        book_id: &str,
        contact_uid: &str,
        vcard: &str,
    ) -> StoreResult<ContactInfo>;
    async fn delete_contact(&self, book_id: &str, contact_uid: &str) -> StoreResult<()>;
}

#[derive(Debug, Clone)]
struct CalendarData {
    info: CalendarInfo,
    events: DashMap<String, EventInfo>,
}

#[derive(Debug, Clone)]
struct AddressBookData {
    info: AddressBookInfo,
    contacts: DashMap<String, ContactInfo>,
}

/// Thread-safe database handle for persistence.
#[cfg(feature = "persistence")]
pub type DbHandle = Arc<std::sync::Mutex<rusqlite::Connection>>;

/// In-memory calendar store with optional SQLite persistence.
#[derive(Debug, Clone)]
pub struct InMemoryCalendarStore {
    calendars: DashMap<String, CalendarData>,
    #[cfg(feature = "persistence")]
    db: Option<DbHandle>,
}

impl InMemoryCalendarStore {
    /// Create a new empty in-memory calendar store.
    pub fn new() -> Self {
        Self {
            calendars: DashMap::new(),
            #[cfg(feature = "persistence")]
            db: None,
        }
    }

    #[cfg(feature = "persistence")]
    /// Create an in-memory calendar store backed by a shared SQLite database.
    pub fn with_db(db: DbHandle) -> Self {
        {
            let conn = db.lock().unwrap_or_else(|e| e.into_inner());
            let _ = conn.execute_batch(
                "
                CREATE TABLE IF NOT EXISTS calendars (
                    principal TEXT NOT NULL,
                    calendar_id TEXT NOT NULL,
                    name TEXT NOT NULL DEFAULT '',
                    color TEXT NOT NULL DEFAULT '',
                    description TEXT NOT NULL DEFAULT '',
                    ctag TEXT NOT NULL DEFAULT '',
                    PRIMARY KEY (principal, calendar_id)
                );
                CREATE TABLE IF NOT EXISTS calendar_events (
                    calendar_id TEXT NOT NULL,
                    uid TEXT NOT NULL,
                    ical_data TEXT NOT NULL,
                    etag TEXT NOT NULL DEFAULT '',
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    PRIMARY KEY (calendar_id, uid)
                );
                ",
            );
        }

        let store = Self {
            calendars: DashMap::new(),
            db: Some(db.clone()),
        };

        store.load_all_from_db(&db);
        store
    }

    #[cfg(feature = "persistence")]
    fn load_all_from_db(&self, db: &DbHandle) {
        let conn = match db.lock() {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to lock DB for loading calendars: {}", e);
                return;
            }
        };

        if let Ok(mut stmt) =
            conn.prepare("SELECT principal, calendar_id, name, color, ctag FROM calendars")
        {
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            });
            if let Ok(rows) = rows {
                for row in rows.flatten() {
                    let (principal, calendar_id, name, color, ctag) = row;
                    let now = Utc::now();
                    let key = Self::calendar_key(&principal, &calendar_id);
                    self.calendars.insert(
                        key,
                        CalendarData {
                            info: CalendarInfo {
                                id: calendar_id,
                                principal,
                                name,
                                color,
                                ctag,
                                created_at: now,
                                updated_at: now,
                            },
                            events: DashMap::new(),
                        },
                    );
                }
            }
        }

        if let Ok(mut stmt) = conn.prepare(
            "SELECT calendar_id, uid, ical_data, etag, created_at, updated_at FROM calendar_events",
        ) {
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            });
            if let Ok(rows) = rows {
                for row in rows.flatten() {
                    let (calendar_id, uid, ical_data, etag, created_at_str, updated_at_str) = row;
                    let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now());
                    let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now());
                    for entry in self.calendars.iter() {
                        if entry.value().info.id == calendar_id {
                            entry.value().events.insert(
                                uid.clone(),
                                EventInfo {
                                    uid,
                                    calendar_id,
                                    ical_data,
                                    etag,
                                    created_at,
                                    updated_at,
                                },
                            );
                            break;
                        }
                    }
                }
            }
        }
    }

    fn calendar_key(principal: &str, calendar_id: &str) -> String {
        format!("{}:{}", principal, calendar_id)
    }

    fn next_ctag() -> String {
        uuid::Uuid::new_v4().to_string()[..8].to_string()
    }
}

impl Default for InMemoryCalendarStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CalendarStore for InMemoryCalendarStore {
    async fn list_calendars(&self, principal: &str) -> Vec<CalendarInfo> {
        let mut result = Vec::new();
        for entry in self.calendars.iter() {
            if entry.value().info.principal == principal {
                result.push(entry.value().info.clone());
            }
        }
        result
    }

    async fn get_calendar(&self, principal: &str, calendar_id: &str) -> Option<CalendarInfo> {
        let key = Self::calendar_key(principal, calendar_id);
        self.calendars.get(&key).map(|e| e.value().info.clone())
    }

    async fn create_calendar(
        &self,
        principal: &str,
        name: &str,
        color: &str,
    ) -> StoreResult<CalendarInfo> {
        let id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        let now = Utc::now();
        let info = CalendarInfo {
            id: id.clone(),
            principal: principal.to_string(),
            name: name.to_string(),
            color: color.to_string(),
            ctag: Self::next_ctag(),
            created_at: now,
            updated_at: now,
        };
        let key = Self::calendar_key(principal, &id);
        if self.calendars.contains_key(&key) {
            return Err(StoreError("Calendar already exists".to_string()));
        }
        self.calendars.insert(
            key,
            CalendarData {
                info: info.clone(),
                events: DashMap::new(),
            },
        );

        #[cfg(feature = "persistence")]
        if let Some(ref db) = self.db {
            if let Ok(conn) = db.lock() {
                if let Err(e) = conn.execute(
                    "INSERT OR REPLACE INTO calendars (principal, calendar_id, name, color, ctag) VALUES (?1, ?2, ?3, ?4, ?5)",
                    rusqlite::params![info.principal, info.id, info.name, info.color, info.ctag],
                ) {
                    warn!("Failed to persist calendar to SQLite: {}", e);
                }
            }
        }

        Ok(info)
    }

    async fn delete_calendar(&self, principal: &str, calendar_id: &str) -> StoreResult<()> {
        let key = Self::calendar_key(principal, calendar_id);
        if self.calendars.remove(&key).is_none() {
            return Err(StoreError("Calendar not found".to_string()));
        }

        #[cfg(feature = "persistence")]
        if let Some(ref db) = self.db {
            if let Ok(conn) = db.lock() {
                if let Err(e) = conn.execute(
                    "DELETE FROM calendar_events WHERE calendar_id = ?1",
                    rusqlite::params![calendar_id],
                ) {
                    warn!("Failed to delete calendar events from SQLite: {}", e);
                }
                if let Err(e) = conn.execute(
                    "DELETE FROM calendars WHERE principal = ?1 AND calendar_id = ?2",
                    rusqlite::params![principal, calendar_id],
                ) {
                    warn!("Failed to delete calendar from SQLite: {}", e);
                }
            }
        }

        Ok(())
    }

    async fn list_events(&self, calendar_id: &str) -> Vec<EventInfo> {
        let mut result = Vec::new();
        for entry in self.calendars.iter() {
            if entry.value().info.id == calendar_id {
                for event_entry in entry.value().events.iter() {
                    result.push(event_entry.value().clone());
                }
            }
        }
        result
    }

    async fn get_event(&self, calendar_id: &str, event_uid: &str) -> Option<EventInfo> {
        for entry in self.calendars.iter() {
            if entry.value().info.id == calendar_id
                && let Some(event) = entry.value().events.get(event_uid)
            {
                return Some(event.value().clone());
            }
        }
        None
    }

    async fn create_event(&self, calendar_id: &str, ical: &str) -> StoreResult<EventInfo> {
        let uid = crate::ical::parse_ical(ical)
            .ok()
            .and_then(|comps| {
                comps.iter().find_map(|c| {
                    if c.name == "VCALENDAR" {
                        c.children.iter().find_map(|child| {
                            if child.name == "VEVENT" || child.name == "VTODO" {
                                crate::ical::get_first_prop(child, "UID").map(|p| p.value.clone())
                            } else {
                                None
                            }
                        })
                    } else {
                        None
                    }
                })
            })
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let now = Utc::now();
        let etag = format!("\"{}\"", now.timestamp());
        let event = EventInfo {
            uid: uid.clone(),
            calendar_id: calendar_id.to_string(),
            ical_data: ical.to_string(),
            etag: etag.clone(),
            created_at: now,
            updated_at: now,
        };

        let cal_key = self
            .calendars
            .iter()
            .find(|e| e.value().info.id == calendar_id)
            .map(|e| e.key().clone());

        let Some(cal_key) = cal_key else {
            return Err(StoreError("Calendar not found".to_string()));
        };

        let Some(mut cal_entry) = self.calendars.get_mut(&cal_key) else {
            return Err(StoreError("Calendar not found".to_string()));
        };

        if cal_entry.events.contains_key(&uid) {
            return Err(StoreError("Event already exists".to_string()));
        }
        cal_entry.events.insert(uid.clone(), event.clone());
        let new_ctag = Self::next_ctag();
        #[cfg(feature = "persistence")]
        let principal = cal_entry.info.principal.clone();
        cal_entry.info.ctag = new_ctag.clone();

        #[cfg(feature = "persistence")]
        if let Some(ref db) = self.db {
            if let Ok(conn) = db.lock() {
                if let Err(e) = conn.execute(
                    "INSERT OR REPLACE INTO calendar_events (calendar_id, uid, ical_data, etag, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![
                        event.calendar_id,
                        event.uid,
                        event.ical_data,
                        event.etag,
                        event.created_at.to_rfc3339(),
                        event.updated_at.to_rfc3339(),
                    ],
                ) {
                    warn!("Failed to persist event to SQLite: {}", e);
                }
                if let Err(e) = conn.execute(
                    "UPDATE calendars SET ctag = ?1 WHERE principal = ?2 AND calendar_id = ?3",
                    rusqlite::params![new_ctag, principal, cal_entry.info.id],
                ) {
                    warn!("Failed to persist calendar ctag to SQLite: {}", e);
                }
            }
        }

        Ok(event)
    }

    async fn update_event(
        &self,
        calendar_id: &str,
        event_uid: &str,
        ical: &str,
    ) -> StoreResult<EventInfo> {
        let now = Utc::now();
        let etag = format!("\"{}\"", now.timestamp());

        let cal_key = self
            .calendars
            .iter()
            .find(|e| e.value().info.id == calendar_id)
            .map(|e| e.key().clone());

        let Some(cal_key) = cal_key else {
            return Err(StoreError("Calendar not found".to_string()));
        };

        let Some(mut cal_entry) = self.calendars.get_mut(&cal_key) else {
            return Err(StoreError("Calendar not found".to_string()));
        };

        let mut event = cal_entry
            .events
            .get(event_uid)
            .ok_or_else(|| StoreError("Event not found".to_string()))?
            .value()
            .clone();
        event.ical_data = ical.to_string();
        event.etag = etag.clone();
        event.updated_at = now;
        cal_entry
            .events
            .insert(event_uid.to_string(), event.clone());
        let new_ctag = Self::next_ctag();
        #[cfg(feature = "persistence")]
        let principal = cal_entry.info.principal.clone();
        cal_entry.info.ctag = new_ctag.clone();

        #[cfg(feature = "persistence")]
        if let Some(ref db) = self.db {
            if let Ok(conn) = db.lock() {
                if let Err(e) = conn.execute(
                    "INSERT OR REPLACE INTO calendar_events (calendar_id, uid, ical_data, etag, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![
                        event.calendar_id,
                        event.uid,
                        event.ical_data,
                        event.etag,
                        event.created_at.to_rfc3339(),
                        event.updated_at.to_rfc3339(),
                    ],
                ) {
                    warn!("Failed to persist event update to SQLite: {}", e);
                }
                if let Err(e) = conn.execute(
                    "UPDATE calendars SET ctag = ?1 WHERE principal = ?2 AND calendar_id = ?3",
                    rusqlite::params![new_ctag, principal, cal_entry.info.id],
                ) {
                    warn!("Failed to persist calendar ctag to SQLite: {}", e);
                }
            }
        }

        Ok(event)
    }

    async fn delete_event(&self, calendar_id: &str, event_uid: &str) -> StoreResult<()> {
        let cal_key = self
            .calendars
            .iter()
            .find(|e| e.value().info.id == calendar_id)
            .map(|e| e.key().clone());

        let Some(cal_key) = cal_key else {
            return Err(StoreError("Calendar not found".to_string()));
        };

        let Some(mut cal_entry) = self.calendars.get_mut(&cal_key) else {
            return Err(StoreError("Calendar not found".to_string()));
        };

        if cal_entry.events.remove(event_uid).is_none() {
            return Err(StoreError("Event not found".to_string()));
        }
        let new_ctag = Self::next_ctag();
        #[cfg(feature = "persistence")]
        let principal = cal_entry.info.principal.clone();
        cal_entry.info.ctag = new_ctag.clone();

        #[cfg(feature = "persistence")]
        if let Some(ref db) = self.db {
            if let Ok(conn) = db.lock() {
                if let Err(e) = conn.execute(
                    "DELETE FROM calendar_events WHERE calendar_id = ?1 AND uid = ?2",
                    rusqlite::params![calendar_id, event_uid],
                ) {
                    warn!("Failed to delete event from SQLite: {}", e);
                }
                if let Err(e) = conn.execute(
                    "UPDATE calendars SET ctag = ?1 WHERE principal = ?2 AND calendar_id = ?3",
                    rusqlite::params![new_ctag, principal, cal_entry.info.id],
                ) {
                    warn!("Failed to persist calendar ctag to SQLite: {}", e);
                }
            }
        }

        Ok(())
    }

    async fn query_events(&self, calendar_id: &str, filter: &CalFilter) -> Vec<EventInfo> {
        let all_events = self.list_events(calendar_id).await;
        if filter.start.is_none() && filter.end.is_none() {
            return all_events;
        }

        all_events
            .into_iter()
            .filter(|event| {
                let comps = match crate::ical::parse_ical(&event.ical_data) {
                    Ok(c) => c,
                    Err(_) => return true,
                };

                let vevent = comps.iter().find_map(|c| {
                    if c.name == "VCALENDAR" {
                        c.children
                            .iter()
                            .find(|ch| ch.name == "VEVENT" || ch.name == "VTODO")
                    } else {
                        None
                    }
                });

                let Some(vevent) = vevent else {
                    return true;
                };

                let dtstart = crate::ical::get_first_prop(vevent, "DTSTART")
                    .and_then(|p| parse_ical_datetime(&p.value, &p.params));
                let dtend = crate::ical::get_first_prop(vevent, "DTEND")
                    .and_then(|p| parse_ical_datetime(&p.value, &p.params));

                match (dtstart, dtend, filter.start, filter.end) {
                    (Some(s), Some(e), Some(fs), Some(fe)) => s < fe && e > fs,
                    (Some(s), _, Some(fs), Some(fe)) => s >= fs && s < fe,
                    (Some(s), _, Some(fs), None) => s >= fs,
                    (Some(s), _, None, Some(fe)) => s < fe,
                    (_, Some(e), Some(fs), Some(fe)) => e > fs && e <= fe,
                    (None, None, _, _) => true,
                    _ => true,
                }
            })
            .collect()
    }
}

fn parse_ical_datetime(
    value: &str,
    params: &std::collections::HashMap<String, String>,
) -> Option<DateTime<Utc>> {
    let is_date = params.get("VALUE").map(|v| v.as_str()) == Some("DATE");

    let cleaned = value.trim();
    if cleaned.is_empty() {
        return None;
    }

    if is_date {
        let parsed = chrono::NaiveDate::parse_from_str(cleaned, "%Y%m%d").ok()?;
        Some(parsed.and_hms_opt(0, 0, 0)?.and_utc())
    } else if let Some(without_z) = cleaned.strip_suffix('Z') {
        chrono::NaiveDateTime::parse_from_str(without_z, "%Y%m%dT%H%M%S")
            .ok()
            .map(|dt| dt.and_utc())
    } else {
        chrono::NaiveDateTime::parse_from_str(cleaned, "%Y%m%dT%H%M%S")
            .ok()
            .map(|dt| dt.and_utc())
    }
}

/// In-memory address book store with optional SQLite persistence.
#[derive(Debug, Clone)]
pub struct InMemoryAddressBookStore {
    address_books: DashMap<String, AddressBookData>,
    #[cfg(feature = "persistence")]
    db: Option<DbHandle>,
}

impl InMemoryAddressBookStore {
    /// Create a new empty in-memory address book store.
    pub fn new() -> Self {
        Self {
            address_books: DashMap::new(),
            #[cfg(feature = "persistence")]
            db: None,
        }
    }

    #[cfg(feature = "persistence")]
    /// Create an in-memory address book store backed by a shared SQLite database.
    pub fn with_db(db: DbHandle) -> Self {
        {
            let conn = db.lock().unwrap_or_else(|e| e.into_inner());
            let _ = conn.execute_batch(
                "
                CREATE TABLE IF NOT EXISTS address_books (
                    principal TEXT NOT NULL,
                    book_id TEXT NOT NULL,
                    name TEXT NOT NULL DEFAULT '',
                    PRIMARY KEY (principal, book_id)
                );
                CREATE TABLE IF NOT EXISTS contacts (
                    book_id TEXT NOT NULL,
                    uid TEXT NOT NULL,
                    vcard_data TEXT NOT NULL,
                    etag TEXT NOT NULL DEFAULT '',
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    PRIMARY KEY (book_id, uid)
                );
                ",
            );
        }

        let store = Self {
            address_books: DashMap::new(),
            db: Some(db.clone()),
        };

        store.load_all_from_db(&db);
        store
    }

    #[cfg(feature = "persistence")]
    fn load_all_from_db(&self, db: &DbHandle) {
        let conn = match db.lock() {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to lock DB for loading address books: {}", e);
                return;
            }
        };

        if let Ok(mut stmt) = conn.prepare("SELECT principal, book_id, name FROM address_books") {
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            });
            if let Ok(rows) = rows {
                for row in rows.flatten() {
                    let (principal, book_id, name) = row;
                    let now = Utc::now();
                    let key = Self::book_key(&principal, &book_id);
                    self.address_books.insert(
                        key,
                        AddressBookData {
                            info: AddressBookInfo {
                                id: book_id,
                                principal,
                                name,
                                ctag: Self::next_ctag(),
                                created_at: now,
                                updated_at: now,
                            },
                            contacts: DashMap::new(),
                        },
                    );
                }
            }
        }

        if let Ok(mut stmt) = conn
            .prepare("SELECT book_id, uid, vcard_data, etag, created_at, updated_at FROM contacts")
        {
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            });
            if let Ok(rows) = rows {
                for row in rows.flatten() {
                    let (book_id, uid, vcard_data, etag, created_at_str, updated_at_str) = row;
                    let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now());
                    let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now());
                    for entry in self.address_books.iter() {
                        if entry.value().info.id == book_id {
                            entry.value().contacts.insert(
                                uid.clone(),
                                ContactInfo {
                                    uid,
                                    address_book_id: book_id.clone(),
                                    vcard_data,
                                    etag,
                                    created_at,
                                    updated_at,
                                },
                            );
                            break;
                        }
                    }
                }
            }
        }
    }

    fn book_key(principal: &str, book_id: &str) -> String {
        format!("{}:{}", principal, book_id)
    }

    fn next_ctag() -> String {
        uuid::Uuid::new_v4().to_string()[..8].to_string()
    }
}

impl Default for InMemoryAddressBookStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AddressBookStore for InMemoryAddressBookStore {
    async fn list_address_books(&self, principal: &str) -> Vec<AddressBookInfo> {
        let mut result = Vec::new();
        for entry in self.address_books.iter() {
            if entry.value().info.principal == principal {
                result.push(entry.value().info.clone());
            }
        }
        result
    }

    async fn get_address_book(&self, principal: &str, book_id: &str) -> Option<AddressBookInfo> {
        let key = Self::book_key(principal, book_id);
        self.address_books.get(&key).map(|e| e.value().info.clone())
    }

    async fn create_address_book(
        &self,
        principal: &str,
        name: &str,
    ) -> StoreResult<AddressBookInfo> {
        let id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        let now = Utc::now();
        let info = AddressBookInfo {
            id: id.clone(),
            principal: principal.to_string(),
            name: name.to_string(),
            ctag: Self::next_ctag(),
            created_at: now,
            updated_at: now,
        };
        let key = Self::book_key(principal, &id);
        if self.address_books.contains_key(&key) {
            return Err(StoreError("Address book already exists".to_string()));
        }
        self.address_books.insert(
            key,
            AddressBookData {
                info: info.clone(),
                contacts: DashMap::new(),
            },
        );

        #[cfg(feature = "persistence")]
        if let Some(ref db) = self.db {
            if let Ok(conn) = db.lock() {
                if let Err(e) = conn.execute(
                    "INSERT OR REPLACE INTO address_books (principal, book_id, name) VALUES (?1, ?2, ?3)",
                    rusqlite::params![info.principal, info.id, info.name],
                ) {
                    warn!("Failed to persist address book to SQLite: {}", e);
                }
            }
        }

        Ok(info)
    }

    async fn delete_address_book(&self, principal: &str, book_id: &str) -> StoreResult<()> {
        let key = Self::book_key(principal, book_id);
        if self.address_books.remove(&key).is_none() {
            return Err(StoreError("Address book not found".to_string()));
        }

        #[cfg(feature = "persistence")]
        if let Some(ref db) = self.db {
            if let Ok(conn) = db.lock() {
                if let Err(e) = conn.execute(
                    "DELETE FROM contacts WHERE book_id = ?1",
                    rusqlite::params![book_id],
                ) {
                    warn!("Failed to delete contacts from SQLite: {}", e);
                }
                if let Err(e) = conn.execute(
                    "DELETE FROM address_books WHERE principal = ?1 AND book_id = ?2",
                    rusqlite::params![principal, book_id],
                ) {
                    warn!("Failed to delete address book from SQLite: {}", e);
                }
            }
        }

        Ok(())
    }

    async fn list_contacts(&self, book_id: &str) -> Vec<ContactInfo> {
        let mut result = Vec::new();
        for entry in self.address_books.iter() {
            if entry.value().info.id == book_id {
                for contact_entry in entry.value().contacts.iter() {
                    result.push(contact_entry.value().clone());
                }
            }
        }
        result
    }

    async fn get_contact(&self, book_id: &str, contact_uid: &str) -> Option<ContactInfo> {
        for entry in self.address_books.iter() {
            if entry.value().info.id == book_id
                && let Some(contact) = entry.value().contacts.get(contact_uid)
            {
                return Some(contact.value().clone());
            }
        }
        None
    }

    async fn create_contact(&self, book_id: &str, vcard: &str) -> StoreResult<ContactInfo> {
        let uid = crate::vcard::parse_vcard(vcard)
            .ok()
            .and_then(|v| v.uid)
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let now = Utc::now();
        let etag = format!("\"{}\"", now.timestamp());
        let contact = ContactInfo {
            uid: uid.clone(),
            address_book_id: book_id.to_string(),
            vcard_data: vcard.to_string(),
            etag: etag.clone(),
            created_at: now,
            updated_at: now,
        };

        let ab_key = self
            .address_books
            .iter()
            .find(|e| e.value().info.id == book_id)
            .map(|e| e.key().clone());

        let Some(ab_key) = ab_key else {
            return Err(StoreError("Address book not found".to_string()));
        };

        let Some(mut ab_entry) = self.address_books.get_mut(&ab_key) else {
            return Err(StoreError("Address book not found".to_string()));
        };

        if ab_entry.contacts.contains_key(&uid) {
            return Err(StoreError("Contact already exists".to_string()));
        }
        ab_entry.contacts.insert(uid.clone(), contact.clone());
        let new_ctag = Self::next_ctag();
        #[cfg(feature = "persistence")]
        let principal = ab_entry.info.principal.clone();
        ab_entry.info.ctag = new_ctag.clone();

        #[cfg(feature = "persistence")]
        if let Some(ref db) = self.db {
            if let Ok(conn) = db.lock() {
                if let Err(e) = conn.execute(
                    "INSERT OR REPLACE INTO contacts (book_id, uid, vcard_data, etag, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![
                        contact.address_book_id,
                        contact.uid,
                        contact.vcard_data,
                        contact.etag,
                        contact.created_at.to_rfc3339(),
                        contact.updated_at.to_rfc3339(),
                    ],
                ) {
                    warn!("Failed to persist contact to SQLite: {}", e);
                }
                if let Err(e) = conn.execute(
                    "UPDATE address_books SET ctag = ?1 WHERE principal = ?2 AND book_id = ?3",
                    rusqlite::params![new_ctag, principal, ab_entry.info.id],
                ) {
                    warn!("Failed to persist address book ctag to SQLite: {}", e);
                }
            }
        }

        Ok(contact)
    }

    async fn update_contact(
        &self,
        book_id: &str,
        contact_uid: &str,
        vcard: &str,
    ) -> StoreResult<ContactInfo> {
        let now = Utc::now();
        let etag = format!("\"{}\"", now.timestamp());

        let ab_key = self
            .address_books
            .iter()
            .find(|e| e.value().info.id == book_id)
            .map(|e| e.key().clone());

        let Some(ab_key) = ab_key else {
            return Err(StoreError("Address book not found".to_string()));
        };

        let Some(mut ab_entry) = self.address_books.get_mut(&ab_key) else {
            return Err(StoreError("Address book not found".to_string()));
        };

        let mut contact = ab_entry
            .contacts
            .get(contact_uid)
            .ok_or_else(|| StoreError("Contact not found".to_string()))?
            .value()
            .clone();
        contact.vcard_data = vcard.to_string();
        contact.etag = etag.clone();
        contact.updated_at = now;
        ab_entry
            .contacts
            .insert(contact_uid.to_string(), contact.clone());
        let new_ctag = Self::next_ctag();
        #[cfg(feature = "persistence")]
        let principal = ab_entry.info.principal.clone();
        ab_entry.info.ctag = new_ctag.clone();

        #[cfg(feature = "persistence")]
        if let Some(ref db) = self.db {
            if let Ok(conn) = db.lock() {
                if let Err(e) = conn.execute(
                    "INSERT OR REPLACE INTO contacts (book_id, uid, vcard_data, etag, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![
                        contact.address_book_id,
                        contact.uid,
                        contact.vcard_data,
                        contact.etag,
                        contact.created_at.to_rfc3339(),
                        contact.updated_at.to_rfc3339(),
                    ],
                ) {
                    warn!("Failed to persist contact update to SQLite: {}", e);
                }
                if let Err(e) = conn.execute(
                    "UPDATE address_books SET ctag = ?1 WHERE principal = ?2 AND book_id = ?3",
                    rusqlite::params![new_ctag, principal, ab_entry.info.id],
                ) {
                    warn!("Failed to persist address book ctag to SQLite: {}", e);
                }
            }
        }

        Ok(contact)
    }

    async fn delete_contact(&self, book_id: &str, contact_uid: &str) -> StoreResult<()> {
        let ab_key = self
            .address_books
            .iter()
            .find(|e| e.value().info.id == book_id)
            .map(|e| e.key().clone());

        let Some(ab_key) = ab_key else {
            return Err(StoreError("Address book not found".to_string()));
        };

        let Some(mut ab_entry) = self.address_books.get_mut(&ab_key) else {
            return Err(StoreError("Address book not found".to_string()));
        };

        if ab_entry.contacts.remove(contact_uid).is_none() {
            return Err(StoreError("Contact not found".to_string()));
        }
        let new_ctag = Self::next_ctag();
        #[cfg(feature = "persistence")]
        let principal = ab_entry.info.principal.clone();
        ab_entry.info.ctag = new_ctag.clone();

        #[cfg(feature = "persistence")]
        if let Some(ref db) = self.db {
            if let Ok(conn) = db.lock() {
                if let Err(e) = conn.execute(
                    "DELETE FROM contacts WHERE book_id = ?1 AND uid = ?2",
                    rusqlite::params![book_id, contact_uid],
                ) {
                    warn!("Failed to delete contact from SQLite: {}", e);
                }
                if let Err(e) = conn.execute(
                    "UPDATE address_books SET ctag = ?1 WHERE principal = ?2 AND book_id = ?3",
                    rusqlite::params![new_ctag, principal, ab_entry.info.id],
                ) {
                    warn!("Failed to persist address book ctag to SQLite: {}", e);
                }
            }
        }

        Ok(())
    }
}

/// Type-erased calendar store reference for use in async contexts.
pub type DynCalendarStore = Arc<dyn CalendarStore>;
/// Type-erased address book store reference for use in async contexts.
pub type DynAddressBookStore = Arc<dyn AddressBookStore>;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_calendar_create_and_list() {
        let store = InMemoryCalendarStore::new();
        let info = store
            .create_calendar("user1", "Personal", "#ff0000")
            .await
            .unwrap();
        assert_eq!(info.name, "Personal");
        assert_eq!(info.color, "#ff0000");

        let calendars = store.list_calendars("user1").await;
        assert_eq!(calendars.len(), 1);
    }

    #[tokio::test]
    async fn test_address_book_create_and_list() {
        let store = InMemoryAddressBookStore::new();
        let info = store
            .create_address_book("user1", "Contacts")
            .await
            .unwrap();
        assert_eq!(info.name, "Contacts");

        let books = store.list_address_books("user1").await;
        assert_eq!(books.len(), 1);
    }

    #[cfg(feature = "persistence")]
    #[tokio::test]
    async fn test_calendar_persistence_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        let db: DbHandle = Arc::new(std::sync::Mutex::new(conn));

        let store1 = InMemoryCalendarStore::with_db(db.clone());
        let info = store1
            .create_calendar("user1", "Work", "#0000ff")
            .await
            .unwrap();
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
        let info = store1
            .create_address_book("user1", "Personal")
            .await
            .unwrap();
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
        let info = store1
            .create_calendar("user1", "Temp", "#ff0000")
            .await
            .unwrap();
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
}
