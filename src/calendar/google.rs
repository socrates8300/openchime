#![allow(dead_code)]
#![allow(unused_imports)]

// Google Calendar integration via ICS feed
// Handles ICS fetching and parsing (OAuth removed - ICS-only now)

use crate::models::{Account, SyncResult, CalendarEvent};
use crate::utils::logging;
use crate::calendar::common;
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc, TimeZone, Datelike};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use std::str::FromStr;
use icalendar::{Component, Event as IcsEvent, EventLike, Calendar as IcsCalendar};

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

pub async fn sync_google_calendar(account: &Account, db: &sqlx::SqlitePool) -> Result<SyncResult> {
    let start_time = Instant::now();
    log::info!("Starting Google calendar sync for account: {}", account.account_name);

    // Google Calendar integration now uses ICS feed only
    let events = sync_google_ics(account).await?;

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
    logging::log_auth_event("Google Calendar ICS connection test", &account.account_name);

    let ics_url = &account.auth_data;

    match common::fetch_ics_data(ics_url, "google_calendar").await {
        Ok(_) => {
            log::info!("Google ICS connection successful for: {}", account.account_name);
            Ok(true)
        }
        Err(e) => {
            log::warn!("Google ICS connection failed for {}: {}", account.account_name, e);
            Ok(false)
        }
    }
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
        created_at: Utc::now(),
        updated_at: Utc::now(),
    })
}

/// Handle Google Calendar sync via ICS URL
async fn sync_google_ics(account: &Account) -> Result<Vec<GoogleCalendarEvent>> {
    let ics_url = &account.auth_data;
    log::info!("Fetching Google ICS data from URL: {}", ics_url);

    // Fetch ICS data
    let ics_data = common::fetch_ics_data(ics_url, "google_calendar").await?;
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

/// Parse ICS data to Google Calendar events
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
        .and_then(common::parse_ical_datetime)
        .map(|dt| dt.with_timezone(&Utc));
    let end_time = ics_event.get_end()
        .as_ref()
        .and_then(common::parse_ical_datetime)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_video_info_google_meet() {
        let desc = Some("Join with Google Meet: https://meet.google.com/abc-defg-hij".to_string());
        let (link, platform) = extract_video_info(&desc);
        assert_eq!(link, desc);
        assert_eq!(platform, Some("Google Meet".to_string()));
    }

    #[test]
    fn test_extract_video_info_zoom() {
        let desc = Some("Zoom Meeting: https://zoom.us/j/123456789".to_string());
        let (link, platform) = extract_video_info(&desc);
        assert_eq!(link, desc);
        assert_eq!(platform, Some("Zoom".to_string()));
    }

    #[test]
    fn test_extract_video_info_teams() {
        let desc = Some("Microsoft Teams Meeting\nhttps://teams.microsoft.com/l/meetup-join/...".to_string());
        let (link, platform) = extract_video_info(&desc);
        assert_eq!(link, desc);
        assert_eq!(platform, Some("Microsoft Teams".to_string()));
    }

    #[test]
    fn test_extract_video_info_none() {
        let desc = Some("Regular meeting".to_string());
        let (link, platform) = extract_video_info(&desc);
        assert_eq!(link, None);
        assert_eq!(platform, None);
    }

    #[test]
    fn test_extract_video_info_empty() {
        let desc = None;
        let (link, platform) = extract_video_info(&desc);
        assert_eq!(link, None);
        assert_eq!(platform, None);
    }
}
