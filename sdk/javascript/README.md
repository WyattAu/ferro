# Ferro JavaScript SDK

## Installation

```bash
npm install @ferro/sdk
```

## Quick Start

```typescript
import { FerroClient } from '@ferro/sdk';

const client = new FerroClient({
  host: 'localhost',
  port: 8080,
  username: 'admin',
  password: 'password'
});

// List calendars
const calendars = await client.calendars.list();
for (const calendar of calendars) {
  console.log(calendar.name);
}

// Create event
const event = await client.calendars.createEvent('default', {
  summary: 'Team Meeting',
  start: '2024-01-01T10:00:00Z',
  end: '2024-01-01T11:00:00Z'
});

// List contacts
const contacts = await client.contacts.list();
for (const contact of contacts) {
  console.log(contact.name);
}
```

## API Reference

### Client

```typescript
class FerroClient {
  constructor(options: {
    host: string;
    port: number;
    username: string;
    password: string;
  });
  
  calendars: CalendarManager;
  contacts: ContactManager;
}
```

### CalendarManager

```typescript
class CalendarManager {
  list(): Promise<Calendar[]>;
  get(calendarId: string): Promise<Calendar>;
  create(name: string): Promise<Calendar>;
  delete(calendarId: string): Promise<void>;
  createEvent(calendarId: string, event: CreateEventRequest): Promise<Event>;
  listEvents(calendarId: string): Promise<Event[]>;
}
```

### ContactManager

```typescript
class ContactManager {
  list(): Promise<Contact[]>;
  get(contactId: string): Promise<Contact>;
  create(name: string): Promise<Contact>;
  delete(contactId: string): Promise<void>;
}
```
