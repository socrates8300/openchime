// Simple test to debug sync issues
use reqwest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing ICS parsing directly...");
    
    // Test the same URL our app uses
    let url = "https://www.calendarlabs.com/ical-calendar/ics/76/US_Holidays.ics";
    
    println!("Fetching ICS from: {}", url);
    
    let client = reqwest::Client::new();
    let response = client.get(url).send().await?;
    
    if !response.status().is_success() {
        return Err(format!("HTTP {}: {}", response.status()).into());
    }
    
    let ics_data = response.text().await?;
    println!("Fetched {} bytes", ics_data.len());
    
    // Parse using the same library our app uses
    let calendar = icalendar::Calendar::from_str(&ics_data)?;
    println!("Parsed calendar with {} components", calendar.components.len());
    
    let mut event_count = 0;
    for component in calendar.components {
        if let Some(ics_event) = component.as_event() {
            event_count += 1;
            
            let title = ics_event.get_summary().unwrap_or("Untitled");
            let start = ics_event.get_start();
            let end = ics_event.get_end();
            
            println!("Event {}: {}", event_count, title);
            println!("  Start: {:?}", start);
            println!("  End: {:?}", end);
            
            if let Some(dt) = start.as_ref() {
                match dt {
                    icalendar::DatePerhapsTime::DateTime(dt) => {
                        match dt {
                            icalendar::CalendarDateTime::Utc(dt) => {
                                println!("  Parsed UTC: {}", dt.naive_utc().and_utc());
                            }
                            icalendar::CalendarDateTime::Floating(dt) => {
                                println!("  Parsed Floating: {}", dt.and_utc());
                            }
                            icalendar::CalendarDateTime::WithTimezone { date_time, .. } => {
                                println!("  Parsed WithTimezone: {}", date_time.and_utc());
                            }
                        }
                    }
                    icalendar::DatePerhapsTime::Date(date) => {
                        println!("  Date-only: {}", date);
                        let parsed = chrono::Utc.with_ymd_and_hms(date.year(), date.month(), date.day(), 0, 0, 0);
                        println!("  Parsed date-only: {}", parsed);
                    }
                }
            }
        }
    }
    
    println!("Total events found: {}", event_count);
    Ok(())
}