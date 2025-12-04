#![allow(dead_code)]
// Proton Calendar integration via ICS feed
// Handles ICS fetching and parsing

use crate::models::{Account, CalendarEvent, SyncResult};
use crate::utils;
use crate::utils::logging;
use crate::calendar::common;
use anyhow::{anyhow, Result};
use chrono::Utc;
use icalendar::{Component, Event as IcsEvent, EventLike, Calendar as IcsCalendar};
use sqlx::SqlitePool;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::str::FromStr;
use std::time::Instant;

pub async fn sync_proton_calendar(account: &Account, pool: &SqlitePool) -> Result<SyncResult> {
    let start_time = Instant::now();
    log::info!("Starting Proton calendar sync for account: {}", account.account_name);
    
    // Extract ICS URL from auth_data
    let ics_url = &account.auth_data;
    log::info!("Fetching ICS data from URL: {}", ics_url);
    
    // Fetch ICS data
    let ics_data = common::fetch_ics_data(ics_url, "proton_calendar").await?;
    log::info!("Fetched {} bytes of ICS data", ics_data.len());
    
    // Parse ICS data
    let events = parse_ics_data(&ics_data)?;
    log::info!("Parsed {} events from ICS data", events.len());
    
    // Store events in database
    let mut events_added = 0;
    let mut events_updated = 0;
    
    for event in events {
        log::debug!("Processing event: {} ({})", event.title, event.start_time);
        match store_event(&event, account.id.unwrap_or(0), pool).await {
            Ok(true) => {
                events_added += 1;
                log::debug!("Added new event: {}", event.title);
            }
            Ok(false) => {
                events_updated += 1;
                log::debug!("Updated existing event: {}", event.title);
            }
            Err(e) => {
                log::warn!("Failed to store event {}: {}", event.title, e);
            }
        }
    }
    
    let duration = start_time.elapsed();
    logging::log_calendar_sync(&account.account_name, events_added + events_updated, duration.as_millis() as u64);
    
    let sync_result = SyncResult::with_counts(
        account.id.unwrap_or(0),
        events_added,
        events_updated,
    );
    
    log::info!("Proton calendar sync completed: {} events added, {} updated", events_added, events_updated);
    Ok(sync_result)
}

pub async fn test_connection(account: &Account) -> Result<bool> {
    let ics_url = &account.auth_data;
    
    logging::log_auth_event("Proton ICS connection test", &account.account_name);
    
    match common::fetch_ics_data(ics_url, "proton_calendar").await {
        Ok(_) => {
            log::info!("Proton ICS connection successful for: {}", account.account_name);
            Ok(true)
        }
        Err(e) => {
            log::warn!("Proton ICS connection failed for {}: {}", account.account_name, e);
            Ok(false)
        }
    }
}

pub async fn validate_ics_url(ics_url: &str) -> Result<bool> {
    match common::fetch_ics_data(ics_url, "proton_calendar").await {
        Ok(ics_data) => {
            // Try to parse the ICS data to ensure it's valid
            match IcsCalendar::from_str(&ics_data) {
                Ok(_) => {
                    log::info!("ICS URL is valid and accessible: {}", ics_url);
                    Ok(true)
                }
                Err(e) => {
                    log::warn!("ICS data is invalid from {}: {}", ics_url, e);
                    Ok(false)
                }
            }
        }
        Err(e) => {
            log::warn!("Failed to access ICS URL {}: {}", ics_url, e);
            Ok(false)
        }
    }
}

fn parse_ics_data(ics_data: &str) -> Result<Vec<CalendarEvent>> {
    let calendar = IcsCalendar::from_str(ics_data)
        .map_err(|e| anyhow!("Failed to parse ICS data: {}", e))?;
    
    let mut events = Vec::new();
    
    for component in calendar.components {
        if let Some(ics_event) = component.as_event() {
            if let Ok(event) = convert_ics_event(ics_event) {
                events.push(event);
            }
        }
    }
    
    if events.is_empty() && !ics_data.is_empty() {
        log::warn!("Parsed 0 events. ICS data size: {} bytes. First 100 chars: {:?}", 
            ics_data.len(), 
            ics_data.chars().take(100).collect::<String>());
    } else {
        log::info!("Parsed {} events from ICS data", events.len());
    }
    
    Ok(events)
}

fn convert_ics_event(ics_event: &IcsEvent) -> Result<CalendarEvent> {
    let title = ics_event.get_summary()
        .unwrap_or("Untitled Event")
        .to_string();
    
    let description = ics_event.get_description()
        .map(|s| s.to_string())
        .unwrap_or_default();
    
    // Extract video links from description
    let video_link = utils::extract_video_link(Some(&description), None);
    
    // Parse start and end times
    let start_time = ics_event.get_start()
        .as_ref()
        .and_then(common::parse_ical_datetime)
        .unwrap_or_else(Utc::now);
    
    let end_time = ics_event.get_end()
        .as_ref()
        .and_then(common::parse_ical_datetime)
        .unwrap_or_else(|| start_time + chrono::Duration::hours(1));
    
    // Generate unique ID from UID or create one
    let external_id = ics_event.get_uid()
        .map(|uid| uid.to_string())
        .unwrap_or_else(|| {
            // Create a hash from title and start time as fallback
            let mut hasher = DefaultHasher::new();
            format!("{}{}", title, start_time.timestamp()).hash(&mut hasher);
            format!("proton-{:x}", hasher.finish())
        });
    
    // Extract location if available (not used in current model)
    let _location = ics_event.get_location()
        .map(|s| s.to_string())
        .unwrap_or_default();
    
    Ok(CalendarEvent {
        id: None,
        external_id,
        title,
        description: Some(description),
        start_time,
        end_time,
        video_link: video_link.as_ref().map(|info| info.url.clone()),
        video_platform: video_link.map(|info| info.platform.clone()),
        snooze_count: 0,
        has_alerted: false,
        last_alert_threshold: None,
        is_dismissed: false,
        account_id: 0, // Will be set when storing
        created_at: Utc::now(),
        updated_at: Utc::now(),
    })
}

async fn store_event(event: &CalendarEvent, account_id: i64, pool: &SqlitePool) -> Result<bool> {
    // Check if event already exists
    let existing_event = sqlx::query_as::<_, CalendarEvent>(
        "SELECT id, external_id, account_id, title, description, start_time, end_time, video_link, video_platform, snooze_count, has_alerted, last_alert_threshold, is_dismissed, created_at, updated_at FROM events WHERE external_id = ? AND account_id = ?"
    )
    .bind(&event.external_id)
    .bind(account_id)
    .fetch_optional(pool)
    .await?;
    
    match existing_event {
        Some(existing) => {
            // Update existing event if it has changed
            if existing.title != event.title || 
               existing.description != event.description ||
               existing.start_time != event.start_time ||
               existing.end_time != event.end_time ||
               existing.video_link != event.video_link {
                
                sqlx::query(
                    "UPDATE events SET title = ?, description = ?, start_time = ?, end_time = ?, 
                     video_link = ?, video_platform = ?, updated_at = ? WHERE id = ?"
                )
                .bind(&event.title)
                .bind(&event.description)
                .bind(event.start_time)
                .bind(event.end_time)
                .bind(&event.video_link)
                .bind(&event.video_platform)
                .bind(Utc::now())
                .bind(existing.id)
                .execute(pool)
                .await?;
                
                log::debug!("Updated event: {}", event.title);
                Ok(false) // Updated, not added
            } else {
                Ok(false) // No changes
            }
        }
        None => {
            // Insert new event
            sqlx::query(
                "INSERT INTO events (external_id, title, description, start_time, end_time, 
                 video_link, video_platform, account_id, created_at, updated_at) 
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&event.external_id)
            .bind(&event.title)
            .bind(&event.description)
            .bind(event.start_time)
            .bind(event.end_time)
            .bind(&event.video_link)
            .bind(&event.video_platform)
            .bind(account_id)
            .bind(Utc::now())
            .bind(Utc::now())
            .execute(pool)
            .await?;
            
            log::debug!("Added new event: {}", event.title);
            Ok(true) // Added
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_ics_url_valid_https() {
        // Valid HTTPS URLs
        assert!(common::validate_ics_url_format("https://calendar.proton.me/api/calendar/v1/url/abc123/calendar.ics").is_ok());
        assert!(common::validate_ics_url_format("https://example.com/path/to/calendar.ics").is_ok());
        assert!(common::validate_ics_url_format("https://calendar.google.com/calendar/ical/user@example.com/public/basic.ics").is_ok());
    }

    #[test]
    fn test_validate_ics_url_rejects_http() {
        // HTTP should be rejected for security
        let result = common::validate_ics_url_format("http://calendar.proton.me/calendar.ics");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("HTTPS"));
    }

    #[test]
    fn test_validate_ics_url_rejects_empty() {
        // Empty URL should be rejected
        let result = common::validate_ics_url_format("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));

        // Whitespace-only URL should be rejected
        let result = common::validate_ics_url_format("   ");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_validate_ics_url_rejects_invalid_format() {
        // Invalid URL format
        let result = common::validate_ics_url_format("not-a-url");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid ICS URL format"));

        // Missing scheme
        let result = common::validate_ics_url_format("calendar.proton.me/calendar.ics");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_ics_url_rejects_localhost() {
        // Localhost should be rejected for security
        let result = common::validate_ics_url_format("https://localhost/calendar.ics");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("localhost"));

        // 127.0.0.1 should be rejected
        let result = common::validate_ics_url_format("https://127.0.0.1/calendar.ics");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_ics_url_rejects_local_network() {
        // Local network addresses should be rejected
        let test_cases = vec![
            "https://192.168.1.1/calendar.ics",
            "https://10.0.0.1/calendar.ics",
            "https://172.16.0.1/calendar.ics",
        ];

        for url in test_cases {
            let result = common::validate_ics_url_format(url);
            assert!(result.is_err(), "Should reject local network URL: {}", url);
            assert!(result.unwrap_err().to_string().contains("local network"));
        }
    }

    #[test]
    fn test_validate_ics_url_malformed() {
        // Malformed URLs should be rejected
        let test_cases = vec![
            "https://",           // Missing domain
            "https:// /path",     // Space in URL
            "https://exa mple.com/calendar.ics", // Space in domain
        ];

        for url in test_cases {
            let result = common::validate_ics_url_format(url);
            assert!(result.is_err(), "Should reject malformed URL: {}", url);
        }
    }

    #[test]
    fn test_validate_ics_url_accepts_various_domains() {
        // Various valid calendar service domains
        let valid_urls = vec![
            "https://calendar.proton.me/api/calendar/v1/url/secret/calendar.ics",
            "https://outlook.office365.com/owa/calendar/123/calendar.ics",
            "https://caldav.icloud.com/published/2/calendar.ics",
            "https://p01-calendars.icloud.com/published/2/calendar",
        ];

        for url in valid_urls {
            assert!(
                common::validate_ics_url_format(url).is_ok(),
                "Should accept valid URL: {}",
                url
            );
        }
    }

    #[test]
    fn test_validate_ics_url_warns_missing_path() {
        // URL with no path should still pass but log warning
        // (We can't test the log warning directly, but it should not error)
        let result = common::validate_ics_url_format("https://example.com");
        assert!(result.is_ok());

        let result = common::validate_ics_url_format("https://example.com/");
        assert!(result.is_ok());
    }
}