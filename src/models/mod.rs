#![allow(unused_imports)]
// file: src/lib.rs (or the original filename)

// Declare modules
pub mod account;
pub mod alert;
pub mod event;
pub mod meeting;
pub mod settings;
pub mod sync;

// Re-export all public types to ensure no breaking changes for external callers.
// This flattens the structure so imports like `use crate::CalendarEvent` still work.
pub use account::{Account, CalendarProvider};
pub use alert::{AlertInfo, AlertType};
pub use event::CalendarEvent;
pub use meeting::VideoMeetingInfo;
pub use settings::{Setting, Settings};
pub use sync::SyncResult;
