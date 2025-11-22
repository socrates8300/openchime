#![allow(dead_code)]
// Proton Calendar integration via ICS feed
// Handles ICS fetching and parsing

use crate::models::{Account, CalendarEvent, SyncResult};
use crate::utils;
use crate::utils::{logging, circuit_breaker::get_circuit_breaker};
use crate::utils::retry::RetryConfig;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc, TimeZone, Datelike};
use icalendar::{Component, Event as IcsEvent, EventLike, Calendar as IcsCalendar};
use reqwest::Client;
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
    let ics_data = fetch_ics_data(ics_url).await?;
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
    
    match fetch_ics_data(ics_url).await {
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
    match fetch_ics_data(ics_url).await {
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

async fn fetch_ics_data(ics_url: &str) -> Result<String> {
    let retry_config = RetryConfig {
        max_attempts: 3,
        base_delay: std::time::Duration::from_millis(1000),
        max_delay: std::time::Duration::from_secs(20),
        backoff_multiplier: 2.0,
    };
    
    let circuit_breaker = get_circuit_breaker("proton_calendar").await;
    let ics_url_str = ics_url.to_string();
    
    circuit_breaker.execute(move || {
        let config = retry_config.clone();
        let url = ics_url_str.clone();
        
        async move {
            utils::retry::retry_with_exponential_backoff(&config, move || {
                let inner_url = url.clone();
                Box::pin(async move {
                    let client = Client::builder()
                        .user_agent("OpenChime/1.0")
                        .timeout(std::time::Duration::from_secs(30))
                        .build()
                        .map_err(|e| anyhow!("Failed to build client: {}", e))?;
                    
                    let response = client.get(&inner_url).send().await
                        .map_err(|e| anyhow!("Request failed: {}", e))?;
                    
                    if !response.status().is_success() {
                        let status = response.status();
                        let text = response.text().await.unwrap_or_else(|_| "Unable to read error response".to_string());
                        return Err(anyhow!("HTTP {}: {}", status, text));
                    }
                    
                    let content = response.text().await
                        .map_err(|e| anyhow!("Failed to read response body: {}", e))?;
                        
                    // Basic validation to catch HTML responses
                    if content.trim().starts_with("<!DOCTYPE") || content.trim().starts_with("<html") {
                        return Err(anyhow!("Invalid ICS URL: The server returned HTML instead of a calendar file. Please ensure you are using the 'Secret address in iCal format' from your calendar settings, not the web browser URL."));
                    }
                    
                    // Basic verification of ICS header
                    if !content.contains("BEGIN:VCALENDAR") {
                         log::warn!("Content does not contain BEGIN:VCALENDAR.");
                    }
                    
                    Ok(content)
                })
            }).await
        }
    }).await
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
        .and_then(parse_ical_datetime)
        .unwrap_or_else(Utc::now);
    
    let end_time = ics_event.get_end()
        .as_ref()
        .and_then(parse_ical_datetime)
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
            Some(Utc.with_ymd_and_hms(date.year(), date.month(), date.day(), 0, 0, 0).unwrap())
        }
    }
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