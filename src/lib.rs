// OpenChime Library
// Exposes core functionality for testing and reuse
// ICS-only mode - OAuth and encryption removed

pub mod database;
pub mod models;
pub mod calendar;
pub mod alerts;
pub mod audio;
pub mod utils;
pub mod error;
pub mod command_handlers;
pub mod http_config;
pub mod config;
pub mod app;
pub mod messages;
pub mod ui;
pub mod ui_state;

// Re-export commonly used types
pub use models::*;
pub use database::{Database, PoolStats};
pub use audio::{AudioManager, AlertType, SoundFiles};
pub use alerts::{should_trigger_alert, get_upcoming_events, sync_calendars, MonitorEvent};
pub use error::AppError;

use std::sync::Arc;

/// Application state shared across the application
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub audio: Arc<AudioManager>,
    pub shutdown: tokio_util::sync::CancellationToken,
}