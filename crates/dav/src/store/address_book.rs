use super::*;

#[derive(Debug, Clone)]
struct AddressBookData {
    info: AddressBookInfo,
    contacts: DashMap<String, ContactInfo>,
}

/// In-memory address book store with optional `SQLite` persistence.
#[derive(Debug, Clone)]
pub struct InMemoryAddressBookStore {
    address_books: DashMap<String, AddressBookData>,
    #[cfg(feature = "persistence")]
    db: Option<DbHandle>,
}

impl InMemoryAddressBookStore {
    /// Create a new empty in-memory address book store.
    #[must_use]
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
        load_address_books_from_db(&self.address_books, &conn);
        load_contacts_from_db(&self.address_books, &conn);
    }

    fn book_key(principal: &str, book_id: &str) -> String {
        format!("{principal}:{book_id}")
    }

    fn next_ctag() -> String {
        uuid::Uuid::new_v4().to_string()[..8].to_string()
    }
}

#[cfg(feature = "persistence")]
fn load_address_books_from_db(address_books: &DashMap<String, AddressBookData>, conn: &rusqlite::Connection) {
    let Ok(mut stmt) = conn.prepare("SELECT principal, book_id, name FROM address_books") else {
        return;
    };
    let Ok(rows) = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    }) else {
        return;
    };
    for row in rows.flatten() {
        let (principal, book_id, name) = row;
        let now = Utc::now();
        let key = format!("{principal}:{book_id}");
        address_books.insert(
            key,
            AddressBookData {
                info: AddressBookInfo {
                    id: book_id,
                    principal,
                    name,
                    ctag: InMemoryAddressBookStore::next_ctag(),
                    created_at: now,
                    updated_at: now,
                },
                contacts: DashMap::new(),
            },
        );
    }
}

#[cfg(feature = "persistence")]
fn load_contacts_from_db(address_books: &DashMap<String, AddressBookData>, conn: &rusqlite::Connection) {
    let Ok(mut stmt) = conn.prepare("SELECT book_id, uid, vcard_data, etag, created_at, updated_at FROM contacts")
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
        let (book_id, uid, vcard_data, etag, created_at_str, updated_at_str) = row;
        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        for entry in address_books.iter() {
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

fn parse_uid_from_vcard(vcard: &str) -> String {
    crate::vcard::parse_vcard(vcard)
        .ok()
        .and_then(|v| v.uid)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
}

fn make_contact_info(vcard: &str, book_id: &str) -> ContactInfo {
    let uid = parse_uid_from_vcard(vcard);
    let now = Utc::now();
    ContactInfo {
        uid,
        address_book_id: book_id.to_string(),
        vcard_data: vcard.to_string(),
        etag: format!("\"{}\"", now.timestamp()),
        created_at: now,
        updated_at: now,
    }
}

fn update_contact_fields(contact: &mut ContactInfo, vcard: &str) {
    let now = Utc::now();
    contact.vcard_data = vcard.to_string();
    contact.etag = format!("\"{}\"", now.timestamp());
    contact.updated_at = now;
}

fn find_address_book_key(address_books: &DashMap<String, AddressBookData>, book_id: &str) -> Option<String> {
    address_books
        .iter()
        .find(|e| e.value().info.id == book_id)
        .map(|e| e.key().clone())
}

fn make_address_book_info(principal: &str, name: &str) -> AddressBookInfo {
    let now = Utc::now();
    AddressBookInfo {
        id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
        principal: principal.to_string(),
        name: name.to_string(),
        ctag: InMemoryAddressBookStore::next_ctag(),
        created_at: now,
        updated_at: now,
    }
}

#[cfg(feature = "persistence")]
fn persist_contact_insert(db: &DbHandle, contact: &ContactInfo) {
    let Ok(conn) = db.lock() else {
        return;
    };
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
}

#[cfg(feature = "persistence")]
fn persist_contact_delete(db: &DbHandle, book_id: &str, uid: &str) {
    let Ok(conn) = db.lock() else {
        return;
    };
    if let Err(e) = conn.execute(
        "DELETE FROM contacts WHERE book_id = ?1 AND uid = ?2",
        rusqlite::params![book_id, uid],
    ) {
        warn!("Failed to delete contact from SQLite: {}", e);
    }
}

#[cfg(feature = "persistence")]
fn persist_address_book_ctag(db: &DbHandle, principal: &str, book_id: &str, ctag: &str) {
    let Ok(conn) = db.lock() else {
        return;
    };
    if let Err(e) = conn.execute(
        "UPDATE address_books SET ctag = ?1 WHERE principal = ?2 AND book_id = ?3",
        rusqlite::params![ctag, principal, book_id],
    ) {
        warn!("Failed to persist address book ctag to SQLite: {}", e);
    }
}

#[cfg(feature = "persistence")]
fn persist_address_book_insert(db: &DbHandle, info: &AddressBookInfo) {
    let Ok(conn) = db.lock() else {
        return;
    };
    if let Err(e) = conn.execute(
        "INSERT OR REPLACE INTO address_books (principal, book_id, name) VALUES (?1, ?2, ?3)",
        rusqlite::params![info.principal, info.id, info.name],
    ) {
        warn!("Failed to persist address book to SQLite: {}", e);
    }
}

#[cfg(feature = "persistence")]
fn persist_address_book_delete(db: &DbHandle, principal: &str, book_id: &str) {
    let Ok(conn) = db.lock() else {
        return;
    };
    if let Err(e) = conn.execute("DELETE FROM contacts WHERE book_id = ?1", rusqlite::params![book_id]) {
        warn!("Failed to delete contacts from SQLite: {}", e);
    }
    if let Err(e) = conn.execute(
        "DELETE FROM address_books WHERE principal = ?1 AND book_id = ?2",
        rusqlite::params![principal, book_id],
    ) {
        warn!("Failed to delete address book from SQLite: {}", e);
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
        for entry in &self.address_books {
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

    async fn create_address_book(&self, principal: &str, name: &str) -> StoreResult<AddressBookInfo> {
        let info = make_address_book_info(principal, name);
        let key = Self::book_key(principal, &info.id);
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
            persist_address_book_insert(db, &info);
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
            persist_address_book_delete(db, principal, book_id);
        }
        Ok(())
    }

    async fn list_contacts(&self, book_id: &str) -> Vec<ContactInfo> {
        let mut result = Vec::new();
        for entry in &self.address_books {
            if entry.value().info.id == book_id {
                for contact_entry in &entry.value().contacts {
                    result.push(contact_entry.value().clone());
                }
            }
        }
        result
    }

    async fn get_contact(&self, book_id: &str, contact_uid: &str) -> Option<ContactInfo> {
        for entry in &self.address_books {
            if entry.value().info.id == book_id
                && let Some(contact) = entry.value().contacts.get(contact_uid)
            {
                return Some(contact.value().clone());
            }
        }
        None
    }

    async fn create_contact(&self, book_id: &str, vcard: &str) -> StoreResult<ContactInfo> {
        let contact = make_contact_info(vcard, book_id);
        let ab_key = find_address_book_key(&self.address_books, book_id)
            .ok_or_else(|| StoreError("Address book not found".to_string()))?;
        let Some(mut ab_entry) = self.address_books.get_mut(&ab_key) else {
            return Err(StoreError("Address book not found".to_string()));
        };
        if ab_entry.contacts.contains_key(&contact.uid) {
            return Err(StoreError("Contact already exists".to_string()));
        }
        ab_entry.contacts.insert(contact.uid.clone(), contact.clone());
        ab_entry.info.ctag = Self::next_ctag();
        #[cfg(feature = "persistence")]
        if let Some(ref db) = self.db {
            persist_contact_insert(db, &contact);
            persist_address_book_ctag(db, &ab_entry.info.principal, &ab_entry.info.id, &ab_entry.info.ctag);
        }
        Ok(contact)
    }

    async fn update_contact(&self, book_id: &str, contact_uid: &str, vcard: &str) -> StoreResult<ContactInfo> {
        let ab_key = find_address_book_key(&self.address_books, book_id)
            .ok_or_else(|| StoreError("Address book not found".to_string()))?;
        let Some(mut ab_entry) = self.address_books.get_mut(&ab_key) else {
            return Err(StoreError("Address book not found".to_string()));
        };
        let mut contact = ab_entry
            .contacts
            .get(contact_uid)
            .ok_or_else(|| StoreError("Contact not found".to_string()))?
            .value()
            .clone();
        update_contact_fields(&mut contact, vcard);
        ab_entry.contacts.insert(contact_uid.to_string(), contact.clone());
        ab_entry.info.ctag = Self::next_ctag();
        #[cfg(feature = "persistence")]
        if let Some(ref db) = self.db {
            persist_contact_insert(db, &contact);
            persist_address_book_ctag(db, &ab_entry.info.principal, &ab_entry.info.id, &ab_entry.info.ctag);
        }
        Ok(contact)
    }

    async fn delete_contact(&self, book_id: &str, contact_uid: &str) -> StoreResult<()> {
        let ab_key = find_address_book_key(&self.address_books, book_id)
            .ok_or_else(|| StoreError("Address book not found".to_string()))?;
        let Some(mut ab_entry) = self.address_books.get_mut(&ab_key) else {
            return Err(StoreError("Address book not found".to_string()));
        };
        if ab_entry.contacts.remove(contact_uid).is_none() {
            return Err(StoreError("Contact not found".to_string()));
        }
        ab_entry.info.ctag = Self::next_ctag();
        #[cfg(feature = "persistence")]
        if let Some(ref db) = self.db {
            persist_contact_delete(db, book_id, contact_uid);
            persist_address_book_ctag(db, &ab_entry.info.principal, &ab_entry.info.id, &ab_entry.info.ctag);
        }
        Ok(())
    }
}
