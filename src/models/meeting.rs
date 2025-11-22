#![allow(dead_code)]
// file: src/meeting.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoMeetingInfo {
    pub platform: String,
    pub url: String,
    pub meeting_id: Option<String>,
    pub password: Option<String>,
}

impl VideoMeetingInfo {
    pub fn new(platform: String, url: String) -> Self {
        Self {
            platform,
            url,
            meeting_id: None,
            password: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_video_meeting_info_new() {
        let info = VideoMeetingInfo::new(
            "Zoom".to_string(),
            "https://zoom.us/j/123456".to_string(),
        );

        assert_eq!(info.platform, "Zoom");
        assert_eq!(info.url, "https://zoom.us/j/123456");
        assert!(info.meeting_id.is_none());
        assert!(info.password.is_none());
    }
}
