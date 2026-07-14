# Integration Examples

## Basic Integration

### Calendar Application

```rust
use ferro_auth::users::{InMemoryUserStore, User, UserRole, UserStatus, UserStoreTrait, hash_password, ZeroizeString};
use ferro_dav::store::{CalendarStore, InMemoryCalendarStore, CalFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize stores
    let user_store = InMemoryUserStore::new();
    let calendar_store = InMemoryCalendarStore::new();
    
    // Create a user
    let user = User {
        id: uuid::Uuid::new_v4().to_string(),
        username: "alice".to_string(),
        display_name: "Alice Smith".to_string(),
        email: "alice@example.com".to_string(),
        role: UserRole::User,
        created_at: chrono::Utc::now(),
        last_login: None,
        status: UserStatus::Active,
        storage_quota_bytes: None,
        storage_used_bytes: 0,
        is_ldap: false,
        password_hash: Some(ZeroizeString::new(hash_password("secure_pass")?)),
        totp_secret: None,
        totp_enabled: false,
    };
    
    user_store.create_user(user.clone()).await?;
    
    // Create a calendar for the user
    let calendar = calendar_store.create_calendar(
        &format!("user:{}", user.username),
        "Personal Calendar",
        "#4285f4",
    ).await?;
    
    println!("Created calendar: {} ({})", calendar.name, calendar.id);
    
    // Add events
    let ical_data = r#"BEGIN:VCALENDAR
BEGIN:VEVENT
UID:event-001@example.com
DTSTART:20240115T090000Z
DTEND:20240115T100000Z
SUMMARY:Morning Standup
DESCRIPTION:Daily team sync
LOCATION:Conference Room A
END:VEVENT
END:VCALENDAR"#;
    
    let event = calendar_store.create_event(&calendar.id, ical_data).await?;
    println!("Created event: {}", event.uid);
    
    // Query events for today
    let today_start = chrono::Utc::now().date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
    let today_end = chrono::Utc::now().date_naive().and_hms_opt(23, 59, 59).unwrap().and_utc();
    
    let filter = CalFilter {
        start: Some(today_start),
        end: Some(today_end),
    };
    
    let events = calendar_store.query_events(&calendar.id, &filter).await;
    println!("Events today: {}", events.len());
    
    Ok(())
}
```

### Contact Management

```rust
use ferro_auth::users::{InMemoryUserStore, User, UserRole, UserStatus, UserStoreTrait, hash_password, ZeroizeString};
use ferro_dav::store::{AddressBookStore, InMemoryAddressBookStore};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize stores
    let user_store = InMemoryUserStore::new();
    let address_book_store = InMemoryAddressBookStore::new();
    
    // Create a user
    let user = User {
        id: uuid::Uuid::new_v4().to_string(),
        username: "bob".to_string(),
        display_name: "Bob Johnson".to_string(),
        email: "bob@example.com".to_string(),
        role: UserRole::User,
        created_at: chrono::Utc::now(),
        last_login: None,
        status: UserStatus::Active,
        storage_quota_bytes: None,
        storage_used_bytes: 0,
        is_ldap: false,
        password_hash: Some(ZeroizeString::new(hash_password("secure_pass")?)),
        totp_secret: None,
        totp_enabled: false,
    };
    
    user_store.create_user(user.clone()).await?;
    
    // Create an address book
    let address_book = address_book_store.create_address_book(
        &format!("user:{}", user.username),
        "My Contacts",
    ).await?;
    
    println!("Created address book: {} ({})", address_book.name, address_book.id);
    
    // Add contacts
    let vcard_data = r#"BEGIN:VCARD
VERSION:3.0
FN:Carol Williams
N:Williams;Carol;;;
EMAIL;TYPE=WORK:carol@company.com
TEL;TYPE=WORK:+1-555-987-6543
ORG:Company Inc
TITLE:Product Manager
ADR;TYPE=WORK:;;123 Business Ave;San Francisco;CA;94105;USA
END:VCARD"#;
    
    let contact = address_book_store.create_contact(&address_book.id, vcard_data).await?;
    println!("Created contact: {}", contact.uid);
    
    // List all contacts
    let contacts = address_book_store.list_contacts(&address_book.id).await;
    println!("Total contacts: {}", contacts.len());
    
    for contact in &contacts {
        println!("  - {} ({})", contact.vcard_data.lines().find(|l| l.starts_with("FN:")).unwrap_or(&"Unknown"), contact.uid);
    }
    
    Ok(())
}
```

## Advanced Integration

### Authentication Middleware

```rust
use axum::{
    routing::get,
    Router,
    middleware,
    extract::Request,
    response::Response,
};
use ferro_auth::simple_auth::simple_auth_middleware;
use ferro_auth::users::InMemoryUserStore;
use std::sync::Arc;

async fn protected_handler() -> &'static str {
    "This is a protected endpoint"
}

async fn public_handler() -> &'static str {
    "This is a public endpoint"
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let user_store = Arc::new(InMemoryUserStore::new());
    let admin_user = Some("admin".to_string());
    let admin_password = Some("secret".to_string());
    
    let app = Router::new()
        .route("/api/public", get(public_handler))
        .route("/api/protected", get(protected_handler))
        .layer(middleware::from_fn({
            let user_store = user_store.clone();
            move |req: Request, next: middleware::Next| {
                let admin_user = admin_user.clone();
                let admin_password = admin_password.clone();
                let user_store = user_store.clone();
                async move {
                    simple_auth_middleware(req, admin_user, admin_password, user_store, next).await
                }
            }
        }));
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}
```

### Custom Calendar Store Implementation

```rust
use async_trait::async_trait;
use ferro_dav::store::{
    CalendarStore, CalendarInfo, EventInfo, CalFilter, StoreResult, StoreError,
};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

pub struct PersistentCalendarStore {
    calendars: DashMap<String, CalendarInfo>,
    events: DashMap<String, EventInfo>,
}

impl PersistentCalendarStore {
    pub fn new() -> Self {
        Self {
            calendars: DashMap::new(),
            events: DashMap::new(),
        }
    }
}

#[async_trait]
impl CalendarStore for PersistentCalendarStore {
    async fn list_calendars(&self, principal: &str) -> Vec<CalendarInfo> {
        self.calendars
            .iter()
            .filter(|c| c.principal == principal)
            .map(|c| c.value().clone())
            .collect()
    }
    
    async fn get_calendar(&self, principal: &str, calendar_id: &str) -> Option<CalendarInfo> {
        self.calendars
            .get(calendar_id)
            .filter(|c| c.principal == principal)
            .map(|c| c.value().clone())
    }
    
    async fn create_calendar(
        &self,
        principal: &str,
        name: &str,
        color: &str,
    ) -> StoreResult<CalendarInfo> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        
        let calendar = CalendarInfo {
            id: id.clone(),
            principal: principal.to_string(),
            name: name.to_string(),
            color: color.to_string(),
            ctag: format!("{}", now.timestamp()),
            created_at: now,
            updated_at: now,
        };
        
        self.calendars.insert(id, calendar.clone());
        Ok(calendar)
    }
    
    async fn delete_calendar(&self, _principal: &str, calendar_id: &str) -> StoreResult<()> {
        self.calendars
            .remove(calendar_id)
            .ok_or_else(|| StoreError("Calendar not found".to_string()))?;
        Ok(())
    }
    
    async fn list_events(&self, calendar_id: &str) -> Vec<EventInfo> {
        self.events
            .iter()
            .filter(|e| e.calendar_id == calendar_id)
            .map(|e| e.value().clone())
            .collect()
    }
    
    async fn get_event(&self, calendar_id: &str, event_uid: &str) -> Option<EventInfo> {
        self.events
            .get(event_uid)
            .filter(|e| e.calendar_id == calendar_id)
            .map(|e| e.value().clone())
    }
    
    async fn create_event(&self, calendar_id: &str, ical: &str) -> StoreResult<EventInfo> {
        // Parse the iCal data to extract UID
        let uid = extract_uid_from_ical(ical)
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        
        let now = Utc::now();
        let event = EventInfo {
            uid: uid.clone(),
            calendar_id: calendar_id.to_string(),
            ical_data: ical.to_string(),
            etag: format!("{}", now.timestamp()),
            created_at: now,
            updated_at: now,
        };
        
        self.events.insert(uid, event.clone());
        Ok(event)
    }
    
    async fn update_event(
        &self,
        calendar_id: &str,
        event_uid: &str,
        ical: &str,
    ) -> StoreResult<EventInfo> {
        let mut event = self
            .events
            .get_mut(event_uid)
            .ok_or_else(|| StoreError("Event not found".to_string()))?;
        
        if event.calendar_id != calendar_id {
            return Err(StoreError("Event not in specified calendar".to_string()));
        }
        
        event.ical_data = ical.to_string();
        event.updated_at = Utc::now();
        event.etag = format!("{}", event.updated_at.timestamp());
        
        Ok(event.value().clone())
    }
    
    async fn delete_event(&self, _calendar_id: &str, event_uid: &str) -> StoreResult<()> {
        self.events
            .remove(event_uid)
            .ok_or_else(|| StoreError("Event not found".to_string()))?;
        Ok(())
    }
    
    async fn query_events(&self, calendar_id: &str, filter: &CalFilter) -> Vec<EventInfo> {
        self.list_events(calendar_id)
            .await
            .into_iter()
            .filter(|event| {
                // Simple filtering logic - in production, parse iCal data
                true
            })
            .collect()
    }
}

fn extract_uid_from_ical(ical: &str) -> Option<String> {
    ical.lines()
        .find(|line| line.starts_with("UID:"))
        .map(|line| line[4..].to_string())
}
```

## Web Integration

### REST API Client (JavaScript)

```javascript
const axios = require('axios');

const client = axios.create({
  baseURL: 'http://localhost:8080',
  headers: {
    'Authorization': 'Basic ' + Buffer.from('admin:secret').toString('base64'),
    'Content-Type': 'application/json',
  },
});

// Get server config
const config = await client.get('/api/config');
console.log('Server config:', config.data);

// Get user info
const userInfo = await client.get('/api/auth/info');
console.log('User info:', userInfo.data);

// Search files
const searchResults = await client.get('/api/search', {
  params: { q: 'document', limit: 10 },
});
console.log('Search results:', searchResults.data);

// Get storage stats
const stats = await client.get('/api/storage/stats');
console.log('Storage stats:', stats.data);
```

### WebDAV Client (JavaScript)

```javascript
const { createClient } = require('webdav');

const client = createClient(
  'http://localhost:8080',
  {
    username: 'admin',
    password: 'secret',
  }
);

// List files
const files = await client.getDirectoryContents('/');
console.log('Files:', files);

// Upload file
await client.putFileContents('/test.txt', 'Hello, world!');

// Download file
const content = await client.getFileContents('/test.txt');
console.log('Content:', content.toString());

// Create directory
await client.createDirectory('/new-folder');

// Delete file
await client.deleteFile('/test.txt');

// Check if file exists
const exists = await client.exists('/test.txt');
console.log('File exists:', exists);
```

### CalDAV Client (Python)

```python
import requests
from datetime import datetime, timedelta

# Configuration
BASE_URL = "http://localhost:8080"
AUTH = ("admin", "secret")

# List calendars
response = requests.get(
    f"{BASE_URL}/dav/calendars/",
    auth=AUTH,
    headers={"Depth": "1"},
    content_type="application/xml"
)
print("Calendars:", response.text)

# Create a calendar event
event_data = """BEGIN:VCALENDAR
BEGIN:VEVENT
UID:python-event@example.com
DTSTART:{start}
DTEND:{end}
SUMMARY:Python Meeting
DESCRIPTION:Meeting created via Python
END:VEVENT
END:VCALENDAR""".format(
    start=datetime.now().strftime("%Y%m%dT%H%M%SZ"),
    end=(datetime.now() + timedelta(hours=1)).strftime("%Y%m%dT%H%M%SZ")
)

response = requests.put(
    f"{BASE_URL}/dav/calendars/default/events/python-event.ics",
    data=event_data,
    auth=AUTH,
    content_type="text/calendar"
)
print("Create event status:", response.status_code)
```

## Testing Integration

### Unit Test Example

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use ferro_auth::users::{InMemoryUserStore, User, UserRole, UserStatus, UserStoreTrait, hash_password, ZeroizeString};
    use ferro_dav::store::{CalendarStore, InMemoryCalendarStore};

    #[tokio::test]
    async fn test_calendar_crud() {
        let store = InMemoryCalendarStore::new();
        
        // Create
        let calendar = store.create_calendar("user:test", "Test", "#ff0000").await.unwrap();
        assert_eq!(calendar.name, "Test");
        
        // Read
        let fetched = store.get_calendar("user:test", &calendar.id).await;
        assert!(fetched.is_some());
        
        // List
        let calendars = store.list_calendars("user:test").await;
        assert_eq!(calendars.len(), 1);
        
        // Delete
        store.delete_calendar("user:test", &calendar.id).await.unwrap();
        let deleted = store.get_calendar("user:test", &calendar.id).await;
        assert!(deleted.is_none());
    }

    #[tokio::test]
    async fn test_user_authentication() {
        let store = InMemoryUserStore::new();
        
        let user = User {
            id: uuid::Uuid::new_v4().to_string(),
            username: "testuser".to_string(),
            display_name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            role: UserRole::User,
            created_at: chrono::Utc::now(),
            last_login: None,
            status: UserStatus::Active,
            storage_quota_bytes: None,
            storage_used_bytes: 0,
            is_ldap: false,
            password_hash: Some(ZeroizeString::new(hash_password("pass123").unwrap())),
            totp_secret: None,
            totp_enabled: false,
        };
        
        store.create_user(user).await.unwrap();
        
        // Successful authentication
        let result = store.authenticate("testuser", "pass123").await;
        assert!(result.is_ok());
        
        // Failed authentication
        let result = store.authenticate("testuser", "wrongpass").await;
        assert!(result.is_err());
    }
}
```

### Integration Test Example

```rust
use reqwest;
use serde_json::json;

#[tokio::test]
async fn test_api_endpoints() {
    let client = reqwest::Client::new();
    let base_url = "http://localhost:8080";
    
    // Health check
    let resp = client.get(format!("{}/.well-known/ferro", base_url))
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());
    
    // Config endpoint
    let resp = client.get(format!("{}/api/config", base_url))
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());
    
    // Auth info
    let resp = client.get(format!("{}/api/auth/info", base_url))
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());
}
```
