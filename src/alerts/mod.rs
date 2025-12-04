#![allow(dead_code)]
use crate::{models::{CalendarEvent, Account}, calendar, AppState};
use crate::audio::AlertType;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use log::{info, error, warn, debug};
use chrono::Utc;
use tokio::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub enum MonitorEvent {
    AlertTriggered(CalendarEvent),
    SyncCompleted { added: usize, updated: usize },
    Error(String),
}

pub async fn monitor_meetings(state: Arc<AppState>, sender: Option<Sender<MonitorEvent>>) {
    info!("Starting meeting monitor loop");

    let mut last_sync = Utc::now();

    loop {
        // Check for shutdown signal
        if state.shutdown.is_cancelled() {
            info!("Shutdown signal received, stopping monitor loop");
            break;
        }

        match monitor_cycle(&state, &mut last_sync, &sender).await {
            Ok(_) => {
                debug!("Monitor cycle completed successfully");
            }
            Err(e) => {
                error!("Error in monitor cycle: {}", e);
                if let Some(tx) = &sender {
                    let _ = tx.send(MonitorEvent::Error(e.to_string())).await;
                }
            }
        }

        // Sleep for 30 seconds between checks, but wake on shutdown
        tokio::select! {
            _ = sleep(Duration::from_secs(30)) => {
                // Normal sleep completed, continue loop
            }
            _ = state.shutdown.cancelled() => {
                info!("Shutdown signal received during sleep, stopping monitor loop");
                break;
            }
        }
    }

    info!("Meeting monitor loop stopped gracefully");
}

async fn monitor_cycle(state: &AppState, last_sync: &mut chrono::DateTime<Utc>, sender: &Option<Sender<MonitorEvent>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let now = Utc::now();
    
    // Check if we need to sync calendars (every 5 minutes)
    if (now - *last_sync).num_seconds() >= 300 {
        info!("Triggering calendar sync");
        match sync_calendars(state).await {
            Ok(stats) => {
                *last_sync = now;
                if let Some(tx) = sender {
                    let _ = tx.send(MonitorEvent::SyncCompleted { 
                        added: stats.0, 
                        updated: stats.1 
                    }).await;
                }
            }
            Err(e) => {
                error!("Calendar sync failed: {}", e);
                return Err(e);
            }
        }
    }
    
    // Get upcoming events that need alerts
    let events_needing_alerts = get_upcoming_events(&state.db.pool).await?;
    let settings = state.db.get_settings().await?;
    
    for event in events_needing_alerts {
        if let Some((threshold, alert_type)) = check_alert_thresholds(&event, &settings) {
            info!("Triggering {}m alert for event: {}", threshold, event.title);
            
            // Play alert sound
            if let Err(e) = play_alert_sound(&event, &state, alert_type.clone()).await {
                warn!("Failed to play alert sound: {}", e);
            }
            
            // Notify UI via channel
            if let Some(tx) = sender {
                let _ = tx.send(MonitorEvent::AlertTriggered(event.clone())).await;
            }
            
            // Update last_alert_threshold in DB
            sqlx::query("UPDATE events SET last_alert_threshold = ? WHERE id = ?")
                .bind(threshold)
                .bind(event.id)
                .execute(&state.db.pool)
                .await?;
        }
    }
    
    Ok(())
}

pub fn check_alert_thresholds(event: &CalendarEvent, settings: &crate::models::Settings) -> Option<(i32, AlertType)> {
    let now = Utc::now();
    let minutes_until = (event.start_time - now).num_minutes();
    
    // Check strict thresholds
    let thresholds = [
        (30, settings.alert_30m, AlertType::Warning30m),
        (10, settings.alert_10m, AlertType::Warning10m),
        (5, settings.alert_5m, AlertType::Warning5m),
        (1, settings.alert_1m, AlertType::Warning1m),
        (0, settings.alert_default, if event.is_video_meeting() { AlertType::VideoMeeting } else { AlertType::Meeting }), // 0 is "Start"
    ];
    
    for (threshold, enabled, alert_type) in thresholds {
        if enabled {
            // Logic:
            // 1. We have passed the threshold (minutes_until <= threshold)
            // 2. We are within a reasonable window (e.g. 2 minutes) so we don't alert for 30m when we are at 5m (if missed)
            // 3. We haven't alerted for this threshold yet (implied by last_alert > threshold, OR last_alert is None)
            //    (Since we iterate descending 30->0, if last_alert is 10, we skip 30. Correct).
            
            let window_ok = minutes_until <= threshold as i64 && minutes_until > (threshold as i64 - 5); // 5 minute grace window
            
            let not_alerted_yet = match event.last_alert_threshold {
                Some(last) => last > threshold,
                None => true,
            };
            
            if window_ok && not_alerted_yet {
                return Some((threshold, alert_type));
            }
        }
    }
    
    None
}

pub fn should_trigger_alert(event: &CalendarEvent) -> bool {
    // Legacy function kept for compatibility if needed, checking default logic
    let now = Utc::now();
    let minutes_until = (event.start_time - now).num_minutes();
    (0..=3).contains(&minutes_until)
}

pub async fn sync_calendars(state: &AppState) -> Result<(usize, usize), Box<dyn std::error::Error + Send + Sync>> {
    info!("Starting calendar sync");
    
    let accounts = sqlx::query_as::<_, Account>(
        "SELECT id, provider, account_name, auth_data, refresh_token, last_synced_at FROM accounts"
    )
    .fetch_all(&state.db.pool)
    .await?;
    
    let mut total_added = 0;
    let mut total_updated = 0;
    
    for account in accounts {
        match calendar::sync_account(&account, &state.db.pool).await {
            Ok(sync_result) => {
                info!("Synced account {}: {} events added, {} events updated", 
                      account.account_name, sync_result.events_added, sync_result.events_updated);
                
                total_added += sync_result.events_added;
                total_updated += sync_result.events_updated;

                // Update last_synced_at
                sqlx::query("UPDATE accounts SET last_synced_at = CURRENT_TIMESTAMP WHERE id = ?")
                    .bind(account.id.unwrap_or(0))
                    .execute(&state.db.pool)
                    .await?;
            }
            Err(e) => {
                error!("Failed to sync account {}: {}", account.account_name, e);
            }
        }
    }
    
    info!("Calendar sync completed");
    Ok((total_added, total_updated))
}

async fn play_alert_sound(event: &CalendarEvent, state: &AppState, alert_type: AlertType) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Playing alert sound for event: {}", event.title);
    
    state.audio.play_alert(alert_type)
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { 
            format!("Audio playback failed: {}", e).into() 
        })?;
    
    Ok(())
}

async fn show_alert_window(event: &CalendarEvent) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Showing alert window for event: {}", event.title);
    
    // Alert window display is handled by the main application UI
    Ok(())
}

pub async fn get_upcoming_events(pool: &sqlx::SqlitePool) -> Result<Vec<CalendarEvent>, Box<dyn std::error::Error + Send + Sync>> {
    let now = Utc::now();
    let future = now + chrono::Duration::minutes(60); // Look ahead 60 minutes to catch 30m alerts
    
    let events = sqlx::query_as::<_, CalendarEvent>(
        r#"
        SELECT id, external_id, account_id, title, description, start_time, end_time, video_link, video_platform,
               snooze_count, has_alerted, last_alert_threshold, is_dismissed,
               created_at, updated_at
        FROM events 
        WHERE start_time BETWEEN ? AND ?
        ORDER BY start_time ASC
        "#
    )
    .bind(now - chrono::Duration::minutes(5)) // Look back 5 mins for late alerts
    .bind(future)
    .fetch_all(pool)
    .await?;
    
    Ok(events)
}

pub async fn trigger_manual_alert(event_id: i64, state: &AppState) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Get the specific event
    let events = sqlx::query_as::<_, CalendarEvent>(
        "SELECT id, external_id, account_id, title, description, start_time, end_time, video_link, video_platform, snooze_count, has_alerted, last_alert_threshold, is_dismissed, created_at, updated_at FROM events WHERE id = ?"
    )
    .bind(event_id)
    .fetch_all(&state.db.pool)
    .await?;
    
    if let Some(event) = events.into_iter().next() {
        info!("Manually triggering alert for event: {}", event.title);
        
        let alert_type = if event.is_video_meeting() { AlertType::VideoMeeting } else { AlertType::Meeting };
        
        // Play alert sound
        if let Err(e) = play_alert_sound(&event, state, alert_type).await {
            warn!("Failed to play alert sound: {}", e);
        }
        
        // Show alert window
        if let Err(e) = show_alert_window(&event).await {
            error!("Failed to show alert window: {}", e);
        }
    } else {
        return Err(format!("Event not found: {}", event_id).into());
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CalendarEvent, AlertType, AlertInfo};
    use chrono::{Duration, Utc};
    use tempfile::NamedTempFile;
    use std::sync::Arc;
    use sqlx::SqlitePool;
    use crate::AudioManager;

    fn create_test_event(minutes_from_now: i64, has_video: bool) -> CalendarEvent {
        let now = Utc::now();
        CalendarEvent {
            id: Some(1),
            external_id: "test-event".to_string(),
            account_id: 1,
            title: "Test Meeting".to_string(),
            description: Some("Test description".to_string()),
            start_time: now + Duration::minutes(minutes_from_now),
            end_time: now + Duration::minutes(minutes_from_now + 60),
            video_link: if has_video {
                Some("https://zoom.us/test".to_string())
            } else {
                None
            },
            video_platform: if has_video {
                Some("Zoom".to_string())
            } else {
                None
            },
            snooze_count: 0,
            has_alerted: false,
            last_alert_threshold: None,
            is_dismissed: false,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn test_should_trigger_alert_video_meeting() {
        let event_2_min_away = create_test_event(2, true);
        let event_3_min_away = create_test_event(3, true);
        // Use 5 minutes to safely ensure it's > 3 minutes even with slow execution
        let event_5_min_away = create_test_event(5, true); 
        let event_past = create_test_event(-1, true);

        assert!(should_trigger_alert(&event_2_min_away)); // 2 min <= 3 min threshold
        assert!(should_trigger_alert(&event_3_min_away)); // 3 min <= 3 min threshold
        assert!(!should_trigger_alert(&event_5_min_away)); // 5 min > 3 min threshold
        assert!(!should_trigger_alert(&event_past)); // Past event (handled by >= 0 check in code)
    }

    #[test]
    fn test_should_trigger_alert_regular_meeting() {
        let event_30_sec_away = create_test_event(0, false); // 30 seconds away
        let event_1_min_away = create_test_event(1, false);
        let event_3_min_away = create_test_event(3, false);
        let event_5_min_away = create_test_event(5, false);
        let event_past = create_test_event(-1, false);

        assert!(should_trigger_alert(&event_30_sec_away)); // 0 is in 0..=3 range
        assert!(should_trigger_alert(&event_1_min_away)); // 1 is in 0..=3 range
        assert!(should_trigger_alert(&event_3_min_away)); // 3 is in 0..=3 range
        assert!(!should_trigger_alert(&event_5_min_away)); // 5 > 3, outside range
        assert!(!should_trigger_alert(&event_past)); // Past event (-1 not in 0..=3)
    }

    #[tokio::test]
    async fn test_get_upcoming_events_empty() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = format!("sqlite:file:{}?mode=rwc", temp_file.path().to_str().unwrap());
        
        let pool = SqlitePool::connect(&db_path).await.unwrap();
        let schema = include_str!("../database/schema.sql");
        sqlx::query(schema).execute(&pool).await.unwrap();

        let events = get_upcoming_events(&pool).await.unwrap();
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn test_trigger_manual_alert_event_not_found() {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = format!("sqlite:file:{}?mode=rwc", temp_file.path().to_str().unwrap());
        
        let pool = SqlitePool::connect(&db_path).await.unwrap();
        let schema = include_str!("../database/schema.sql");
        sqlx::query(schema).execute(&pool).await.unwrap();

        // Create a mock audio manager that doesn't actually play sound
        let audio = AudioManager::new().unwrap();
        let db = crate::database::Database { pool };
        let state = Arc::new(crate::AppState {
            db: std::sync::Arc::new(db),
            audio: std::sync::Arc::new(audio),
            shutdown: tokio_util::sync::CancellationToken::new(),
        });

        let result = trigger_manual_alert(999, &state).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Event not found"));
    }

    #[test]
    fn test_alert_info_creation() {
        let event = create_test_event(5, true);
        let alert_info = AlertInfo::new(event.clone());
        
        assert!(matches!(alert_info.alert_type, AlertType::VideoMeeting));
        // Allow for 1 minute of drift (4 or 5 is acceptable)
        assert!(alert_info.minutes_remaining >= 4 && alert_info.minutes_remaining <= 5, 
            "Expected ~5 minutes, got {}", alert_info.minutes_remaining);
        assert_eq!(alert_info.event.title, "Test Meeting");
    }

    #[test]
    fn test_alert_info_regular_meeting() {
        let event = create_test_event(2, false);
        let alert_info = AlertInfo::new(event.clone());
        
        assert!(matches!(alert_info.alert_type, AlertType::Meeting));
        let minutes = alert_info.minutes_remaining;
        assert!(minutes >= 1 && minutes <= 3, "Expected ~2 minutes, got {}", minutes);
    }

    #[tokio::test]
    async fn test_play_alert_sound_success() {
        let event = create_test_event(5, true);
        let audio = Arc::new(AudioManager::new().unwrap());
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = format!("sqlite:file:{}?mode=rwc", temp_file.path().to_str().unwrap());
        let pool = SqlitePool::connect(&db_path).await.unwrap();
        let schema = include_str!("../database/schema.sql");
        sqlx::query(schema).execute(&pool).await.unwrap();
        let db = Arc::new(crate::database::Database { pool });
        let state = Arc::new(crate::AppState {
            db,
            audio,
            shutdown: tokio_util::sync::CancellationToken::new(),
        });

        // This should not panic even if sound file doesn't exist
        let result = play_alert_sound(&event, &state, crate::models::AlertType::VideoMeeting).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_show_alert_window() {
        let event = create_test_event(5, true);
        
        // This should not panic even though it's not fully implemented
        let result = show_alert_window(&event).await;
        assert!(result.is_ok());
    }
}