# Shared Calendars

## Overview

Shared calendars allow multiple users to view and edit the same calendar.

## Features

### Permissions
- **Owner:** Full control (create, read, update, delete, share)
- **Editor:** Can create, read, update, delete events
- **Viewer:** Can only read events

### Sharing
- Share by email address
- Share by link (public/private)
- Revoke access

## API Endpoints

### Share Calendar
```http
POST /dav/calendars/{calendar_id}/share
Content-Type: application/json

{
  "email": "user@example.com",
  "permission": "editor"
}
```

### Revoke Access
```http
DELETE /dav/calendars/{calendar_id}/share/{user_id}
```

### List Shared Users
```http
GET /dav/calendars/{calendar_id}/share
```

## Implementation

### Database Schema
```sql
CREATE TABLE calendar_shares (
    id TEXT PRIMARY KEY,
    calendar_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    permission TEXT NOT NULL,
    created_at DATETIME NOT NULL,
    FOREIGN KEY (calendar_id) REFERENCES calendars(id),
    FOREIGN KEY (user_id) REFERENCES users(id)
);
```

### Rust Types
```rust
pub enum SharePermission {
    Owner,
    Editor,
    Viewer,
}

pub struct CalendarShare {
    pub id: String,
    pub calendar_id: String,
    pub user_id: String,
    pub permission: SharePermission,
    pub created_at: DateTime<Utc>,
}
```
