// file: src/database.rs

use anyhow::{Context, Result};
use log::info;
use sqlx::{migrate::MigrateDatabase, sqlite::SqlitePool, Sqlite, Row};

// Declare submodules
pub mod accounts;
pub mod events;
pub mod settings;

#[derive(Clone)]
pub struct Database {
    pub pool: SqlitePool,
}

impl Database {
    pub async fn new() -> Result<Self> {
        let db_path = "sqlite:openchime.db?mode=rwc";

        // Create database if it doesn't exist
        let db_exists = Sqlite::database_exists(db_path)
            .await
            .context("Failed to check if database exists")?;
        if !db_exists {
            info!("Creating database");
            Sqlite::create_database(db_path)
                .await
                .context("Failed to create database")?;
        }

        // Connect to database
        let pool = SqlitePool::connect(db_path)
            .await
            .context("Failed to connect to database")?;

        // Run schema migrations
        run_schema(&pool).await.context("Failed to run database schema")?;

        // Ensure specific migrations for existing databases
        ensure_migrations(&pool).await.context("Failed to ensure migrations")?;

        info!("Database initialized successfully");

        Ok(Database { pool })
    }

    // --- Event Delegates ---

    pub async fn get_upcoming_events(&self) -> Result<Vec<crate::models::CalendarEvent>> {
        events::get_upcoming(&self.pool).await
    }

    pub async fn get_events_needing_alert(&self) -> Result<Vec<crate::models::CalendarEvent>> {
        events::get_needing_alert(&self.pool).await
    }

    pub async fn mark_event_alerted(&self, event_id: &str) -> Result<()> {
        events::mark_alerted(&self.pool, event_id).await
    }

    pub async fn snooze_event(&self, event_id: &str) -> Result<()> {
        events::snooze(&self.pool, event_id).await
    }

    pub async fn dismiss_event(&self, event_id: &str) -> Result<()> {
        events::dismiss(&self.pool, event_id).await
    }

    // --- Settings Delegates ---

    pub async fn get_settings(&self) -> Result<crate::models::Settings> {
        settings::get(&self.pool).await
    }

    pub async fn update_settings(&self, settings: &crate::models::Settings) -> Result<()> {
        settings::update(&self.pool, settings).await
    }

    // --- Account Delegates ---

    pub async fn add_account(&self, account: &crate::models::Account) -> Result<i64> {
        accounts::add(&self.pool, account).await
    }

    pub async fn get_accounts(&self) -> Result<Vec<crate::models::Account>> {
        accounts::get_all(&self.pool).await
    }

    pub async fn update_sync_time(&self, account_id: i64) -> Result<()> {
        accounts::update_sync_time(&self.pool, account_id).await
    }
}

async fn run_schema(pool: &SqlitePool) -> Result<()> {
    let schema = include_str!("schema.sql");
    
    let mut current_statement = String::new();
    let mut in_trigger = false;
    
    for line in schema.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("--") || trimmed.is_empty() {
            continue;
        }
        
        if trimmed.to_uppercase().starts_with("CREATE TRIGGER") {
            in_trigger = true;
        }
        
        current_statement.push_str(line);
        current_statement.push('\n'); 
        
        if trimmed.ends_with(';') {
            if in_trigger {
                if trimmed.to_uppercase() == "END;" {
                    in_trigger = false;
                    sqlx::query(&current_statement).execute(pool).await?;
                    current_statement.clear();
                }
            } else {
                sqlx::query(&current_statement).execute(pool).await?;
                current_statement.clear();
            }
        }
    }
    Ok(())
}

async fn ensure_migrations(pool: &SqlitePool) -> Result<()> {
    // Check columns in events table
    let rows = sqlx::query("PRAGMA table_info(events)")
        .fetch_all(pool)
        .await
        .context("Failed to fetch table info")?;
    
    let columns: Vec<String> = rows
        .iter()
        .map(|row| row.get::<String, _>("name"))
        .collect();

    if !columns.contains(&"video_platform".to_string()) {
        info!("Migrating: Adding video_platform column to events table");
        sqlx::query("ALTER TABLE events ADD COLUMN video_platform TEXT")
            .execute(pool)
            .await
            .context("Failed to add video_platform column")?;
    }

    if !columns.contains(&"snooze_count".to_string()) {
        info!("Migrating: Adding snooze_count column to events table");
        sqlx::query("ALTER TABLE events ADD COLUMN snooze_count INTEGER DEFAULT 0")
            .execute(pool)
            .await
            .context("Failed to add snooze_count column")?;
    }

    if !columns.contains(&"has_alerted".to_string()) {
        info!("Migrating: Adding has_alerted column to events table");
        sqlx::query("ALTER TABLE events ADD COLUMN has_alerted BOOLEAN DEFAULT 0")
            .execute(pool)
            .await
            .context("Failed to add has_alerted column")?;
    }

    if !columns.contains(&"last_alert_threshold".to_string()) {
        info!("Migrating: Adding last_alert_threshold column to events table");
        sqlx::query("ALTER TABLE events ADD COLUMN last_alert_threshold INTEGER")
            .execute(pool)
            .await
            .context("Failed to add last_alert_threshold column")?;
    }

    if !columns.contains(&"is_dismissed".to_string()) {
        info!("Migrating: Adding is_dismissed column to events table");
        sqlx::query("ALTER TABLE events ADD COLUMN is_dismissed BOOLEAN DEFAULT 0")
            .execute(pool)
            .await
            .context("Failed to add is_dismissed column")?;
    }

    if !columns.contains(&"last_snoozed_at".to_string()) {
        info!("Migrating: Adding last_snoozed_at column to events table");
        sqlx::query("ALTER TABLE events ADD COLUMN last_snoozed_at DATETIME")
            .execute(pool)
            .await
            .context("Failed to add last_snoozed_at column")?;
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Account, Settings};
    use tempfile::NamedTempFile;

    async fn create_test_database() -> Database {
        let temp_file = NamedTempFile::new().unwrap();
        let (_, path) = temp_file.keep().unwrap();
        let db_path = format!("sqlite:{}", path.to_str().unwrap());

        let pool = SqlitePool::connect(&db_path).await.unwrap();

        // Run schema
        run_schema(&pool).await.unwrap();

        Database { pool }
    }

    #[tokio::test]
    async fn test_database_new() {
        let db = create_test_database().await;
        assert!(db.pool.is_closed() == false);
    }

    #[tokio::test]
    async fn test_add_account() {
        let db = create_test_database().await;
        let account = Account::new_google(
            "test@gmail.com".to_string(),
            "auth_data".to_string(),
            Some("refresh_token".to_string()),
        );

        let account_id = db.add_account(&account).await.unwrap();
        assert!(account_id > 0);
    }

    #[tokio::test]
    async fn test_get_accounts() {
        let db = create_test_database().await;

        // Add test accounts
        let google_account = Account::new_google(
            "test@gmail.com".to_string(),
            "auth_data".to_string(),
            None,
        );
        let proton_account = Account::new_proton(
            "user@proton.me".to_string(),
            "https://calendar.proton.me/ics".to_string(),
        );

        db.add_account(&google_account).await.unwrap();
        db.add_account(&proton_account).await.unwrap();

        let accounts = db.get_accounts().await.unwrap();
        assert_eq!(accounts.len(), 2);
        assert_eq!(accounts[0].provider, "google");
        assert_eq!(accounts[1].provider, "proton");
    }

    #[tokio::test]
    async fn test_get_settings_default() {
        let db = create_test_database().await;
        let settings = db.get_settings().await.unwrap();

        assert_eq!(settings.sound, "bells");
        assert_eq!(settings.volume, 0.7);
        assert_eq!(settings.video_alert_offset, 3);
    }

    #[tokio::test]
    async fn test_update_settings() {
        let db = create_test_database().await;
        let mut settings = Settings::default();
        settings.volume = 0.5;
        settings.sound = "chime".to_string();

        db.update_settings(&settings).await.unwrap();

        let retrieved = db.get_settings().await.unwrap();
        assert_eq!(retrieved.volume, 0.5);
        assert_eq!(retrieved.sound, "chime");
    }

    #[tokio::test]
    async fn test_update_sync_time() {
        let db = create_test_database().await;
        let account = Account::new_google(
            "test@gmail.com".to_string(),
            "auth_data".to_string(),
            None,
        );

        let account_id = db.add_account(&account).await.unwrap();
        db.update_sync_time(account_id).await.unwrap();

        let accounts = db.get_accounts().await.unwrap();
        assert!(accounts[0].last_synced_at.is_some());
    }

    #[tokio::test]
    async fn test_get_upcoming_events_empty() {
        let db = create_test_database().await;
        let events = db.get_upcoming_events().await.unwrap();
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn test_get_events_needing_alert_empty() {
        let db = create_test_database().await;
        let events = db.get_events_needing_alert().await.unwrap();
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn test_snooze_event_not_found() {
        let db = create_test_database().await;
        let result = db.snooze_event("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_dismiss_event_not_found() {
        let db = create_test_database().await;
        let result = db.dismiss_event("nonexistent").await;
        assert!(result.is_ok()); // Updating 0 rows is not an error in SQL
    }

    #[tokio::test]
    async fn test_mark_event_alerted_not_found() {
        let db = create_test_database().await;
        let result = db.mark_event_alerted("nonexistent").await;
        assert!(result.is_ok()); // Updating 0 rows is not an error in SQL
    }
}
