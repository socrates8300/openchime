#![allow(dead_code)]
use crate::utils::circuit_breaker::get_circuit_breaker;
use crate::utils::retry::RetryConfig;
use crate::utils;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc, TimeZone, Datelike};
use reqwest::Client;
use std::str::FromStr;
use url::Url;

/// Validates an ICS URL for security and format correctness
pub fn validate_ics_url_format(ics_url: &str) -> Result<()> {
    // Check for empty or whitespace-only URL
    if ics_url.trim().is_empty() {
        return Err(anyhow!(
            "ICS URL cannot be empty. Please provide a valid calendar ICS URL."
        ));
    }

    // Parse the URL to validate its structure
    let parsed_url = Url::parse(ics_url).map_err(|e| {
        anyhow!(
            "Invalid ICS URL format: {}. Please ensure the URL is properly formatted (e.g., https://calendar.example.com/path/calendar.ics)",
            e
        )
    })?;

    // Enforce HTTPS for security
    if parsed_url.scheme() != "https" {
        return Err(anyhow!(
            "ICS URL must use HTTPS protocol for security. HTTP is not allowed. \
             Your URL starts with '{}://'. Please use an HTTPS URL instead.",
            parsed_url.scheme()
        ));
    }

    // Validate that a domain is present
    let domain = parsed_url.host_str().ok_or_else(|| {
        anyhow!(
            "ICS URL must have a valid domain name. The provided URL '{}' does not contain a valid host.",
            ics_url
        )
    })?;

    // Check for obviously invalid or suspicious domains
    if domain.is_empty() {
        return Err(anyhow!("ICS URL domain cannot be empty."));
    }

    // Reject localhost and local network addresses for security
    if domain == "localhost"
        || domain.starts_with("127.")
        || domain.starts_with("192.168.")
        || domain.starts_with("10.")
        || domain.starts_with("172.16.") {
        return Err(anyhow!(
            "ICS URL cannot point to localhost or local network addresses. \
             Please use a publicly accessible calendar URL."
        ));
    }

    // Validate that a path is present (ICS URLs should have a path component)
    let path = parsed_url.path();
    if path.is_empty() || path == "/" {
        log::warn!(
            "ICS URL has no path component. This may not be a valid calendar feed URL: {}",
            ics_url
        );
    }

    // Optional: Check if the path looks like an ICS file
    if !path.to_lowercase().ends_with(".ics") && !path.contains("/calendar") {
        log::warn!(
            "ICS URL path does not appear to be a calendar feed (expected .ics extension or /calendar path): {}",
            ics_url
        );
    }

    Ok(())
}

/// Fetch ICS data from URL with retry logic and circuit breaker
pub async fn fetch_ics_data(ics_url: &str, circuit_breaker_name: &str) -> Result<String> {
    let retry_config = RetryConfig {
        max_attempts: 3,
        base_delay: std::time::Duration::from_millis(1000),
        max_delay: std::time::Duration::from_secs(20),
        backoff_multiplier: 2.0,
    };
    
    let circuit_breaker = get_circuit_breaker(circuit_breaker_name).await;
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
                        
                    // TODO: For very large ICS files, consider streaming the response
                    // instead of loading the entire body into memory.
                    // Current icalendar crate requires full string, so this would need a streaming parser.
                        
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

/// Parse ICS datetime with proper timezone conversion
pub fn parse_ical_datetime(dt: &icalendar::DatePerhapsTime) -> Option<DateTime<Utc>> {
    match dt {
        icalendar::DatePerhapsTime::DateTime(dt) => {
            match dt {
                // Already in UTC - no conversion needed
                icalendar::CalendarDateTime::Utc(dt) => Some(dt.naive_utc().and_utc()),

                // Floating time (no timezone specified) - interpret as local system time
                icalendar::CalendarDateTime::Floating(naive_dt) => {
                    chrono::Local
                        .from_local_datetime(naive_dt)
                        .single()
                        .map(|local| local.with_timezone(&Utc))
                }

                // Time with explicit timezone - convert to UTC properly
                icalendar::CalendarDateTime::WithTimezone { date_time, tzid } => {
                    // Try to parse the timezone using chrono-tz
                    if let Ok(tz) = chrono_tz::Tz::from_str(tzid) {
                        tz.from_local_datetime(date_time)
                            .single()
                            .map(|zoned| zoned.with_timezone(&Utc))
                    } else {
                        // Fallback: if timezone not recognized, log warning and treat as local
                        log::warn!("Unrecognized timezone '{}', treating as local time", tzid);
                        chrono::Local
                            .from_local_datetime(date_time)
                            .single()
                            .map(|local| local.with_timezone(&Utc))
                    }
                }
            }
        }
        icalendar::DatePerhapsTime::Date(date) => {
            // For date-only events, assume start of day in local timezone, then convert to UTC
            chrono::Local
                .with_ymd_and_hms(date.year(), date.month(), date.day(), 0, 0, 0)
                .single()
                .map(|local| local.with_timezone(&Utc))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, NaiveDate, NaiveDateTime, NaiveTime};
    use icalendar::{DatePerhapsTime, CalendarDateTime};

    #[test]
    fn test_validate_ics_url_format_valid() {
        let url = "https://calendar.google.com/calendar/ical/user/private/basic.ics";
        assert!(validate_ics_url_format(url).is_ok());
    }

    #[test]
    fn test_validate_ics_url_format_invalid_scheme() {
        let url = "http://calendar.example.com/basic.ics";
        let result = validate_ics_url_format(url);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("HTTPS"));
    }

    #[test]
    fn test_validate_ics_url_format_empty() {
        let url = "   ";
        let result = validate_ics_url_format(url);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_validate_ics_url_format_localhost() {
        let url = "https://localhost/calendar.ics";
        let result = validate_ics_url_format(url);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("localhost"));
    }

    #[test]
    fn test_validate_ics_url_format_private_ip() {
        let url = "https://192.168.1.1/calendar.ics";
        let result = validate_ics_url_format(url);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("local network"));
    }

    #[test]
    fn test_parse_ical_datetime_utc() {
        let naive = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap().and_hms_opt(12, 0, 0).unwrap();
        let utc_dt = Utc.from_utc_datetime(&naive);
        let dt = DatePerhapsTime::DateTime(CalendarDateTime::Utc(utc_dt));
        
        let result = parse_ical_datetime(&dt);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), utc_dt);
    }

    #[test]
    fn test_parse_ical_datetime_floating() {
        // Floating time should be interpreted as local, then converted to UTC
        let naive = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap().and_hms_opt(12, 0, 0).unwrap();
        let dt = DatePerhapsTime::DateTime(CalendarDateTime::Floating(naive));
        
        let result = parse_ical_datetime(&dt);
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_ical_datetime_with_timezone() {
        let naive = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap().and_hms_opt(12, 0, 0).unwrap();
        let dt = DatePerhapsTime::DateTime(CalendarDateTime::WithTimezone { 
            date_time: naive, 
            tzid: "America/New_York".to_string() 
        });
        
        let result = parse_ical_datetime(&dt);
        assert!(result.is_some());
        // 12:00 NY is 17:00 UTC
        let expected = Utc.with_ymd_and_hms(2023, 1, 1, 17, 0, 0).unwrap();
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_parse_ical_datetime_date_only() {
        let date = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let dt = DatePerhapsTime::Date(date);
        
        let result = parse_ical_datetime(&dt);
        assert!(result.is_some());
    }
}
