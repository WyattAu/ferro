pub mod calendar;
pub mod contacts;
pub mod notes;
pub mod photos;
pub mod tasks;
pub mod whiteboard;

use ferro_dav::store::{AddressBookStore, CalendarStore};
use std::sync::Arc;

pub type DbHandle = Arc<std::sync::Mutex<rusqlite::Connection>>;

pub trait ProductivityState: common::server_context::HasStorage + Clone + Send + Sync + 'static {
    fn data_dir(&self) -> Option<&str>;
    fn calendar_store(&self) -> &Arc<dyn CalendarStore>;
    fn address_book_store(&self) -> &Arc<dyn AddressBookStore>;
    fn task_store(&self) -> &tasks::TaskStore;
}
