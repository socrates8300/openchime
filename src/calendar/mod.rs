#![allow(dead_code)]
// Calendar integration module
// Handles Google Calendar ICS and Proton ICS feed parsing (ICS-only, OAuth removed)

use crate::models::{Account, SyncResult};
use anyhow::Result;
use sqlx::SqlitePool;

pub mod google;
pub mod proton;
pub mod common;

pub async fn sync_account(account: &Account, db: &SqlitePool) -> Result<SyncResult> {
    use crate::utils::circuit_breaker::get_circuit_breaker;

    let provider = account.provider().map_err(|e| anyhow::anyhow!("{}", e))?;
    let service_name = match provider {
        crate::models::CalendarProvider::Google => "google_calendar",
        crate::models::CalendarProvider::Proton => "proton_calendar",
    };

    // Get circuit breaker for this service
    let breaker = get_circuit_breaker(service_name).await;

    // Execute sync through circuit breaker
    let account_clone = account.clone();
    let db_clone = db.clone();
    let provider_clone = provider.clone();

    breaker.execute(move || {
        let account = account_clone.clone();
        let db = db_clone.clone();
        let provider = provider_clone.clone();
        async move {
            match provider {
                crate::models::CalendarProvider::Google => {
                    google::sync_google_calendar(&account, &db).await
                }
                crate::models::CalendarProvider::Proton => {
                    proton::sync_proton_calendar(&account, &db).await
                }
            }
        }
    }).await
}

pub async fn test_connection(account: &Account) -> Result<bool> {
    use crate::utils::circuit_breaker::get_circuit_breaker;

    let provider = account.provider().map_err(|e| anyhow::anyhow!("{}", e))?;
    let service_name = match provider {
        crate::models::CalendarProvider::Google => "google_calendar",
        crate::models::CalendarProvider::Proton => "proton_calendar",
    };

    // Get circuit breaker for this service
    let breaker = get_circuit_breaker(service_name).await;

    // Execute connection test through circuit breaker
    let account_clone = account.clone();
    let provider_clone = provider.clone();

    breaker.execute(move || {
        let account = account_clone.clone();
        let provider = provider_clone.clone();
        async move {
            match provider {
                crate::models::CalendarProvider::Google => {
                    google::test_connection(&account).await
                }
                crate::models::CalendarProvider::Proton => {
                    proton::test_connection(&account).await
                }
            }
        }
    }).await
}