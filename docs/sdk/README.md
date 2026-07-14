# Ferro SDK Documentation

## Overview

The Ferro SDK provides a comprehensive API for interacting with the Ferro storage platform. Built entirely in Rust, it offers WebDAV-compatible file access, CalDAV/CardDAV protocol support, and authentication management.

## Quick Start

### Installation

Add the following dependencies to your `Cargo.toml`:

```toml
[dependencies]
ferro-common = { path = "../crates/common" }
ferro-auth = { path = "../crates/auth" }
ferro-dav = { path = "../crates/dav" }
ferro-core = { path = "../crates/core" }
```

### Basic Usage

```rust
use common::auth::Claims;
use ferro_auth::users::{InMemoryUserStore, User, UserRole, UserStoreTrait};
use ferro_dav::store::{CalendarStore, AddressBookStore, InMemoryCalendarStore, InMemoryAddressBookStore};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize user store
    let user_store = InMemoryUserStore::new();
    
    // Create a user
    let user = User {
        id: uuid::Uuid::new_v4().to_string(),
        username: "admin".to_string(),
        display_name: "Administrator".to_string(),
        email: "admin@example.com".to_string(),
        role: UserRole::Admin,
        created_at: chrono::Utc::now(),
        last_login: None,
        status: ferro_auth::users::UserStatus::Active,
        storage_quota_bytes: None,
        storage_used_bytes: 0,
        is_ldap: false,
        password_hash: Some(ferro_auth::users::ZeroizeString::new(
            ferro_auth::users::hash_password("secure_password")?
        )),
        totp_secret: None,
        totp_enabled: false,
    };
    
    user_store.create_user(user).await?;
    
    // Initialize calendar store
    let calendar_store = InMemoryCalendarStore::new();
    
    // Create a calendar
    let calendar = calendar_store.create_calendar(
        "user:admin",
        "My Calendar",
        "#4285f4",
    ).await?;
    
    println!("Created calendar: {}", calendar.id);
    
    Ok(())
}
```

## API Reference

### Authentication

#### User Management (`ferro-auth::users`)

- `User` - User account structure
- `UserRole` - User role (Admin, User, ReadOnly)
- `UserStatus` - Account status (Active, Disabled, Locked)
- `UserInfo` - Lightweight user identity for requests
- `InMemoryUserStore` - In-memory user storage with optional SQLite persistence
- `UserStoreTrait` - Async interface for user persistence

#### OIDC Authentication (`ferro-auth::oidc`)

- `OidcConfig` - OIDC provider configuration
- `OidcValidator` - JWT token validation and PKCE session management
- `PkceSession` - PKCE OAuth session data

#### Simple Auth (`ferro-auth::simple_auth`)

- `simple_auth_middleware` - HTTP Basic authentication middleware
- `simple_auth_middleware_with_api_keys` - Extended middleware with API key support

### DAV Operations

#### Calendar Store (`ferro-dav::store`)

- `CalendarInfo` - Calendar collection metadata
- `EventInfo` - Calendar event data
- `CalFilter` - Time-range filter for event queries
- `CalendarStore` - Async trait for calendar storage backends
- `InMemoryCalendarStore` - In-memory calendar storage

#### Address Book Store (`ferro-dav::store`)

- `AddressBookInfo` - Address book collection metadata
- `ContactInfo` - Contact (vCard) data
- `AddressBookStore` - Async trait for address book storage backends
- `InMemoryAddressBookStore` - In-memory address book storage

#### iCalendar (`ferro-dav::ical`)

- `IcalComponent` - Parsed iCalendar component
- `IcalProperty` - iCalendar property
- `parse_ical()` - Parse RFC 5545 iCalendar string
- `serialize_ical()` - Serialize components to iCalendar string
- `get_first_prop()` / `get_all_props()` - Property accessors

#### vCard (`ferro-dav::vcard`)

- `Vcard` - Parsed vCard contact
- `VcardProperty` - vCard property
- `VcardValue` - Typed value with TYPE parameters
- `VcardAddress` - Structured postal address
- `parse_vcard()` - Parse RFC 6350 vCard string
- `serialize_vcard()` - Serialize contact to vCard string

### Common Types (`ferro-common`)

- `Claims` - JWT claims structure
- `FerroError` - Application error type
- `ZeroizeString` - Secure string with zeroize on drop

## Examples

### Calendar Operations

```rust
use ferro_dav::store::{CalendarStore, InMemoryCalendarStore};
use ferro_dav::ical::{parse_ical, serialize_ical, IcalComponent};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = InMemoryCalendarStore::new();
    
    // Create a calendar
    let calendar = store.create_calendar(
        "user:admin",
        "Work Calendar",
        "#039be5",
    ).await?;
    
    // Create an event
    let ical_data = r#"BEGIN:VCALENDAR
BEGIN:VEVENT
UID:meeting-001@example.com
DTSTART:20240101T100000Z
DTEND:20240101T110000Z
SUMMARY:Team Meeting
DESCRIPTION:Weekly team sync
END:VEVENT
END:VCALENDAR"#;
    
    let event = store.create_event(&calendar.id, ical_data).await?;
    println!("Created event: {}", event.uid);
    
    // List events
    let events = store.list_events(&calendar.id).await;
    println!("Total events: {}", events.len());
    
    // Query events by time range
    use ferro_dav::store::CalFilter;
    use chrono::{Utc, Duration};
    
    let filter = CalFilter {
        start: Some(Utc::now() - Duration::days(7)),
        end: Some(Utc::now() + Duration::days(7)),
    };
    
    let filtered_events = store.query_events(&calendar.id, &filter).await;
    println!("Events in range: {}", filtered_events.len());
    
    Ok(())
}
```

### Contact Operations

```rust
use ferro_dav::store::{AddressBookStore, InMemoryAddressBookStore};
use ferro_dav::vcard::{parse_vcard, serialize_vcard, Vcard, VcardValue};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = InMemoryAddressBookStore::new();
    
    // Create an address book
    let address_book = store.create_address_book(
        "user:admin",
        "My Contacts",
    ).await?;
    
    // Create a contact
    let vcard_data = r#"BEGIN:VCARD
VERSION:3.0
FN:John Doe
N:Doe;John;;;
EMAIL;TYPE=HOME:john@example.com
TEL;TYPE=WORK:+1-555-123-4567
ORG:Acme Corp
TITLE:Software Engineer
END:VCARD"#;
    
    let contact = store.create_contact(&address_book.id, vcard_data).await?;
    println!("Created contact: {}", contact.uid);
    
    // List contacts
    let contacts = store.list_contacts(&address_book.id).await;
    println!("Total contacts: {}", contacts.len());
    
    Ok(())
}
```

### User Authentication

```rust
use ferro_auth::users::{
    InMemoryUserStore, User, UserRole, UserStatus, UserStoreTrait,
    CreateUserRequest, UpdateUserRequest, hash_password, ZeroizeString,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = InMemoryUserStore::new();
    
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
        storage_quota_bytes: Some(1024 * 1024 * 1024), // 1GB
        storage_used_bytes: 0,
        is_ldap: false,
        password_hash: Some(ZeroizeString::new(hash_password("secure_pass")?)),
        totp_secret: None,
        totp_enabled: false,
    };
    
    store.create_user(user).await?;
    
    // Authenticate
    let authenticated_user = store.authenticate("alice", "secure_pass").await?;
    println!("Authenticated: {}", authenticated_user.username);
    
    // Update user
    store.update_user(
        &authenticated_user.id,
        UpdateUserRequest {
            display_name: Some("Alice J. Smith".to_string()),
            ..Default::default()
        },
    ).await?;
    
    Ok(())
}
```

## Configuration

### Environment Variables

```bash
# Server configuration
FERRO_HOST=0.0.0.0
FERRO_PORT=8080
FERRO_LOG_LEVEL=info

# Storage backend
FERRO_STORAGE=local:/path/to/files
FERRO_DATA_DIR=/path/to/data

# Authentication
FERRO_OIDC_ISSUER=https://auth.example.com
FERRO_OIDC_CLIENT_ID=ferro-client
FERRO_ADMIN_USER=admin
FERRO_ADMIN_PASSWORD=secret
```

### Configuration File

Create a `ferro.toml` file:

```toml
host = "0.0.0.0"
port = 8080
storage = "local:/data/files"
data_dir = "/var/lib/ferro"
log_level = "info"

# OIDC configuration
oidc_issuer = "https://auth.example.com"
oidc_client_id = "ferro-client"
oidc_audience = "ferro"
```

## Troubleshooting

### Common Issues

#### Authentication Errors

- Ensure `FERRO_ADMIN_USER` and `FERRO_ADMIN_PASSWORD` are set
- Check that the user account is active (not disabled or locked)
- Verify password hash is correctly generated with `hash_password()`

#### Storage Errors

- Check storage directory permissions (read/write for the server process)
- Verify disk space availability
- Ensure the storage backend is properly configured

#### DAV Protocol Errors

- Validate iCalendar/vCard data format (RFC 5545/6350)
- Check that calendar/address book IDs exist
- Verify user permissions for the requested principal

### Debug Logging

Enable debug logging for detailed error information:

```bash
RUST_LOG=debug cargo run
```

## Contributing

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines on contributing to the Ferro project.
