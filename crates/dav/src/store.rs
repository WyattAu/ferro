use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CalendarInfo {
    pub id: String,
    pub principal: String,
    pub name: String,
    pub color: String,
    pub ctag: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EventInfo {
    pub uid: String,
    pub calendar_id: String,
    pub ical_data: String,
    pub etag: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CalFilter {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AddressBookInfo {
    pub id: String,
    pub principal: String,
    pub name: String,
    pub ctag: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContactInfo {
    pub uid: String,
    pub address_book_id: String,
    pub vcard_data: String,
    pub etag: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct StoreError(String);

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for StoreError {}

pub type StoreResult<T> = Result<T, StoreError>;

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

#[derive(Debug, Clone)]
pub struct InMemoryCalendarStore {
    calendars: DashMap<String, CalendarData>,
}

impl InMemoryCalendarStore {
    pub fn new() -> Self {
        Self {
            calendars: DashMap::new(),
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
        Ok(info)
    }

    async fn delete_calendar(&self, principal: &str, calendar_id: &str) -> StoreResult<()> {
        let key = Self::calendar_key(principal, calendar_id);
        if self.calendars.remove(&key).is_none() {
            return Err(StoreError("Calendar not found".to_string()));
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
                                crate::ical::get_first_prop(child, "UID")
                                    .map(|p| p.value.clone())
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

        for entry in self.calendars.iter() {
            if entry.value().info.id == calendar_id {
                if entry.value().events.contains_key(&uid) {
                    return Err(StoreError("Event already exists".to_string()));
                }
                entry.value().events.insert(uid.clone(), event.clone());
                return Ok(event);
            }
        }

        Err(StoreError("Calendar not found".to_string()))
    }

    async fn update_event(
        &self,
        calendar_id: &str,
        event_uid: &str,
        ical: &str,
    ) -> StoreResult<EventInfo> {
        let now = Utc::now();
        let etag = format!("\"{}\"", now.timestamp());

        for entry in self.calendars.iter() {
            if entry.value().info.id == calendar_id {
                let mut event = entry
                    .value()
                    .events
                    .get(event_uid)
                    .ok_or_else(|| StoreError("Event not found".to_string()))?
                    .value()
                    .clone();
                event.ical_data = ical.to_string();
                event.etag = etag.clone();
                event.updated_at = now;
                entry.value().events.insert(event_uid.to_string(), event.clone());
                return Ok(event);
            }
        }

        Err(StoreError("Calendar not found".to_string()))
    }

    async fn delete_event(&self, calendar_id: &str, event_uid: &str) -> StoreResult<()> {
        for entry in self.calendars.iter() {
            if entry.value().info.id == calendar_id {
                if entry.value().events.remove(event_uid).is_none() {
                    return Err(StoreError("Event not found".to_string()));
                }
                return Ok(());
            }
        }
        Err(StoreError("Calendar not found".to_string()))
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
                        c.children.iter().find(|ch| ch.name == "VEVENT")
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

#[derive(Debug, Clone)]
pub struct InMemoryAddressBookStore {
    address_books: DashMap<String, AddressBookData>,
}

impl InMemoryAddressBookStore {
    pub fn new() -> Self {
        Self {
            address_books: DashMap::new(),
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
        Ok(info)
    }

    async fn delete_address_book(&self, principal: &str, book_id: &str) -> StoreResult<()> {
        let key = Self::book_key(principal, book_id);
        if self.address_books.remove(&key).is_none() {
            return Err(StoreError("Address book not found".to_string()));
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

        for entry in self.address_books.iter() {
            if entry.value().info.id == book_id {
                if entry.value().contacts.contains_key(&uid) {
                    return Err(StoreError("Contact already exists".to_string()));
                }
                entry
                    .value()
                    .contacts
                    .insert(uid.clone(), contact.clone());
                return Ok(contact);
            }
        }

        Err(StoreError("Address book not found".to_string()))
    }

    async fn update_contact(
        &self,
        book_id: &str,
        contact_uid: &str,
        vcard: &str,
    ) -> StoreResult<ContactInfo> {
        let now = Utc::now();
        let etag = format!("\"{}\"", now.timestamp());

        for entry in self.address_books.iter() {
            if entry.value().info.id == book_id {
                let mut contact = entry
                    .value()
                    .contacts
                    .get(contact_uid)
                    .ok_or_else(|| StoreError("Contact not found".to_string()))?
                    .value()
                    .clone();
                contact.vcard_data = vcard.to_string();
                contact.etag = etag.clone();
                contact.updated_at = now;
                entry
                    .value()
                    .contacts
                    .insert(contact_uid.to_string(), contact.clone());
                return Ok(contact);
            }
        }

        Err(StoreError("Address book not found".to_string()))
    }

    async fn delete_contact(&self, book_id: &str, contact_uid: &str) -> StoreResult<()> {
        for entry in self.address_books.iter() {
            if entry.value().info.id == book_id {
                if entry.value().contacts.remove(contact_uid).is_none() {
                    return Err(StoreError("Contact not found".to_string()));
                }
                return Ok(());
            }
        }
        Err(StoreError("Address book not found".to_string()))
    }
}

pub type DynCalendarStore = Arc<dyn CalendarStore>;
pub type DynAddressBookStore = Arc<dyn AddressBookStore>;
