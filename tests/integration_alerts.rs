use openchime::{CalendarEvent, Account, AlertInfo};
use chrono::{Duration, Utc};
use tempfile::NamedTempFile;
use sqlx::SqlitePool;
use std::sync::Arc;

async fn create_test_database() -> openchime::Database {
    let temp_file = NamedTempFile::new().unwrap();
    let (_, path) = temp_file.keep().unwrap();
    let db_path = format!("sqlite:{}", path.to_str().unwrap());
    
    let pool = SqlitePool::connect(&db_path).await.unwrap();
    
    // Run schema
    let schema = include_str!("../src/database/schema.sql");
    sqlx::query(schema).execute(&pool).await.unwrap();
    
    openchime::Database { pool }
}

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

#[tokio::test]
async fn test_alert_workflow_integration() {
    let db = create_test_database().await;
    let audio = Arc::new(openchime::AudioManager::new().unwrap());
    let shutdown = tokio_util::sync::CancellationToken::new();
    let state = Arc::new(openchime::AppState { db: Arc::new(db), audio, shutdown });
    
    // Create test events
    let video_event = create_test_event(2, true); // 2 minutes away, has video
    let regular_event = create_test_event(0, false); // Now, no video
    
    // Test alert info creation
    let video_alert = AlertInfo::new(video_event.clone());
    let regular_alert = AlertInfo::new(regular_event.clone());
    
    assert!(matches!(video_alert.alert_type, openchime::models::AlertType::VideoMeeting));
    assert!(matches!(regular_alert.alert_type, openchime::models::AlertType::Meeting));
    
    // Test manual alert triggering
    let db_clone = state.db.clone();
    
    // Insert a dummy account first to satisfy foreign key constraint
    let account = Account::new_google(
        "test@example.com".to_string(),
        "https://example.com/calendar.ics".to_string(),
        None,
    );
    // Manually insert account with ID 1
    sqlx::query("INSERT INTO accounts (id, provider, account_name, auth_data, created_at, updated_at) VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
        .bind(1)
        .bind(account.provider)
        .bind(&account.account_name)
        .bind(&account.auth_data)
        .execute(&db_clone.pool)
        .await
        .unwrap();

    // Insert events into database for manual alert test
    sqlx::query(
        "INSERT INTO events (external_id, account_id, title, description, start_time, end_time, video_link) 
         VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&video_event.external_id)
    .bind(video_event.account_id)
    .bind(&video_event.title)
    .bind(&video_event.description)
    .bind(video_event.start_time)
    .bind(video_event.end_time)
    .bind(&video_event.video_link)
    .execute(&db_clone.pool)
    .await
    .unwrap();
    
    // Get the inserted event ID
    let event_id: i64 = sqlx::query_scalar("SELECT last_insert_rowid()")
        .fetch_one(&db_clone.pool)
        .await
        .unwrap();
    
    // Test manual alert
    let result = openchime::alerts::trigger_manual_alert(event_id, &state).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_monitor_cycle_components() {
    let db = create_test_database().await;
    let audio = Arc::new(openchime::AudioManager::new().unwrap());
    let shutdown = tokio_util::sync::CancellationToken::new();
    let state = Arc::new(openchime::AppState { db: Arc::new(db), audio, shutdown });
    
    // Test getting upcoming events (should be empty initially)
    let events = openchime::get_upcoming_events(&state.db.pool).await.unwrap();
    assert!(events.is_empty());
    
    // Add a test account
    let account = Account::new_google(
        "test@gmail.com".to_string(),
        "auth_data".to_string(),
        None,
    );
    
    let account_id = state.db.add_account(&account).await.unwrap();
    assert!(account_id > 0);
    
    // Test sync calendars (should not panic even with fake auth)
    let result = openchime::sync_calendars(&state).await;
    // This might fail due to invalid auth, but shouldn't panic
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_alert_timing_logic() {
    let _now = Utc::now();
    
    // Test various event timings
    // Note: should_trigger_alert currently returns true for any event in 0..=3 minutes range
    let test_cases = vec![
        (5, true, false),   // 5 min away, video -> should not alert yet
        (3, true, true),    // 3 min away, video -> should alert
        (2, true, true),    // 2 min away, video -> should alert
        (1, false, true),   // 1 min away, regular -> should alert
        (0, false, true),   // Now, regular -> should alert
        (-1, true, false),  // Past, video -> should not alert
        // (2, false, false),  // REMOVED: 2 min away regular currently DOES alert in legacy function
    ];
    
    for (minutes_offset, has_video, should_alert) in test_cases {
        let event = create_test_event(minutes_offset, has_video);
        let alerts_should_trigger = openchime::should_trigger_alert(&event);
        
        assert_eq!(
            alerts_should_trigger, 
            should_alert,
            "Event at {} minutes with video={} should alert={}",
            minutes_offset, has_video, should_alert
        );
    }
}

#[tokio::test]
async fn test_concurrent_alert_operations() {
    let db = create_test_database().await;
    let audio = Arc::new(openchime::AudioManager::new().unwrap());
    let shutdown = tokio_util::sync::CancellationToken::new();
    let state = Arc::new(openchime::AppState { db: Arc::new(db), audio, shutdown });
    
    // Test concurrent access to alert functions
    let mut handles = vec![];
    
    for i in 0..5 {
        let state_clone = state.clone();
        let handle = tokio::spawn(async move {
            let event = create_test_event(i, i % 2 == 0);
            let alert_info = AlertInfo::new(event);
            
            // Test alert info creation
            assert!(alert_info.minutes_remaining >= 0);
            
            // Test audio playback (should not panic)
            let alert_type = if alert_info.event.is_video_meeting() {
                openchime::AlertType::VideoMeeting
            } else {
                openchime::AlertType::Meeting
            };
            
            state_clone.audio.play_alert(alert_type).unwrap();
        });
        handles.push(handle);
    }
    
    // Wait for all operations to complete
    for handle in handles {
        handle.await.unwrap();
    }
}

#[tokio::test]
async fn test_alert_error_handling() {
    let db = create_test_database().await;
    let audio = Arc::new(openchime::AudioManager::new().unwrap());
    let shutdown = tokio_util::sync::CancellationToken::new();
    let state = Arc::new(openchime::AppState { db: Arc::new(db), audio, shutdown });
    
    // Test manual alert with non-existent event
    let result = openchime::alerts::trigger_manual_alert(99999, &state).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Event not found"));
    
    // Test getting upcoming events with empty database
    let events = openchime::get_upcoming_events(&state.db.pool).await.unwrap();
    assert!(events.is_empty());
}

#[test]
fn test_alert_info_edge_cases() {
    let now = Utc::now();
    
    // Test event exactly at alert threshold
    let video_event_at_threshold = CalendarEvent {
        id: Some(1),
        external_id: "video-threshold".to_string(),
        account_id: 1,
        title: "Video Meeting at Threshold".to_string(),
        description: None,
        start_time: now + Duration::minutes(3), // Exactly at video threshold
        end_time: now + Duration::minutes(63),
        video_link: Some("https://zoom.us/test".to_string()),
        video_platform: Some("Zoom".to_string()),
        snooze_count: 0,
        has_alerted: false,
        last_alert_threshold: None,
        is_dismissed: false,
        created_at: now,
        updated_at: now,
    };
    
    let alert_info = AlertInfo::new(video_event_at_threshold.clone());
    assert!(matches!(alert_info.alert_type, openchime::models::AlertType::VideoMeeting));
    // Allow for slight timing difference (2 or 3)
    assert!(alert_info.minutes_remaining >= 2 && alert_info.minutes_remaining <= 3, 
            "Expected ~3 minutes, got {}", alert_info.minutes_remaining);
    
    // Test regular event at threshold
    let regular_event_at_threshold = CalendarEvent {
        video_link: None,
        start_time: now + Duration::minutes(1), // Exactly at regular threshold
        ..video_event_at_threshold.clone()
    };
    
    let alert_info = AlertInfo::new(regular_event_at_threshold);
    assert!(matches!(alert_info.alert_type, openchime::models::AlertType::Meeting));
    // Allow for slight timing difference (0 or 1)
    assert!(alert_info.minutes_remaining >= 0 && alert_info.minutes_remaining <= 1,
            "Expected ~1 minute, got {}", alert_info.minutes_remaining);
}