//! Pure Agenda domain logic: task types, date/recurrence helpers, smart-list
//! filters and quick-entry parsing shared by the desktop GPUI app and the
//! Android port (KOS-370). No GPUI, no HTTP, no Engine transport — and no
//! hidden clock reads: every function that depends on "now" takes a
//! [`LocalDay`] from its caller.

mod dates;
mod filters;
pub mod mapping;
pub mod projects;
mod quick_entry;
mod recurrence;
pub mod seed;
mod types;

pub use dates::*;
pub use filters::*;
pub use quick_entry::*;
pub use recurrence::*;
pub use types::*;
