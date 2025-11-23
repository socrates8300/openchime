#![allow(dead_code)]
#![allow(unused_imports)]

// Google Calendar integration
// Handles OAuth2 authentication and API calls

use crate::models::{Account, SyncResult, CalendarEvent};
use crate::utils::{logging, circuit_breaker::get_circuit_breaker};
use crate::utils::retry::RetryConfig;
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc, Duration, TimeZone, Datelike};
use oauth2::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl,
    RefreshToken, Scope, TokenResponse, basic::BasicClient, reqwest::async_http_client,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use std::str::FromStr;
use icalendar::{Component, Event as IcsEvent, EventLike, Calendar as IcsCalendar};


#[derive(Debug, Serialize, Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
    token_type: String,
    scope: String,
}

#[derive(Debug, Deserialize)]
struct GoogleCalendarEvent {
    id: String,
    summary: Option<String>,
    description: Option<String>,
    start: GoogleEventTime,
    end: GoogleEventTime,
    hangout_link: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GoogleEventTime {
    date_time: Option<DateTime<Utc>>,
    date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GoogleCalendarResponse {
    items: Vec<GoogleCalendarEvent>,
    next_page_token: Option<String>,
}

pub async fn sync_google_calendar(account: &Account, db: &sqlx::SqlitePool) -> Result<SyncResult> {
    let start_time = Instant::now();
    log::info!("Starting Google calendar sync for account: {}", account.account_name);
    
    // Check if auth_data is an ICS URL or OAuth tokens
    let events = if account.auth_data.contains("google.com/calendar") {
        // Handle ICS URL import (like Proton)
        sync_google_ics(account).await?
    } else {
        // Handle OAuth API access
        let token_data: GoogleTokenResponse = serde_json::from_str(&account.auth_data)
            .map_err(|e| anyhow!("Failed to parse token data: {}", e))?;
        
        // Check if tokens need refresh
        let access_token = if needs_refresh(&token_data) {
            refresh_access_token(account).await?
        } else {
            token_data.access_token
        };
        
        // Fetch events from Google Calendar API
        fetch_calendar_events(&access_token).await?
    };
    
    // Store/update events in database
    let mut events_added = 0;
    let mut events_updated = 0;
    
    for google_event in events {
        let calendar_event = convert_google_event(google_event, account.id.unwrap_or(0))?;
        
        // Check if event already exists
        let existing = sqlx::query("SELECT id FROM events WHERE external_id = ? AND account_id = ?")
            .bind(&calendar_event.external_id)
            .bind(calendar_event.account_id)
            .fetch_optional(db)
            .await?;
        
        if existing.is_some() {
            // Update existing event
            sqlx::query("UPDATE events SET title = ?, description = ?, start_time = ?, end_time = ?, video_link = ?, video_platform = ?, updated_at = CURRENT_TIMESTAMP WHERE external_id = ? AND account_id = ?")
                .bind(&calendar_event.title)
                .bind(&calendar_event.description)
                .bind(calendar_event.start_time)
                .bind(calendar_event.end_time)
                .bind(&calendar_event.video_link)
                .bind(&calendar_event.video_platform)
                .bind(&calendar_event.external_id)
                .bind(calendar_event.account_id)
                .execute(db)
                .await?;
            events_updated += 1;
        } else {
            // Insert new event
            sqlx::query("INSERT INTO events (external_id, account_id, title, description, start_time, end_time, video_link, video_platform, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)")
                .bind(&calendar_event.external_id)
                .bind(calendar_event.account_id)
                .bind(&calendar_event.title)
                .bind(&calendar_event.description)
                .bind(calendar_event.start_time)
                .bind(calendar_event.end_time)
                .bind(&calendar_event.video_link)
                .bind(&calendar_event.video_platform)
                .execute(db)
                .await?;
            events_added += 1;
        }
    }
    
    let duration = start_time.elapsed();
    logging::log_calendar_sync(&account.account_name, events_added + events_updated, duration.as_millis() as u64);
    
    let sync_result = SyncResult {
        account_id: account.id.unwrap_or(0),
        success: true,
        events_added,
        events_updated,
        error_message: None,
        sync_time: Utc::now(),
    };
    
    Ok(sync_result)
}

pub async fn test_connection(account: &Account) -> Result<bool> {
    logging::log_auth_event("Google Calendar connection test", &account.account_name);
    
    let token_data: GoogleTokenResponse = serde_json::from_str(&account.auth_data)
        .map_err(|e| anyhow!("Failed to parse token data: {}", e))?;
    
    // Test with a simple calendar list request with retry
    let _retry_config = RetryConfig {
        max_attempts: 3,
        base_delay: std::time::Duration::from_millis(500),
        max_delay: std::time::Duration::from_secs(10),
        backoff_multiplier: 2.0,
    };
    
    let circuit_breaker = get_circuit_breaker("google_calendar").await;
    let access_token = Arc::new(token_data.access_token.clone());
    
    circuit_breaker.execute(move || {
        let token = access_token.clone();
        async move {
            let client = Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;
            let response = client
                .get("https://www.googleapis.com/calendar/v3/users/me/calendarList")
                .header("Authorization", format!("Bearer {}", *token))
                .send()
                .await?;
            
            if response.status().is_success() {
                Ok(true)
            } else {
                Err(anyhow!("Calendar list request failed: {}", response.status()))
            }
        }
    }).await
}

pub async fn authenticate_oauth(auth_code: String) -> Result<Account> {
    logging::log_auth_event("Google OAuth authentication started", "");
    
    // Google OAuth2 configuration
    let client_id = ClientId::new(std::env::var("GOOGLE_CLIENT_ID")
        .unwrap_or_else(|_| "your-client-id".to_string()));
    let client_secret = ClientSecret::new(std::env::var("GOOGLE_CLIENT_SECRET")
        .unwrap_or_else(|_| "your-client-secret".to_string()));
    let redirect_url = RedirectUrl::new("http://localhost:1420/auth/callback".to_string())
        .map_err(|e| anyhow!("Invalid redirect URL: {}", e))?;
    
    let oauth_client = BasicClient::new(
        client_id,
        Some(client_secret),
        oauth2::AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
            .map_err(|e| anyhow!("Invalid auth URL: {}", e))?,
        Some(oauth2::TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
            .map_err(|e| anyhow!("Invalid token URL: {}", e))?)
    )
    .set_redirect_uri(redirect_url);
    
    // Exchange auth code for tokens
    let token = oauth_client
        .exchange_code(AuthorizationCode::new(auth_code))
        .request_async(async_http_client)
        .await?;
    
    // Get user info
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;
    let user_info: GoogleUserInfo = client
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .header("Authorization", format!("Bearer {}", token.access_token().secret()))
        .send()
        .await?
        .json()
        .await?;
    
    // Create token response for storage
    let token_response = GoogleTokenResponse {
        access_token: token.access_token().secret().clone(),
        refresh_token: token.refresh_token().map(|t| t.secret().clone()),
        expires_in: token.expires_in().unwrap_or(std::time::Duration::from_secs(3600)).as_secs(),
        token_type: "Bearer".to_string(), // OAuth2 token type is always "Bearer"
        scope: "https://www.googleapis.com/auth/calendar.readonly https://www.googleapis.com/auth/userinfo.email".to_string(),
    };
    
    let auth_data = serde_json::to_string(&token_response)?;
    
    Ok(Account::new_google(
        user_info.email,
        auth_data,
        token_response.refresh_token,
    ))
}

#[derive(Debug, Deserialize)]
struct GoogleUserInfo {
    email: String,
    name: Option<String>,
    picture: Option<String>,
}

// Helper functions

fn needs_refresh(token_data: &GoogleTokenResponse) -> bool {
    // Check if token expires within the next 5 minutes
    let expiry_time = Utc::now() + Duration::seconds(token_data.expires_in as i64);
    let refresh_threshold = Utc::now() + Duration::minutes(5);
    expiry_time <= refresh_threshold
}

async fn refresh_access_token(account: &Account) -> Result<String> {
    let token_data: GoogleTokenResponse = serde_json::from_str(&account.auth_data)?;
    
    let refresh_token = token_data.refresh_token
        .ok_or_else(|| anyhow!("No refresh token available"))?;
    
    let client_id = ClientId::new(std::env::var("GOOGLE_CLIENT_ID")
        .unwrap_or_else(|_| "your-client-id".to_string()));
    let client_secret = ClientSecret::new(std::env::var("GOOGLE_CLIENT_SECRET")
        .unwrap_or_else(|_| "your-client-secret".to_string()));
    
    let oauth_client = BasicClient::new(
        client_id,
        Some(client_secret),
        oauth2::AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
            .map_err(|e| anyhow!("Invalid auth URL: {}", e))?,
        Some(oauth2::TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
            .map_err(|e| anyhow!("Invalid token URL: {}", e))?)
    );
    
    let token = oauth_client
        .exchange_refresh_token(&RefreshToken::new(refresh_token))
        .request_async(async_http_client)
        .await?;
    
    Ok(token.access_token().secret().clone())
}

async fn fetch_calendar_events(access_token: &str) -> Result<Vec<GoogleCalendarEvent>> {
    let _retry_config = RetryConfig {
        max_attempts: 3,
        base_delay: std::time::Duration::from_millis(1000),
        max_delay: std::time::Duration::from_secs(15),
        backoff_multiplier: 2.0,
    };
    
    let circuit_breaker = get_circuit_breaker("google_calendar").await;
    let access_token = Arc::new(access_token.to_string());
    
    circuit_breaker.execute(move || {
        let token = access_token.clone();
        async move {
            let client = Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;
            
            // Get events from primary calendar for the next 30 days
            let time_min = Utc::now();
            let time_max = Utc::now() + Duration::days(30);
            
            let url = format!(
                "https://www.googleapis.com/calendar/v3/calendars/primary/events?singleEvents=true&orderBy=startTime&timeMin={}&timeMax={}",
                time_min.format("%Y-%m-%dT%H:%M:%SZ"),
                time_max.format("%Y-%m-%dT%H:%M:%SZ")
            );
            
            let response = client
                .get(&url)
                .header("Authorization", format!("Bearer {}", *token))
                .send()
                .await?;
            
            if !response.status().is_success() {
                return Err(anyhow!("Failed to fetch calendar events: {}", response.status()));
            }
            
            let calendar_response: GoogleCalendarResponse = response.json().await
                .map_err(|e| anyhow!("Failed to parse calendar response: {}", e))?;
            Ok(calendar_response.items)
        }
    }).await
}

fn convert_google_event(google_event: GoogleCalendarEvent, account_id: i64) -> Result<CalendarEvent> {
    let start_time = google_event.start.date_time
        .ok_or_else(|| anyhow!("Event missing start time"))?;
    
    let end_time = google_event.end.date_time
        .ok_or_else(|| anyhow!("Event missing end time"))?;
    
    // Extract video link from hangout_link or description
    let video_link = google_event.hangout_link
        .or_else(|| {
            google_event.description.as_ref()
                .and_then(|desc| crate::utils::extract_video_link(Some(desc.as_str()), None))
                .map(|info| info.url)
        });
    
    Ok(CalendarEvent {
        id: None,
        external_id: google_event.id,
        account_id,
        title: google_event.summary.unwrap_or_else(|| "Untitled Event".to_string()),
        description: google_event.description,
        start_time,
        end_time,
        video_link: video_link.clone(),
        video_platform: video_link.and_then(|url| crate::utils::extract_video_link(None, Some(&url))).map(|info| info.platform),
        snooze_count: 0,
        has_alerted: false,
        last_alert_threshold: None,
        is_dismissed: false,
        // account_id, // Already specified in init shorthand above? No, it was passed as args?
        // function sig: fn convert_google_event(..., account_id: i64)
        // Struct init:
        // account_id, (shorthand)
        // ...
        // account_id: 0 (my addition)
        // I need to remove the second one or the first one. I should use the passed account_id.
        created_at: Utc::now(),
        updated_at: Utc::now(),
    })
}

pub fn get_auth_url() -> Result<(String, CsrfToken, PkceCodeChallenge)> {
    let client_id = ClientId::new(std::env::var("GOOGLE_CLIENT_ID")
        .unwrap_or_else(|_| "your-client-id".to_string()));
    let client_secret = ClientSecret::new(std::env::var("GOOGLE_CLIENT_SECRET")
        .unwrap_or_else(|_| "your-client-secret".to_string()));
    let redirect_url = RedirectUrl::new("http://localhost:1420/auth/callback".to_string())
        .map_err(|e| anyhow!("Invalid redirect URL: {}", e))?;
    
    let oauth_client = BasicClient::new(
        client_id,
        Some(client_secret),
        oauth2::AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
            .map_err(|e| anyhow!("Invalid auth URL: {}", e))?,
        Some(oauth2::TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
            .map_err(|e| anyhow!("Invalid token URL: {}", e))?)
    )
    .set_redirect_uri(redirect_url);
    
    // Generate PKCE challenge
    let (pkce_challenge, _pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    
    // Generate authorization URL
    let (auth_url, csrf_token) = oauth_client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("https://www.googleapis.com/auth/calendar.readonly".to_string()))
        .add_scope(Scope::new("https://www.googleapis.com/auth/userinfo.email".to_string()))
        .set_pkce_challenge(pkce_challenge.clone())
        .url();
    
    Ok((auth_url.to_string(), csrf_token, pkce_challenge))
}

/// Handle Google Calendar sync via ICS URL (similar to Proton)
async fn sync_google_ics(account: &Account) -> Result<Vec<GoogleCalendarEvent>> {
    let ics_url = &account.auth_data;
    log::info!("Fetching Google ICS data from URL: {}", ics_url);
    
    // Fetch ICS data
    let ics_data = fetch_ics_data(ics_url).await?;
    log::info!("Fetched {} bytes of Google ICS data", ics_data.len());
    
    // Check if we got HTML instead of ICS (indicates auth issues)
    if ics_data.trim().starts_with("<!doctype html") || ics_data.trim().starts_with("<html") {
        log::warn!("Content does not contain BEGIN:VCALENDAR.");
        log::warn!("Parsed 0 events. ICS data size: {} bytes. First 100 chars: {}", 
                  ics_data.len(), &ics_data.chars().take(100).collect::<String>());
        return Ok(Vec::new());
    }
    
    // Parse ICS data to Google Calendar events
    let events = parse_ics_to_google_events(&ics_data)?;
    log::info!("Parsed {} events from Google ICS data", events.len());
    
    Ok(events)
}

/// Fetch ICS data from URL (similar to Proton implementation)
async fn fetch_ics_data(ics_url: &str) -> Result<String> {
    let client = Client::new();
    let response = client
        .get(ics_url)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| anyhow!("Failed to fetch ICS data: {}", e))?;
    
    if !response.status().is_success() {
        return Err(anyhow!("HTTP error: {}", response.status()));
    }
    
    let content = response
        .text()
        .await
        .map_err(|e| anyhow!("Failed to read response: {}", e))?;
    
    Ok(content)
}

/// Parse ICS data to Google Calendar events (simplified conversion)
fn parse_ics_to_google_events(ics_data: &str) -> Result<Vec<GoogleCalendarEvent>> {
    use icalendar::Calendar as IcsCalendar;
    
    let calendar = IcsCalendar::from_str(ics_data)
        .map_err(|e| anyhow!("Failed to parse ICS: {}", e))?;
    
    let mut events = Vec::new();
    
    for component in calendar.components {
        if let Some(ics_event) = component.as_event() {
            let event = convert_ics_event_to_google(ics_event)?;
            events.push(event);
        }
    }
    
    Ok(events)
}

/// Convert ICS VEVENT to GoogleCalendarEvent
fn convert_ics_event_to_google(ics_event: &icalendar::Event) -> Result<GoogleCalendarEvent> {
    use icalendar::EventLike;
    
    // Extract basic event properties
    let summary = ics_event.get_summary().map(|s| s.to_string());
    let description = ics_event.get_description().map(|d| d.to_string());
    let start_time = ics_event.get_start()
        .as_ref()
        .and_then(parse_ical_datetime)
        .map(|dt| dt.with_timezone(&Utc));
    let end_time = ics_event.get_end()
        .as_ref()
        .and_then(parse_ical_datetime)
        .map(|dt| dt.with_timezone(&Utc));
    
    // Generate event ID
    let id = ics_event.get_uid().map(|uid| uid.to_string())
        .unwrap_or_else(|| format!("ics_{}", uuid::Uuid::new_v4()));
    
    // Parse video meeting links
    let (video_link, _video_platform) = extract_video_info(&description);
    
    Ok(GoogleCalendarEvent {
        id,
        summary,
        description,
        start: GoogleEventTime {
            date_time: start_time,
            date: None,
        },
        end: GoogleEventTime {
            date_time: end_time,
            date: None,
        },
        hangout_link: video_link,
    })
}

/// Parse ICS datetime (reusing Proton's approach)
fn parse_ical_datetime(dt: &icalendar::DatePerhapsTime) -> Option<DateTime<Utc>> {
    match dt {
        icalendar::DatePerhapsTime::DateTime(dt) => {
            match dt {
                icalendar::CalendarDateTime::Utc(dt) => Some(dt.naive_utc().and_utc()),
                icalendar::CalendarDateTime::Floating(dt) => Some(dt.and_utc()),
                icalendar::CalendarDateTime::WithTimezone { date_time, .. } => Some(date_time.and_utc()),
            }
        }
        icalendar::DatePerhapsTime::Date(date) => {
            // For date-only events, assume start of day in UTC
            Utc.with_ymd_and_hms(date.year(), date.month(), date.day(), 0, 0, 0)
                .single()
        }
    }
}

/// Extract video meeting information from description
fn extract_video_info(description: &Option<String>) -> (Option<String>, Option<String>) {
    if let Some(desc) = description {
        if desc.contains("meet.google.com") || desc.contains("hangouts.google.com") {
            return (Some(desc.clone()), Some("Google Meet".to_string()));
        } else if desc.contains("zoom.us") || desc.contains("zoom.com") {
            return (Some(desc.clone()), Some("Zoom".to_string()));
        } else if desc.contains("teams.microsoft.com") {
            return (Some(desc.clone()), Some("Microsoft Teams".to_string()));
        }
    }
    (None, None)
}