use super::*;

#[derive(Debug, Clone)]
struct CalendarData {
    info: CalendarInfo,
    events: DashMap<String, EventInfo>,
}

/// In-memory calendar store with optional `SQLite` persistence.
#[derive(Debug, Clone)]
pub struct InMemoryCalendarStore {
    calendars: DashMap<String, CalendarData>,
    #[cfg(feature = "persistence")]
    db: Option<DbHandle>,
}

impl InMemoryCalendarStore {
    /// Create a new empty in-memory calendar store.
    #[must_use]
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
        load_calendars_from_db(&self.calendars, &conn);
        load_events_from_db(&self.calendars, &conn);
    }

    fn calendar_key(principal: &str, calendar_id: &str) -> String {
        format!("{principal}:{calendar_id}")
    }

    fn next_ctag() -> String {
        uuid::Uuid::new_v4().to_string()[..8].to_string()
    }
}

#[cfg(feature = "persistence")]
fn load_calendars_from_db(calendars: &DashMap<String, CalendarData>, conn: &rusqlite::Connection) {
    let Ok(mut stmt) = conn.prepare("SELECT principal, calendar_id, name, color, ctag FROM calendars") else {
        return;
    };
    let Ok(rows) = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
        ))
    }) else {
        return;
    };
    for (principal, calendar_id, name, color, ctag) in rows.flatten() {
        insert_calendar_row(calendars, principal, calendar_id, name, color, ctag);
    }
}

#[cfg(feature = "persistence")]
fn insert_calendar_row(
    calendars: &DashMap<String, CalendarData>,
    principal: String,
    calendar_id: String,
    name: String,
    color: String,
    ctag: String,
) {
    let now = Utc::now();
    let key = format!("{principal}:{calendar_id}");
    calendars.insert(
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

#[cfg(feature = "persistence")]
fn load_events_from_db(calendars: &DashMap<String, CalendarData>, conn: &rusqlite::Connection) {
    let Ok(mut stmt) =
        conn.prepare("SELECT calendar_id, uid, ical_data, etag, created_at, updated_at FROM calendar_events")
    else {
        return;
    };
    let Ok(rows) = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
        ))
    }) else {
        return;
    };
    for row in rows.flatten() {
        insert_event_row(calendars, row);
    }
}

#[cfg(feature = "persistence")]
fn insert_event_row(
    calendars: &DashMap<String, CalendarData>,
    (calendar_id, uid, ical_data, etag, created_str, updated_str): (String, String, String, String, String, String),
) {
    let created_at = parse_rfc3339_or_now(&created_str);
    let updated_at = parse_rfc3339_or_now(&updated_str);
    for entry in calendars.iter() {
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

#[cfg(feature = "persistence")]
fn parse_rfc3339_or_now(s: &str) -> DateTime<Utc> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn parse_uid_from_ical(ical: &str) -> String {
    crate::ical::parse_ical(ical)
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
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
}

fn make_event_info(ical: &str, calendar_id: &str) -> EventInfo {
    let uid = parse_uid_from_ical(ical);
    let now = Utc::now();
    EventInfo {
        uid,
        calendar_id: calendar_id.to_string(),
        ical_data: ical.to_string(),
        etag: format!("\"{}\"", now.timestamp()),
        created_at: now,
        updated_at: now,
    }
}

fn update_event_fields(event: &mut EventInfo, ical: &str) {
    let now = Utc::now();
    event.ical_data = ical.to_string();
    event.etag = format!("\"{}\"", now.timestamp());
    event.updated_at = now;
}

fn find_calendar_key(calendars: &DashMap<String, CalendarData>, calendar_id: &str) -> Option<String> {
    calendars
        .iter()
        .find(|e| e.value().info.id == calendar_id)
        .map(|e| e.key().clone())
}

fn make_calendar_info(principal: &str, name: &str, color: &str) -> CalendarInfo {
    let now = Utc::now();
    CalendarInfo {
        id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
        principal: principal.to_string(),
        name: name.to_string(),
        color: color.to_string(),
        ctag: InMemoryCalendarStore::next_ctag(),
        created_at: now,
        updated_at: now,
    }
}

#[cfg(feature = "persistence")]
fn persist_event_insert(db: &DbHandle, event: &EventInfo) {
    let Ok(conn) = db.lock() else {
        return;
    };
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
}

#[cfg(feature = "persistence")]
fn persist_event_delete(db: &DbHandle, calendar_id: &str, uid: &str) {
    let Ok(conn) = db.lock() else {
        return;
    };
    if let Err(e) = conn.execute(
        "DELETE FROM calendar_events WHERE calendar_id = ?1 AND uid = ?2",
        rusqlite::params![calendar_id, uid],
    ) {
        warn!("Failed to delete event from SQLite: {}", e);
    }
}

#[cfg(feature = "persistence")]
fn persist_calendar_ctag(db: &DbHandle, principal: &str, calendar_id: &str, ctag: &str) {
    let Ok(conn) = db.lock() else {
        return;
    };
    if let Err(e) = conn.execute(
        "UPDATE calendars SET ctag = ?1 WHERE principal = ?2 AND calendar_id = ?3",
        rusqlite::params![ctag, principal, calendar_id],
    ) {
        warn!("Failed to persist calendar ctag to SQLite: {}", e);
    }
}

#[cfg(feature = "persistence")]
fn persist_calendar_insert(db: &DbHandle, info: &CalendarInfo) {
    let Ok(conn) = db.lock() else {
        return;
    };
    if let Err(e) = conn.execute(
        "INSERT OR REPLACE INTO calendars (principal, calendar_id, name, color, ctag) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![info.principal, info.id, info.name, info.color, info.ctag],
    ) {
        warn!("Failed to persist calendar to SQLite: {}", e);
    }
}

#[cfg(feature = "persistence")]
fn persist_calendar_delete(db: &DbHandle, principal: &str, calendar_id: &str) {
    let Ok(conn) = db.lock() else {
        return;
    };
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

fn event_matches_filter(event: &EventInfo, filter: &CalFilter) -> bool {
    let comps = match crate::ical::parse_ical(&event.ical_data) {
        Ok(c) => c,
        Err(_) => return true,
    };
    let vevent = comps.iter().find_map(|c| {
        if c.name == "VCALENDAR" {
            c.children.iter().find(|ch| ch.name == "VEVENT" || ch.name == "VTODO")
        } else {
            None
        }
    });
    let Some(vevent) = vevent else { return true };
    let dtstart = crate::ical::get_first_prop(vevent, "DTSTART").and_then(|p| parse_ical_datetime(&p.value, &p.params));
    let dtend = crate::ical::get_first_prop(vevent, "DTEND").and_then(|p| parse_ical_datetime(&p.value, &p.params));
    time_range_overlaps(dtstart, dtend, filter.start, filter.end)
}

fn time_range_overlaps(
    dtstart: Option<DateTime<Utc>>,
    dtend: Option<DateTime<Utc>>,
    filter_start: Option<DateTime<Utc>>,
    filter_end: Option<DateTime<Utc>>,
) -> bool {
    match (dtstart, dtend, filter_start, filter_end) {
        (Some(s), Some(e), Some(fs), Some(fe)) => s < fe && e > fs,
        (Some(s), _, Some(fs), Some(fe)) => s >= fs && s < fe,
        (Some(s), _, Some(fs), None) => s >= fs,
        (Some(s), _, None, Some(fe)) => s < fe,
        (_, Some(e), Some(fs), Some(fe)) => e > fs && e <= fe,
        (None, None, _, _) => true,
        _ => true,
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
        for entry in &self.calendars {
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

    async fn create_calendar(&self, principal: &str, name: &str, color: &str) -> StoreResult<CalendarInfo> {
        let info = make_calendar_info(principal, name, color);
        let key = Self::calendar_key(principal, &info.id);
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
            persist_calendar_insert(db, &info);
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
            persist_calendar_delete(db, principal, calendar_id);
        }
        Ok(())
    }

    async fn list_events(&self, calendar_id: &str) -> Vec<EventInfo> {
        let mut result = Vec::new();
        for entry in &self.calendars {
            if entry.value().info.id == calendar_id {
                for event_entry in &entry.value().events {
                    result.push(event_entry.value().clone());
                }
            }
        }
        result
    }

    async fn get_event(&self, calendar_id: &str, event_uid: &str) -> Option<EventInfo> {
        for entry in &self.calendars {
            if entry.value().info.id == calendar_id
                && let Some(event) = entry.value().events.get(event_uid)
            {
                return Some(event.value().clone());
            }
        }
        None
    }

    async fn create_event(&self, calendar_id: &str, ical: &str) -> StoreResult<EventInfo> {
        let event = make_event_info(ical, calendar_id);
        let cal_key = find_calendar_key(&self.calendars, calendar_id)
            .ok_or_else(|| StoreError("Calendar not found".to_string()))?;
        let Some(mut cal_entry) = self.calendars.get_mut(&cal_key) else {
            return Err(StoreError("Calendar not found".to_string()));
        };
        if cal_entry.events.contains_key(&event.uid) {
            return Err(StoreError("Event already exists".to_string()));
        }
        cal_entry.events.insert(event.uid.clone(), event.clone());
        cal_entry.info.ctag = Self::next_ctag();
        #[cfg(feature = "persistence")]
        if let Some(ref db) = self.db {
            persist_event_insert(db, &event);
            persist_calendar_ctag(db, &cal_entry.info.principal, &cal_entry.info.id, &cal_entry.info.ctag);
        }
        Ok(event)
    }

    async fn update_event(&self, calendar_id: &str, event_uid: &str, ical: &str) -> StoreResult<EventInfo> {
        let cal_key = find_calendar_key(&self.calendars, calendar_id)
            .ok_or_else(|| StoreError("Calendar not found".to_string()))?;
        let Some(mut cal_entry) = self.calendars.get_mut(&cal_key) else {
            return Err(StoreError("Calendar not found".to_string()));
        };
        let mut event = cal_entry
            .events
            .get(event_uid)
            .ok_or_else(|| StoreError("Event not found".to_string()))?
            .value()
            .clone();
        update_event_fields(&mut event, ical);
        cal_entry.events.insert(event_uid.to_string(), event.clone());
        cal_entry.info.ctag = Self::next_ctag();
        #[cfg(feature = "persistence")]
        if let Some(ref db) = self.db {
            persist_event_insert(db, &event);
            persist_calendar_ctag(db, &cal_entry.info.principal, &cal_entry.info.id, &cal_entry.info.ctag);
        }
        Ok(event)
    }

    async fn delete_event(&self, calendar_id: &str, event_uid: &str) -> StoreResult<()> {
        let cal_key = find_calendar_key(&self.calendars, calendar_id)
            .ok_or_else(|| StoreError("Calendar not found".to_string()))?;
        let Some(mut cal_entry) = self.calendars.get_mut(&cal_key) else {
            return Err(StoreError("Calendar not found".to_string()));
        };
        if cal_entry.events.remove(event_uid).is_none() {
            return Err(StoreError("Event not found".to_string()));
        }
        cal_entry.info.ctag = Self::next_ctag();
        #[cfg(feature = "persistence")]
        if let Some(ref db) = self.db {
            persist_event_delete(db, calendar_id, event_uid);
            persist_calendar_ctag(db, &cal_entry.info.principal, &cal_entry.info.id, &cal_entry.info.ctag);
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
            .filter(|event| event_matches_filter(event, filter))
            .collect()
    }
}

fn parse_ical_datetime(value: &str, params: &hashbrown::HashMap<String, String>) -> Option<DateTime<Utc>> {
    let is_date = params.get("VALUE").map(std::string::String::as_str) == Some("DATE");

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
