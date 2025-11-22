#![allow(dead_code)]
// file: src/alert.rs
use super::event::CalendarEvent;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertInfo {
    pub event: CalendarEvent,
    pub alert_type: AlertType,
    pub minutes_remaining: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertType {
    VideoMeeting,
    Meeting,
    SnoozeReminder,
    Test,
    Warning30m,
    Warning10m,
    Warning5m,
    Warning1m,
}

impl AlertInfo {
    pub fn new(event: CalendarEvent) -> Self {
        let minutes_remaining = event.minutes_until_start();
        let alert_type = if event.is_video_meeting() {
            AlertType::VideoMeeting
        } else {
            AlertType::Meeting
        };

        Self {
            event,
            alert_type,
            minutes_remaining,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    #[test]
    fn test_alert_info_new() {
        let now = Utc::now();
        let video_event = CalendarEvent {
            id: None,
            external_id: "test-5".to_string(),
            account_id: 1,
            title: "Video Call".to_string(),
            description: None,
            start_time: now + Duration::minutes(5),
            end_time: now + Duration::hours(1),
            video_link: Some("https://meet.google.com/abc-def".to_string()),
            created_at: now,
            updated_at: now,
        };

        let alert_info = AlertInfo::new(video_event.clone());
        assert!(matches!(alert_info.alert_type, AlertType::VideoMeeting));
        let minutes = alert_info.minutes_remaining;
        assert!(
            minutes >= 4 && minutes <= 6,
            "Expected ~5 minutes, got {}",
            minutes
        );
    }
}
