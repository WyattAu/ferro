# CalDAV Scheduling

## Overview

CalDAV Scheduling (RFC 6638) enables automatic scheduling of events between users.

## Features

### Free/Busy Query
- Query free/busy time for users
- Find available time slots
- Suggest meeting times

### Attendee Management
- Add attendees to events
- Track RSVP status (accepted, declined, tentative)
- Send invitations

### Calendar Availability
- Publish free/busy information
- Control availability visibility

## API Endpoints

### Free/Busy Query
```http
POST /dav/freebusy
Content-Type: application/json

{
  "start": "2024-01-01T00:00:00Z",
  "end": "2024-01-31T23:59:59Z",
  "users": ["user1@example.com", "user2@example.com"]
}
```

### Update Attendee Status
```http
PUT /dav/calendars/{calendar_id}/events/{event_id}/attendees/{attendee_id}
Content-Type: application/json

{
  "status": "accepted"
}
```

## Implementation

### Database Schema
```sql
CREATE TABLE event_attendees (
    id TEXT PRIMARY KEY,
    event_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    status TEXT NOT NULL,
    role TEXT NOT NULL,
    created_at DATETIME NOT NULL,
    FOREIGN KEY (event_id) REFERENCES events(id),
    FOREIGN KEY (user_id) REFERENCES users(id)
);
```

### Rust Types
```rust
pub enum AttendeeStatus {
    Accepted,
    Declined,
    Tentative,
    NeedsAction,
}

pub enum AttendeeRole {
    Chair,
    Required,
    Optional,
    NonParticipant,
}

pub struct EventAttendee {
    pub id: String,
    pub event_id: String,
    pub user_id: String,
    pub status: AttendeeStatus,
    pub role: AttendeeRole,
    pub created_at: DateTime<Utc>,
}
```
