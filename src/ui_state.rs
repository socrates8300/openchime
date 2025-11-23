//! UI state management module
//! 
//! This module manages all UI-specific state and provides clean separation
//! between application logic and presentation layer.

/// UI view states
#[derive(Debug, Clone, PartialEq)]
pub enum View {
    Calendar,
    Settings,
    Alerts,
}

/// Application UI state
/// 
/// This struct encapsulates all UI-related state that doesn't belong
/// in the core application logic.
/// 
/// Note: Fields are public for now to allow gradual refactoring.
/// Consider making them private in future iterations.
#[derive(Debug, Clone)]
pub struct UiState {
    /// Current active view
    pub current_view: View,
    
    /// Account name input field
    pub account_name: String,
    
    /// ICS URL input field
    pub ics_url: String,
    
    /// Current sync status message
    pub sync_status: String,
    
    /// Whether an async operation is in progress
    pub loading: bool,
    
    /// Timestamp of last successful sync
    pub last_sync_time: Option<chrono::DateTime<chrono::Utc>>,
}

impl UiState {
    /// Create new UI state with default values
    pub fn new() -> Self {
        Self {
            current_view: View::Calendar,
            account_name: String::new(),
            ics_url: String::new(),
            sync_status: "Ready".to_string(),
            loading: false,
            last_sync_time: None,
        }
    }
}
