#![allow(dead_code)]
// Calendar integration module
// Handles Google Calendar API and Proton ICS feed parsing

use crate::models::{Account, SyncResult};
use anyhow::Result;
use sqlx::SqlitePool;

pub mod google;
pub mod proton;

pub async fn sync_account(account: &Account, db: &SqlitePool) -> Result<SyncResult> {
    match account.provider().map_err(|e| anyhow::anyhow!("{}", e))? {
        crate::models::CalendarProvider::Google => {
            google::sync_google_calendar(account, db).await
        }
        crate::models::CalendarProvider::Proton => {
            proton::sync_proton_calendar(account, db).await
        }
    }
}

pub async fn test_connection(account: &Account) -> Result<bool> {
    match account.provider().map_err(|e| anyhow::anyhow!("{}", e))? {
        crate::models::CalendarProvider::Google => {
            google::test_connection(account).await
        }
        crate::models::CalendarProvider::Proton => {
            proton::test_connection(account).await
        }
    }
}

pub async fn authenticate_google(auth_code: String) -> Result<Account> {
    google::authenticate_oauth(auth_code).await
}

pub fn get_google_auth_url() -> Result<(String, String)> {
    let (auth_url, _csrf_token, _pkce_challenge) = google::get_auth_url()?;
    Ok((auth_url, _csrf_token.secret().clone()))
}