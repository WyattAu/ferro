# ferro-dav

[![crates.io](https://img.shields.io/crates/v/ferro-dav.svg)](https://crates.io/crates/ferro-dav)
[![docs.rs](https://docs.rs/ferro-dav/badge.svg)](https://docs.rs/ferro-dav)
[![license](https://img.shields.io/badge/license-AGPL-3.0-blue.svg)](LICENSE)

CalDAV and CardDAV protocol implementations for the Ferro platform. Provides iCalendar (RFC 5545) and vCard (RFC 6350) parsers, store traits for calendar and address book data, and ready-to-use Axum handlers for CalDAV and CardDAV endpoints.

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `handlers` | yes | Axum handler modules for CalDAV and CardDAV HTTP endpoints |
| `persistence` | no | SQLite persistence for in-memory stores |

## Key Types

### Parsers

- **`parse_ical`** / **`serialize_ical`** — RFC 5545 iCalendar parser and serializer
- **`parse_vcard`** / **`serialize_vcard`** — RFC 6350 vCard parser and serializer
- **`IcalComponent`**, **`IcalProperty`** — structured iCalendar representation
- **`Vcard`**, **`VcardValue`**, **`VcardAddress`** — structured vCard representation

### Store Traits

- **`CalendarStore`** — trait for calendar CRUD and time-range event queries
- **`AddressBookStore`** — trait for address book CRUD and contact management
- **`InMemoryCalendarStore`** — thread-safe in-memory calendar store (with optional SQLite persistence)
- **`InMemoryAddressBookStore`** — thread-safe in-memory address book store (with optional SQLite persistence)
- **`DynCalendarStore`** / **`DynAddressBookStore`** — type-erased `Arc<dyn ...>` aliases

### Handlers (requires `handlers` feature)

- **`CalDavState`** / **`caldav::*`** — CalDAV Axum router handlers (OPTIONS, PROPFIND, REPORT, MKCALENDAR, GET, PUT, DELETE)
- **`CardDavState`** / **`carddav::*`** — CardDAV Axum router handlers (OPTIONS, PROPFIND, REPORT, GET, PUT, DELETE)

### XML Utilities

- **`build_dav_multistatus`** — build WebDAV multistatus XML responses
- **`parse_calendar_query_time_range`** — extract time-range filters from calendar-query REPORT
- **`parse_addressbook_query_filter`** — extract filters from addressbook-query REPORT

## Usage

### Parse iCalendar data

```rust
use ferro_dav::ical::{parse_ical, get_first_prop};

let ical = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:mtg-1\r\nSUMMARY:Meeting\r\nDTSTART:20240101T100000Z\r\nDTEND:20240101T110000Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";

let components = parse_ical(ical)?;
let vcal = &components[0];
let vevent = &vcal.children[0];
let summary = get_first_prop(vevent, "SUMMARY").unwrap();
println!("Event: {}", summary.value);
```

### Parse vCard data

```rust
use ferro_dav::vcard::{parse_vcard, serialize_vcard};

let vcard = parse_vcard(
    "BEGIN:VCARD\r\nVERSION:3.0\r\nFN:Jane Doe\r\nUID:1\r\nEND:VCARD\r\n"
)?;
println!("Name: {}", vcard.fn_name);
let roundtrip = serialize_vcard(&vcard);
```

### Build a CalDAV server

```rust
use ferro_dav::store::{InMemoryCalendarStore, DynCalendarStore};
use ferro_dav::caldav::{CalDavState, self};
use std::sync::Arc;

let store: DynCalendarStore = Arc::new(InMemoryCalendarStore::new());
let state = CalDavState {
    store,
    principal: "user1".into(),
};

let app = axum::Router::new()
    .route("/.well-known/caldav", axum::routing::get(caldav::options_handler))
    .route("/dav/calendars", axum::routing::get(caldav::propfind_calendars))
    .with_state(state);
```

## Examples

### Persistent calendar store

```toml
# Cargo.toml
ferro-dav = { version = "0.1", features = ["persistence"] }
```

```rust
use ferro_dav::store::{InMemoryCalendarStore, DbHandle};
use std::sync::{Arc, Mutex};
use rusqlite::Connection;

let conn = Connection::open("calendars.db")?;
let db: DbHandle = Arc::new(Mutex::new(conn));
let store = InMemoryCalendarStore::with_db(db);
```

## License

Licensed under AGPL-3.0-or-later.
