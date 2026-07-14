import requests
from typing import List, Optional
from .models import Calendar, Event, Contact

class FerroClient:
    def __init__(self, host: str, port: int, username: str, password: str):
        self.base_url = f"http://{host}:{port}"
        self.auth = (username, password)
        self.session = requests.Session()
        self.session.auth = self.auth
    
    def calendars(self) -> "CalendarManager":
        return CalendarManager(self)
    
    def contacts(self) -> "ContactManager":
        return ContactManager(self)
    
    def _get(self, path: str) -> dict:
        response = self.session.get(f"{self.base_url}{path}")
        response.raise_for_status()
        return response.json()
    
    def _post(self, path: str, data: dict) -> dict:
        response = self.session.post(f"{self.base_url}{path}", json=data)
        response.raise_for_status()
        return response.json()
    
    def _delete(self, path: str) -> None:
        response = self.session.delete(f"{self.base_url}{path}")
        response.raise_for_status()


class CalendarManager:
    def __init__(self, client: FerroClient):
        self.client = client
    
    def list(self) -> List[Calendar]:
        data = self.client._get("/dav/calendars")
        return [Calendar.from_dict(c) for c in data["calendars"]]
    
    def get(self, calendar_id: str) -> Calendar:
        data = self.client._get(f"/dav/calendars/{calendar_id}")
        return Calendar.from_dict(data)
    
    def create(self, name: str) -> Calendar:
        data = self.client._post("/dav/calendars", {"name": name})
        return Calendar.from_dict(data)
    
    def delete(self, calendar_id: str) -> None:
        self.client._delete(f"/dav/calendars/{calendar_id}")
    
    def create_event(self, calendar_id: str, summary: str, start: str, end: str) -> Event:
        data = self.client._post(
            f"/dav/calendars/{calendar_id}/events",
            {"summary": summary, "start": start, "end": end}
        )
        return Event.from_dict(data)
    
    def list_events(self, calendar_id: str) -> List[Event]:
        data = self.client._get(f"/dav/calendars/{calendar_id}/events")
        return [Event.from_dict(e) for e in data["events"]]


class ContactManager:
    def __init__(self, client: FerroClient):
        self.client = client
    
    def list(self) -> List[Contact]:
        data = self.client._get("/dav/contacts")
        return [Contact.from_dict(c) for c in data["contacts"]]
    
    def get(self, contact_id: str) -> Contact:
        data = self.client._get(f"/dav/contacts/{contact_id}")
        return Contact.from_dict(data)
    
    def create(self, name: str) -> Contact:
        data = self.client._post("/dav/contacts", {"name": name})
        return Contact.from_dict(data)
    
    def delete(self, contact_id: str) -> None:
        self.client._delete(f"/dav/contacts/{contact_id}")
