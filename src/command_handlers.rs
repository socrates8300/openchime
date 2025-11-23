//! Command handlers module
//! 
//! This module contains async command handlers that were previously
//! embedded directly in main.rs. Extracting these improves maintainability
//! and testability.

use crate::database::Database;
use crate::models::{Account, Settings, CalendarEvent};
use crate::audio::AudioManager;
use crate::calendar;
use crate::error::AppError;
use log::{info, error};
use anyhow::anyhow;

/// Database operation handlers
pub struct DatabaseHandlers {
    pub db: Database,
}

impl DatabaseHandlers {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Load all events from database
    pub async fn load_events(&self) -> Result<Vec<CalendarEvent>, AppError> {
        info!("Loading events from database");
        let events = sqlx::query_as::<_, CalendarEvent>(
            "SELECT id, external_id, account_id, title, description, start_time, end_time, video_link, video_platform, snooze_count, has_alerted, last_alert_threshold, is_dismissed, created_at, updated_at FROM events ORDER BY start_time ASC LIMIT 50"
        )
        .fetch_all(&self.db.pool)
        .await
        .map_err(|e| AppError::Database(e))?;

        info!("Loaded {} events from database", events.len());
        Ok(events)
    }

    /// Load all accounts from database
    pub async fn load_accounts(&self) -> Result<Vec<Account>, AppError> {
        info!("Loading accounts from database");
        let accounts = sqlx::query_as::<_, Account>(
            "SELECT id, provider, account_name, auth_data, refresh_token, last_synced_at FROM accounts ORDER BY created_at ASC"
        )
        .fetch_all(&self.db.pool)
        .await
        .map_err(|e| AppError::Database(e))?;

        info!("Loaded {} accounts from database", accounts.len());
        Ok(accounts)
    }

    /// Add a new account to database
    pub async fn add_account(&self, account: Account) -> Result<Account, AppError> {
        info!("Adding account: {}", account.account_name);
        
        sqlx::query(
            "INSERT INTO accounts (provider, account_name, auth_data, created_at, updated_at) VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)"
        )
        .bind("proton")
        .bind(&account.account_name)
        .bind(&account.auth_data)
        .execute(&self.db.pool)
        .await
        .map_err(|e| AppError::Database(e))?;

        info!("Successfully added account: {}", account.account_name);
        Ok(account)
    }

    /// Delete an account from database
    pub async fn delete_account(&self, account_id: i64) -> Result<(), AppError> {
        info!("Deleting account ID: {}", account_id);
        
        sqlx::query("DELETE FROM accounts WHERE id = ?")
            .bind(account_id)
            .execute(&self.db.pool)
            .await
            .map_err(|e| AppError::Database(e))?;

        info!("Successfully deleted account ID: {}", account_id);
        Ok(())
    }

    /// Update settings in database
    pub async fn update_settings(&self, settings: &Settings) -> Result<(), AppError> {
        info!("Updating settings in database");
        crate::database::settings::update(&self.db.pool, settings)
            .await
            .map_err(|e| AppError::Anyhow(anyhow!("Failed to update settings: {}", e)))
    }
}

/// Calendar operation handlers
pub struct CalendarHandlers {
    pub db: Database,
}

impl CalendarHandlers {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Synchronize all calendar accounts
    pub async fn sync_calendars(&self) -> Result<(usize, usize), AppError> {
        info!("Starting calendar synchronization");
        
        // Get all accounts
        let accounts = sqlx::query_as::<_, Account>(
            "SELECT id, provider, account_name, auth_data, refresh_token, last_synced_at FROM accounts"
        )
        .fetch_all(&self.db.pool)
        .await
        .map_err(|e| AppError::Database(e))?;

        if accounts.is_empty() {
            return Err(AppError::OperationFailed("No accounts configured. Please add an account first.".to_string()));
        }

        let mut total_events = 0;
        let mut successful_syncs = 0;

        for account in accounts.iter() {
            info!("Attempting to sync account: {} ({})", account.account_name, account.provider);
            match calendar::sync_account(account, &self.db.pool).await {
                Ok(sync_result) => {
                    total_events += sync_result.events_added + sync_result.events_updated;
                    successful_syncs += 1;
                    info!("Synced account {}: {} events added, {} events updated", 
                          account.account_name, sync_result.events_added, sync_result.events_updated);
                }
                Err(e) => {
                    error!("Failed to sync account {}: {}", account.account_name, e);
                    // Continue with other accounts even if one fails
                }
            }
        }

        if successful_syncs == 0 {
            return Err(AppError::OperationFailed("Failed to sync any accounts".to_string()));
        }

        info!("Calendar synchronization completed: {} events processed", total_events);
        Ok((total_events, successful_syncs))
    }
}

/// Test audio system
pub async fn test_audio(audio: &AudioManager) -> Result<(), AppError> {
    info!("Testing audio system");
    audio.play_alert(crate::audio::AlertType::Meeting)
        .map_err(|e| AppError::Audio(format!("Audio test failed: {}", e)))
}

/// Command handler factory
pub struct CommandHandlers {
    pub database: DatabaseHandlers,
    pub calendar: CalendarHandlers,
    pub audio: std::sync::Arc<AudioManager>,
}

impl CommandHandlers {
    pub fn new(db: &std::sync::Arc<Database>, audio: &std::sync::Arc<AudioManager>) -> Self {
        Self {
            database: DatabaseHandlers::new(db.as_ref().clone()),
            calendar: CalendarHandlers::new(db.as_ref().clone()),
            audio: audio.clone(),
        }
    }
}
