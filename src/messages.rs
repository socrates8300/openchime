use crate::models::{Account, CalendarEvent, Settings};
use crate::alerts::MonitorEvent;

/// Unified application message type
/// 
/// This enum handles all message types throughout the application.
/// Messages are organized by domain for better maintainability.
#[derive(Debug, Clone)]
pub enum Message {
    // ===== UI Navigation Messages =====
    /// Switch to calendar view
    ShowCalendar,
    /// Switch to settings view
    ShowSettings,
    /// Switch to alerts view
    ShowAlerts,
    
    // ===== UI Action Messages =====
    /// Toggle theme (Light/Dark)
    ToggleTheme(bool),
    /// Open a URL in the default browser
    OpenUrl(String),
    /// Join a meeting URL
    JoinMeeting(String),
    /// Play a test sound
    TestAudio,
    /// Stop any playing sound
    StopSound,
    /// Snooze an alert
    SnoozeAlert(i64), // event_id
    /// Dismiss an alert
    DismissAlert(i64), // event_id
    
    // ===== Form Input Messages =====
    /// Update account name input field
    AccountNameChanged(String),
    /// Update ICS URL input field
    IcsUrlChanged(String),
    /// Update auth data input field (token or URL) - kept for compatibility if needed
    AuthDataChanged(String),
    /// Update refresh token input field
    RefreshTokenChanged(String),
    /// Update alert timing preference (30m)
    ToggleAlert30m(bool),
    /// Update alert timing preference (10m)
    ToggleAlert10m(bool),
    /// Update alert timing preference (5m)
    ToggleAlert5m(bool),
    /// Update alert timing preference (1m)
    ToggleAlert1m(bool),
    /// Update alert timing preference (At start)
    ToggleAlertDefault(bool),
    
    // ===== Account Management Messages =====
    /// Request to add a new Proton/ICS account
    AddProtonAccount,
    /// Request to delete an account
    DeleteAccount(i64),
    /// Request to sync an account manually
    SyncAccount(i64),
    /// Request to sync all accounts
    SyncCalendars,
    
    // ===== Async Operation Results =====
    /// Account addition completed
    AccountAdded(Result<Account, String>),
    /// Account deletion completed
    AccountDeleted(Result<(), String>),
    /// Calendar sync completed
    CalendarSyncResult(Result<(), String>),
    /// Audio test completed
    AudioTestResult(Result<(), String>),
    
    // ===== Data Update Messages =====
    /// Events data has been updated
    EventsUpdated(Vec<CalendarEvent>),
    /// Settings data has been updated
    SettingsUpdated(Settings),
    /// Initial data loading completed
    DataLoaded(Vec<CalendarEvent>, Vec<Account>),
    
    // ===== Monitor System Messages =====
    /// Background monitor event received
    MonitorEventReceived(MonitorEvent),
}
