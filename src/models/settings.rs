// file: src/settings.rs
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Setting {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub sound: String,
    pub volume: f32,               // 0.0 to 1.0
    pub video_alert_offset: i32,   // minutes before meeting
    pub regular_alert_offset: i32, // minutes before meeting
    pub snooze_interval: i32,      // minutes
    pub max_snoozes: i32,
    pub sync_interval: i32,        // seconds
    pub auto_join_enabled: bool,
    pub theme: String,
    pub alert_30m: bool,
    pub alert_10m: bool,
    pub alert_5m: bool,
    pub alert_1m: bool,
    pub alert_default: bool, // At start time
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            sound: "bells".to_string(),
            volume: 0.7, // 70% volume by default
            video_alert_offset: 3,
            regular_alert_offset: 1,
            snooze_interval: 2,
            max_snoozes: 3,
            sync_interval: 300, // 5 minutes
            auto_join_enabled: false,
            theme: "dark".to_string(),
            alert_30m: false,
            alert_10m: false,
            alert_5m: true,
            alert_1m: true,
            alert_default: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_default() {
        let settings = Settings::default();
        assert_eq!(settings.sound, "bells");
        assert_eq!(settings.volume, 0.7);
        assert_eq!(settings.video_alert_offset, 3);
        assert_eq!(settings.regular_alert_offset, 1);
        assert_eq!(settings.snooze_interval, 2);
        assert_eq!(settings.max_snoozes, 3);
        assert_eq!(settings.sync_interval, 300);
        assert!(!settings.auto_join_enabled);
        assert_eq!(settings.theme, "dark");
        assert!(!settings.alert_30m);
        assert!(!settings.alert_10m);
        assert!(settings.alert_5m);
        assert!(settings.alert_1m);
        assert!(settings.alert_default);
    }
}
