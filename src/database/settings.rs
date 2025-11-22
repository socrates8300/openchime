// file: src/database/settings.rs
use anyhow::Result;
use sqlx::SqlitePool;

pub async fn get(pool: &SqlitePool) -> Result<crate::models::Settings> {
    let settings = sqlx::query_as::<_, crate::models::Setting>("SELECT key, value FROM settings")
        .fetch_all(pool)
        .await?;

    // Convert to Settings struct
    let mut app_settings = crate::models::Settings::default();
    for setting in settings {
        match setting.key.as_str() {
            "sound" => app_settings.sound = setting.value,
            "volume" => app_settings.volume = setting.value.parse().unwrap_or(0.7),
            "video_alert_offset" => {
                app_settings.video_alert_offset = setting.value.parse().unwrap_or(3)
            }
            "regular_alert_offset" => {
                app_settings.regular_alert_offset = setting.value.parse().unwrap_or(1)
            }
            "snooze_interval" => app_settings.snooze_interval = setting.value.parse().unwrap_or(2),
            "max_snoozes" => app_settings.max_snoozes = setting.value.parse().unwrap_or(3),
            "sync_interval" => app_settings.sync_interval = setting.value.parse().unwrap_or(300),
            "auto_join_enabled" => {
                app_settings.auto_join_enabled = setting.value.parse().unwrap_or(false)
            }
            "theme" => app_settings.theme = setting.value,
            "alert_30m" => app_settings.alert_30m = setting.value.parse().unwrap_or(false),
            "alert_10m" => app_settings.alert_10m = setting.value.parse().unwrap_or(false),
            "alert_5m" => app_settings.alert_5m = setting.value.parse().unwrap_or(true),
            "alert_1m" => app_settings.alert_1m = setting.value.parse().unwrap_or(true),
            "alert_default" => app_settings.alert_default = setting.value.parse().unwrap_or(true),
            _ => {}
        }
    }

    Ok(app_settings)
}

pub async fn update(pool: &SqlitePool, settings: &crate::models::Settings) -> Result<()> {
    let sound_str = settings.sound.clone();
    let volume_str = settings.volume.to_string();
    let video_alert_offset_str = settings.video_alert_offset.to_string();
    let regular_alert_offset_str = settings.regular_alert_offset.to_string();
    let snooze_interval_str = settings.snooze_interval.to_string();
    let max_snoozes_str = settings.max_snoozes.to_string();
    let sync_interval_str = settings.sync_interval.to_string();
    let auto_join_enabled_str = settings.auto_join_enabled.to_string();
    let theme_str = settings.theme.clone();
    let alert_30m_str = settings.alert_30m.to_string();
    let alert_10m_str = settings.alert_10m.to_string();
    let alert_5m_str = settings.alert_5m.to_string();
    let alert_1m_str = settings.alert_1m.to_string();
    let alert_default_str = settings.alert_default.to_string();

    let updates = vec![
        ("sound", sound_str.as_str()),
        ("volume", volume_str.as_str()),
        ("video_alert_offset", video_alert_offset_str.as_str()),
        ("regular_alert_offset", regular_alert_offset_str.as_str()),
        ("snooze_interval", snooze_interval_str.as_str()),
        ("max_snoozes", max_snoozes_str.as_str()),
        ("sync_interval", sync_interval_str.as_str()),
        ("auto_join_enabled", auto_join_enabled_str.as_str()),
        ("theme", theme_str.as_str()),
        ("alert_30m", alert_30m_str.as_str()),
        ("alert_10m", alert_10m_str.as_str()),
        ("alert_5m", alert_5m_str.as_str()),
        ("alert_1m", alert_1m_str.as_str()),
        ("alert_default", alert_default_str.as_str()),
    ];

    for (key, value) in updates {
        sqlx::query("UPDATE settings SET value = ? WHERE key = ?")
            .bind(value)
            .bind(key)
            .execute(pool)
            .await?;
    }

    Ok(())
}
