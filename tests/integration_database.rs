use openchime::{Database, Account, Settings};
use tempfile::NamedTempFile;
use sqlx::SqlitePool;

async fn create_test_database() -> Database {
    let temp_file = NamedTempFile::new().unwrap();
    let (_, path) = temp_file.keep().unwrap();
    let db_path = format!("sqlite:{}", path.to_str().unwrap());
    
    let pool = SqlitePool::connect(&db_path).await.unwrap();
    
    // Run schema
    let schema = include_str!("../src/database/schema.sql");
    sqlx::query(schema).execute(&pool).await.unwrap();
    
    Database { pool }
}

#[tokio::test]
async fn test_full_account_and_event_workflow() {
    let db = create_test_database().await;
    
    // 1. Add a Google account
    let google_account = Account::new_google(
        "test@gmail.com".to_string(),
        "fake_auth_data".to_string(),
        Some("fake_refresh_token".to_string()),
    );
    
    let account_id = db.add_account(&google_account).await.unwrap();
    assert!(account_id > 0);
    
    // 2. Verify account was saved
    let accounts = db.get_accounts().await.unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].provider, "google");
    assert_eq!(accounts[0].account_name, "test@gmail.com");
    
    // 3. Update sync time
    db.update_sync_time(account_id).await.unwrap();
    
    // 4. Verify sync time was updated
    let accounts = db.get_accounts().await.unwrap();
    assert!(accounts[0].last_synced_at.is_some());
    
    // 5. Test settings workflow
    let mut settings = Settings::default();
    settings.volume = 0.8;
    settings.sound = "custom_chime".to_string();
    
    db.update_settings(&settings).await.unwrap();
    
    let retrieved_settings = db.get_settings().await.unwrap();
    assert_eq!(retrieved_settings.volume, 0.8);
    assert_eq!(retrieved_settings.sound, "custom_chime");
}

#[tokio::test]
async fn test_multiple_accounts_management() {
    let db = create_test_database().await;
    
    // Add multiple accounts
    let google_account = Account::new_google(
        "work@gmail.com".to_string(),
        "work_auth".to_string(),
        None,
    );
    
    let proton_account = Account::new_proton(
        "personal@proton.me".to_string(),
        "https://calendar.proton.me/personal/ics".to_string(),
    );
    
    let work_id = db.add_account(&google_account).await.unwrap();
    let _personal_id = db.add_account(&proton_account).await.unwrap();
    
    // Verify both accounts exist
    let accounts = db.get_accounts().await.unwrap();
    assert_eq!(accounts.len(), 2);
    
    // Update sync times independently
    db.update_sync_time(work_id).await.unwrap();
    
    let accounts = db.get_accounts().await.unwrap();
    let work_account = accounts.iter().find(|a| a.provider == "google").unwrap();
    let personal_account = accounts.iter().find(|a| a.provider == "proton").unwrap();
    
    assert!(work_account.last_synced_at.is_some());
    assert!(personal_account.last_synced_at.is_none());
}

#[tokio::test]
async fn test_settings_persistence() {
    let db = create_test_database().await;
    
    // Get default settings
    let initial_settings = db.get_settings().await.unwrap();
    assert_eq!(initial_settings.volume, 0.7);
    
    // Update multiple settings
    let mut new_settings = Settings::default();
    new_settings.volume = 0.3;
    new_settings.sound = "bells".to_string();
    new_settings.video_alert_offset = 5;
    new_settings.regular_alert_offset = 2;
    new_settings.snooze_interval = 5;
    new_settings.max_snoozes = 5;
    new_settings.sync_interval = 600;
    new_settings.auto_join_enabled = true;
    new_settings.theme = "light".to_string();
    
    db.update_settings(&new_settings).await.unwrap();
    
    // Verify all settings persisted
    let persisted = db.get_settings().await.unwrap();
    assert_eq!(persisted.volume, 0.3);
    assert_eq!(persisted.sound, "bells");
    assert_eq!(persisted.video_alert_offset, 5);
    assert_eq!(persisted.regular_alert_offset, 2);
    assert_eq!(persisted.snooze_interval, 5);
    assert_eq!(persisted.max_snoozes, 5);
    assert_eq!(persisted.sync_interval, 600);
    assert!(persisted.auto_join_enabled);
    assert_eq!(persisted.theme, "light");
}

#[tokio::test]
async fn test_database_connection_resilience() {
    let db = create_test_database().await;
    
    // Test multiple concurrent operations
    let db_clone1 = db.clone();
    let db_clone2 = db.clone();
    
    let handle1 = tokio::spawn(async move {
        let account = Account::new_google("user1@test.com".to_string(), "auth1".to_string(), None);
        db_clone1.add_account(&account).await.unwrap()
    });
    
    let handle2 = tokio::spawn(async move {
        let account = Account::new_proton("user2@proton.me".to_string(), "ics_url".to_string());
        db_clone2.add_account(&account).await.unwrap()
    });
    
    let (result1, result2) = tokio::join!(handle1, handle2);
    assert!(result1.unwrap() > 0);
    assert!(result2.unwrap() > 0);
    
    // Verify both accounts were added
    let accounts = db.get_accounts().await.unwrap();
    assert_eq!(accounts.len(), 2);
}