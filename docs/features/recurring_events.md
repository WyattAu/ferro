# Recurring Events

## Overview

Recurring events repeat according to a pattern (daily, weekly, monthly, yearly).

## RFC 5545 Recurrence Rules

### Frequency
- `DAILY`
- `WEEKLY`
- `MONTHLY`
- `YEARLY`

### Interval
- `INTERVAL=1` - Every frequency unit
- `INTERVAL=2` - Every 2 frequency units
- etc.

### Count/Limit
- `COUNT=10` - Repeat 10 times
- `UNTIL=20241231T235959Z` - Repeat until date

### By Day
- `BYDAY=MO,WE,FR` - Monday, Wednesday, Friday
- `BYDAY=1SU` - First Sunday
- `BYDAY=-1FR` - Last Friday

## Examples

### Daily Event
```
RRULE:FREQ=DAILY;COUNT=10
```

### Weekly Event
```
RRULE:FREQ=WEEKLY;BYDAY=MO,WE,FR
```

### Monthly Event
```
RRULE:FREQ=MONTHLY;BYDAY=1SU
```

### Yearly Event
```
RRULE:FREQ=YEARLY;BYMONTH=12;BYMONTHDAY=25
```

## API Endpoints

### Create Recurring Event
```http
POST /dav/calendars/{calendar_id}/events
Content-Type: application/json

{
  "summary": "Team Meeting",
  "start": "2024-01-01T10:00:00Z",
  "end": "2024-01-01T11:00:00Z",
  "recurrence": "FREQ=WEEKLY;BYDAY=MO,WE,FR"
}
```

### Get Recurring Event Instances
```http
GET /dav/calendars/{calendar_id}/events/{event_id}/instances?start=2024-01-01&end=2024-12-31
```

## Implementation

### Rust Types
```rust
pub struct RecurrenceRule {
    pub frequency: Frequency,
    pub interval: u32,
    pub count: Option<u32>,
    pub until: Option<DateTime<Utc>>,
    pub by_day: Vec<DayOfWeek>,
    pub by_month: Vec<u32>,
    pub by_month_day: Vec<u32>,
}

pub enum Frequency {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

pub enum DayOfWeek {
    Sunday,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
}
```
