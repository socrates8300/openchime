// file: src/event.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CalendarEvent {
    pub id: Option<i64>,
    pub external_id: String,
    pub account_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub video_link: Option<String>,
    pub video_platform: Option<String>,
    pub snooze_count: i32,
    pub has_alerted: bool,
    pub last_alert_threshold: Option<i32>,
    pub is_dismissed: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CalendarEvent {
    pub fn is_video_meeting(&self) -> bool {
        self.video_link.is_some()
    }

    pub fn minutes_until_start(&self) -> i64 {
        let now = Utc::now();
        (self.start_time - now).num_minutes()
    }

    pub fn is_past(&self) -> bool {
        self.start_time < Utc::now()
    }

    pub fn is_happening_now(&self) -> bool {
        let now = Utc::now();
        now >= self.start_time && now <= self.end_time
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_calendar_event_is_video_meeting() {
        let event_with_video = CalendarEvent {
            id: None,
            external_id: "test-1".to_string(),
            account_id: 1,
            title: "Video Meeting".to_string(),
            description: None,
            start_time: Utc::now(),
            end_time: Utc::now() + Duration::hours(1),
            video_link: Some("https://zoom.us/j/123456".to_string()),
            video_platform: Some("Zoom".to_string()),
            snooze_count: 0,
            has_alerted: false,
            last_alert_threshold: None,
            is_dismissed: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let event_without_video = CalendarEvent {
            video_link: None,
            ..event_with_video.clone()
        };

        assert!(event_with_video.is_video_meeting());
        assert!(!event_without_video.is_video_meeting());
    }

    #[test]
    fn test_calendar_event_minutes_until_start() {
        let now = Utc::now();
        let future_event = CalendarEvent {
            id: None,
            external_id: "test-2".to_string(),
            account_id: 1,
            title: "Future Meeting".to_string(),
            description: None,
            start_time: now + Duration::minutes(30),
            end_time: now + Duration::minutes(90),
            video_link: None,
            video_platform: None,
            snooze_count: 0,
            has_alerted: false,
            last_alert_threshold: None,
            is_dismissed: false,
            created_at: now,
            updated_at: now,
        };

        let minutes = future_event.minutes_until_start();
        assert!(
            minutes >= 29 && minutes <= 31,
            "Expected ~30 minutes, got {}",
            minutes
        );
    }

    #[test]
    fn test_calendar_event_is_past() {
        let now = Utc::now();
        let past_event = CalendarEvent {
            id: None,
            external_id: "test-3".to_string(),
            account_id: 1,
            title: "Past Meeting".to_string(),
            description: None,
            start_time: now - Duration::hours(1),
            end_time: now - Duration::minutes(30),
            video_link: None,
            video_platform: None,
            snooze_count: 0,
            has_alerted: false,
            last_alert_threshold: None,
            is_dismissed: false,
            created_at: now - Duration::hours(2),
            updated_at: now - Duration::hours(2),
        };

        assert!(past_event.is_past());
    }

    #[test]
    fn test_calendar_event_is_happening_now() {
        let now = Utc::now();
        let ongoing_event = CalendarEvent {
            id: None,
            external_id: "test-4".to_string(),
            account_id: 1,
            title: "Ongoing Meeting".to_string(),
            description: None,
            start_time: now - Duration::minutes(15),
            end_time: now + Duration::minutes(45),
            video_link: None,
            video_platform: None,
            snooze_count: 0,
            has_alerted: false,
            last_alert_threshold: None,
            is_dismissed: false,
            created_at: now - Duration::hours(1),
            updated_at: now - Duration::hours(1),
        };

        assert!(ongoing_event.is_happening_now());
    }
}
