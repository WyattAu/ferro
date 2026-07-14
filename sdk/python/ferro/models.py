from dataclasses import dataclass
from typing import Optional

@dataclass
class Calendar:
    id: str
    name: str
    description: Optional[str] = None
    
    @classmethod
    def from_dict(cls, data: dict) -> "Calendar":
        return cls(
            id=data["id"],
            name=data["name"],
            description=data.get("description"),
        )

@dataclass
class Event:
    id: str
    calendar_id: str
    summary: str
    start: str
    end: str
    description: Optional[str] = None
    
    @classmethod
    def from_dict(cls, data: dict) -> "Event":
        return cls(
            id=data["id"],
            calendar_id=data["calendar_id"],
            summary=data["summary"],
            start=data["start"],
            end=data["end"],
            description=data.get("description"),
        )

@dataclass
class Contact:
    id: str
    name: str
    email: Optional[str] = None
    phone: Optional[str] = None
    
    @classmethod
    def from_dict(cls, data: dict) -> "Contact":
        return cls(
            id=data["id"],
            name=data["name"],
            email=data.get("email"),
            phone=data.get("phone"),
        )
