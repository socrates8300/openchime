// file: src/account.rs
// ICS-only mode - OAuth and encryption removed
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CalendarProvider {
    Google,
    Proton,
}

impl CalendarProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            CalendarProvider::Google => "google",
            CalendarProvider::Proton => "proton",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Account {
    pub id: Option<i64>,
    pub provider: String,
    pub account_name: String,
    pub auth_data: String, // JSON: OAuth tokens for Google, ICS URL for Proton
    pub refresh_token: Option<String>,
    pub last_synced_at: Option<DateTime<Utc>>,
}

impl Account {
    pub fn new_google(
        account_name: String,
        auth_data: String,
        refresh_token: Option<String>,
    ) -> Self {
        Self {
            id: None,
            provider: CalendarProvider::Google.as_str().to_string(),
            account_name,
            auth_data,
            refresh_token,
            last_synced_at: None,
        }
    }

    pub fn new_proton(account_name: String, ics_url: String) -> Self {
        Self {
            id: None,
            provider: CalendarProvider::Proton.as_str().to_string(),
            account_name,
            auth_data: ics_url,
            refresh_token: None,
            last_synced_at: None,
        }
    }

    pub fn provider(&self) -> Result<CalendarProvider, String> {
        match self.provider.as_str() {
            "google" => Ok(CalendarProvider::Google),
            "proton" => Ok(CalendarProvider::Proton),
            _ => Err(format!("Unknown provider: {}", self.provider)),
        }
    }

    // Encryption methods removed - ICS-only mode doesn't need encryption
    // ICS URLs are public/semi-public links, not secret OAuth tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calendar_provider_as_str() {
        assert_eq!(CalendarProvider::Google.as_str(), "google");
        assert_eq!(CalendarProvider::Proton.as_str(), "proton");
    }

    #[test]
    fn test_account_new_google() {
        let account = Account::new_google(
            "test@gmail.com".to_string(),
            "auth_data".to_string(),
            Some("refresh_token".to_string()),
        );

        assert_eq!(account.provider, "google");
        assert_eq!(account.account_name, "test@gmail.com");
        assert_eq!(account.auth_data, "auth_data");
        assert_eq!(account.refresh_token, Some("refresh_token".to_string()));
    }

    #[test]
    fn test_account_new_proton() {
        let account = Account::new_proton(
            "user@proton.me".to_string(),
            "https://calendar.proton.me/ics".to_string(),
        );

        assert_eq!(account.provider, "proton");
        assert_eq!(account.account_name, "user@proton.me");
        assert_eq!(account.auth_data, "https://calendar.proton.me/ics");
        assert_eq!(account.refresh_token, None);
    }

    #[test]
    fn test_account_provider() {
        let google_account =
            Account::new_google("test@gmail.com".to_string(), "auth".to_string(), None);
        let proton_account =
            Account::new_proton("user@proton.me".to_string(), "ics_url".to_string());

        assert!(matches!(
            google_account.provider().unwrap(),
            CalendarProvider::Google
        ));
        assert!(matches!(
            proton_account.provider().unwrap(),
            CalendarProvider::Proton
        ));
    }
}
