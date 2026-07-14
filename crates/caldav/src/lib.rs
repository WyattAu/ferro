pub mod calendar;
pub mod error;
pub mod event;
pub mod handler;
pub mod ical;
pub mod report;

pub use calendar::{Calendar, CalendarItem, CalendarManager};
pub use error::{CalDavError, Result};
pub use event::{EventWithParsed, event_in_time_range, events_overlap};
pub use handler::{
    CalDavState, handle_delete, handle_get, handle_mkcalendar, handle_options, handle_propfind, handle_put,
    handle_report,
};
pub use ical::{CalendarEvent, EventStatus, extract_event_from_ical, generate_ical_event, parse_ical_datetime};
pub use report::{ReportType, parse_report};

pub fn create_calstate(store: ferro_dav::store::DynCalendarStore, principal: String) -> CalDavState {
    CalDavState {
        manager: CalendarManager::new(store),
        principal,
    }
}
