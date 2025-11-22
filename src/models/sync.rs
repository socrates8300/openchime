#![allow(dead_code)]
// file: src/sync.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub account_id: i64,
    pub success: bool,
    pub events_added: usize,
    pub events_updated: usize,
    pub error_message: Option<String>,
    pub sync_time: DateTime<Utc>,
}

impl SyncResult {
    pub fn success(account_id: i64) -> Self {
        Self {
            account_id,
            success: true,
            events_added: 0,
            events_updated: 0,
            error_message: None,
            sync_time: Utc::now(),
        }
    }

    pub fn with_counts(account_id: i64, added: usize, updated: usize) -> Self {
        Self {
            account_id,
            success: true,
            events_added: added,
            events_updated: updated,
            error_message: None,
            sync_time: Utc::now(),
        }
    }

    pub fn with_error(account_id: i64, error: String) -> Self {
        Self {
            account_id,
            success: false,
            events_added: 0,
            events_updated: 0,
            error_message: Some(error),
            sync_time: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_result_success() {
        let result = SyncResult::success(1);
        assert!(result.success);
        assert_eq!(result.account_id, 1);
        assert_eq!(result.events_added, 0);
        assert_eq!(result.events_updated, 0);
        assert!(result.error_message.is_none());
    }

    #[test]
    fn test_sync_result_with_counts() {
        let result = SyncResult::with_counts(1, 5, 3);
        assert!(result.success);
        assert_eq!(result.account_id, 1);
        assert_eq!(result.events_added, 5);
        assert_eq!(result.events_updated, 3);
    }

    #[test]
    fn test_sync_result_with_error() {
        let result = SyncResult::with_error(1, "Network error".to_string());
        assert!(!result.success);
        assert_eq!(result.account_id, 1);
        assert_eq!(result.error_message, Some("Network error".to_string()));
    }
}
