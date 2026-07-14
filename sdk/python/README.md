# Ferro Python SDK

## Installation

```bash
pip install ferro-sdk
```

## Quick Start

```python
from ferro import FerroClient

client = FerroClient(
    host="localhost",
    port=8080,
    username="admin",
    password="password"
)

# List calendars
calendars = client.calendars.list()
for calendar in calendars:
    print(calendar.name)

# Create event
event = client.calendars.create_event(
    calendar_id="default",
    summary="Team Meeting",
    start="2024-01-01T10:00:00Z",
    end="2024-01-01T11:00:00Z"
)

# List contacts
contacts = client.contacts.list()
for contact in contacts:
    print(contact.name)
```

## API Reference

### Client

```python
class FerroClient:
    def __init__(self, host: str, port: int, username: str, password: str):
        ...
    
    def calendars(self) -> CalendarManager:
        ...
    
    def contacts(self) -> ContactManager:
        ...
```

### CalendarManager

```python
class CalendarManager:
    def list(self) -> List[Calendar]:
        ...
    
    def get(self, calendar_id: str) -> Calendar:
        ...
    
    def create(self, name: str) -> Calendar:
        ...
    
    def delete(self, calendar_id: str) -> None:
        ...
    
    def create_event(self, calendar_id: str, summary: str, start: str, end: str) -> Event:
        ...
    
    def list_events(self, calendar_id: str) -> List[Event]:
        ...
```

### ContactManager

```python
class ContactManager:
    def list(self) -> List[Contact]:
        ...
    
    def get(self, contact_id: str) -> Contact:
        ...
    
    def create(self, name: str) -> Contact:
        ...
    
    def delete(self, contact_id: str) -> None:
        ...
```
