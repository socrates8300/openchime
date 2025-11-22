#![allow(dead_code)]
use crate::models::VideoMeetingInfo;
use regex::Regex;

pub mod retry;
pub mod logging;
pub mod circuit_breaker;

pub fn extract_video_link(description: Option<&str>, location: Option<&str>) -> Option<VideoMeetingInfo> {
    let combined_text = format!("{} {}", description.unwrap_or(""), location.unwrap_or(""));
    
    // Video platform patterns
    let patterns = vec![
        // Zoom
        (r"https://.*zoom\.us/j/(\d+)", "Zoom"),
        (r"https://.*zoom\.us/my/([^\\s]+)", "Zoom"),
        (r"https://.*zoom\.us/s/([^\\s]+)", "Zoom"),
        
        // Google Meet
        (r"https://meet\.google\.com/([a-z-]+)", "Google Meet"),
        
        // Microsoft Teams
        (r"https://teams\.microsoft\.com/l/meetup-join/([^\\s]+)", "Teams"),
        (r"https://teams\.live\.com/([^\\s]+)", "Teams"),
        
        // Webex
        (r"https://.*webex\.com/([^\\s]+)", "Webex"),
        (r"https://.*webex\.com/join/([^\\s]+)", "Webex"),
        
        // Skype
        (r"https://join\.skype\.com/([^\\s]+)", "Skype"),
        
        // GoToMeeting
        (r"https://.*gotomeeting\.com/([^\\s]+)", "GoToMeeting"),
        
        // BlueJeans
        (r"https://.*bluejeans\.com/([^\\s]+)", "BlueJeans"),
        
        // RingCentral
        (r"https://.*ringcentral\.com/([^\\s]+)", "RingCentral"),
        
        // Whereby
        (r"https://.*whereby\.com/([^\\s]+)", "Whereby"),
        
        // Jitsi
        (r"https://.*jitsi\.org/([^\\s]+)", "Jitsi"),
        (r"https://meet\.jit\.si/([^\\s]+)", "Jitsi"),
        
        // Discord
        (r"https://discord\.gg/([^\\s]+)", "Discord"),
        (r"https://.*discord\.com/channels/([^\\s]+)", "Discord"),
        
        // Slack
        (r"https://.*slack\.com/archives/([^\\s]+)", "Slack"),
        (r"https://app\.slack\.com/meet/([^\\s]+)", "Slack"),
        
        // FaceTime (iOS links)
        (r"facetime://([^\\s]+)", "FaceTime"),
        (r"facetime-audio://([^\\s]+)", "FaceTime"),
        
        // Zoom alternative patterns
        (r"zoom\.us/j/(\d+)", "Zoom"),
        (r"zoom\.us/my/([^\\s]+)", "Zoom"),
        
        // Generic patterns
        (r"https://([^\\s]*meet[^\\s]*)", "Meeting"),
        (r"https://([^\\s]*call[^\\s]*)", "Meeting"),
        (r"https://([^\\s]*video[^\\s]*)", "Meeting"),
    ];
    
    for (pattern, platform) in patterns {
        if let Ok(regex) = Regex::new(pattern) {
            if let Some(captures) = regex.captures(&combined_text) {
                let full_match = captures.get(0).unwrap().as_str().to_string();
                let meeting_id = captures.get(1).map(|m| m.as_str().to_string());
                
                return Some(VideoMeetingInfo {
                    platform: platform.to_string(),
                    url: full_match,
                    meeting_id,
                    password: None, // Could be enhanced to extract passwords
                });
            }
        }
    }
    
    None
}

pub fn extract_meeting_password(text: &str) -> Option<String> {
    // Common password patterns
    let password_patterns = vec![
        r"password[:\s]+([A-Za-z0-9]+)",
        r"pwd[:\s]+([A-Za-z0-9]+)",
        r"pass[:\s]+([A-Za-z0-9]+)",
        r"code[:\s]+([A-Za-z0-9]+)",
        r"pin[:\s]+([A-Za-z0-9]+)",
    ];
    
    for pattern in password_patterns {
        if let Ok(regex) = Regex::new(pattern) {
            if let Some(captures) = regex.captures(text) {
                if let Some(password) = captures.get(1) {
                    return Some(password.as_str().to_string());
                }
            }
        }
    }
    
    None
}

pub fn is_all_day_event(start_time: chrono::DateTime<chrono::Utc>, end_time: chrono::DateTime<chrono::Utc>) -> bool {
    let duration = end_time - start_time;
    duration.num_hours() >= 24
}

pub fn normalize_title(title: &str) -> String {
    title.trim().to_string()
}

pub fn extract_meeting_keywords(title: &str, description: Option<&str>) -> Vec<String> {
    let combined = format!("{} {}", title, description.unwrap_or("")).to_lowercase();
    let mut keywords = Vec::new();
    
    let meeting_keywords = vec![
        "meeting", "call", "conference", "sync", "standup", "review",
        "demo", "presentation", "interview", "check-in", "checkin",
        "retrospective", "retro", "planning", "grooming", "refinement",
        "kickoff", "kick-off", "training", "workshop", "webinar",
        "all-hands", "all hands", "townhall", "town hall", "q&a", "qa",
    ];
    
    for keyword in meeting_keywords {
        if combined.contains(keyword) {
            keywords.push(keyword.to_string());
        }
    }
    
    keywords
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extract_zoom_link() {
        let description = Some("Join us for weekly sync");
        let location = Some("https://zoom.us/j/123456789");
        
        let result = extract_video_link(description, location);
        assert!(result.is_some());
        assert_eq!(result.unwrap().platform, "Zoom");
    }
    
    #[test]
    fn test_extract_google_meet() {
        let description = Some("Meeting link: https://meet.google.com/abc-def-xyz");
        let location = Some("");
        
        let result = extract_video_link(description, location);
        assert!(result.is_some());
        assert_eq!(result.unwrap().platform, "Google Meet");
    }
    
    #[test]
    fn test_no_video_link() {
        let description = Some("Regular team meeting");
        let location = Some("Conference Room A");
        
        let result = extract_video_link(description, location);
        assert!(result.is_none());
    }
    
    #[test]
    fn test_extract_google_meet_str() {
        let description = Some("Meeting link: https://meet.google.com/abc-def-xyz");
        let location = Some("");
        
        let result = extract_video_link(description, location);
        assert!(result.is_some());
        assert_eq!(result.unwrap().platform, "Google Meet");
    }
    
    #[test]
    fn test_no_video_link_str() {
        let description = Some("Regular team meeting");
        let location = Some("Conference Room A");
        
        let result = extract_video_link(description, location);
        assert!(result.is_none());
    }
    
    #[test]
    fn test_extract_password() {
        let text = "password: 123456";
        let result = extract_meeting_password(text);
        assert_eq!(result.unwrap(), "123456");
    }
}