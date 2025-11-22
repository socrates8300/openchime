use openchime::{AudioManager, AlertType};
use tempfile::TempDir;
use std::fs::File;

#[tokio::test]
async fn test_audio_manager_full_workflow() {
    let manager = AudioManager::new().unwrap();
    
    // Test initial state
    assert_eq!(manager.get_volume(), 0.7);
    
    // Test volume changes
    manager.set_volume(0.5).unwrap();
    assert_eq!(manager.get_volume(), 0.5);
    
    // Test volume boundaries
    manager.set_volume(1.2).unwrap();
    assert_eq!(manager.get_volume(), 1.0);
    
    manager.set_volume(-0.1).unwrap();
    assert_eq!(manager.get_volume(), 0.0);
    
    // Test audio playback (should not panic even without sound files)
    manager.play_alert(AlertType::Meeting).unwrap();
    manager.play_alert(AlertType::VideoMeeting).unwrap();
    manager.play_alert(AlertType::Test).unwrap();
    
    // Test audio test
    manager.test_audio().unwrap();
}

#[tokio::test]
async fn test_sound_file_configuration() {
    let manager = AudioManager::new().unwrap();
    let temp_dir = TempDir::new().unwrap();
    
    // Create mock sound files
    let meeting_sound = temp_dir.path().join("meeting.wav");
    let video_sound = temp_dir.path().join("video.wav");
    let test_sound = temp_dir.path().join("test.wav");
    
    use std::io::Write; // Re-add for file writing in test
    
    // Write minimal WAV file headers (44 bytes)
    for path in [&meeting_sound, &video_sound, &test_sound] {
        let mut file = File::create(path).unwrap();
        file.write_all(&[0; 44]).unwrap(); // Minimal WAV header
    }
    
    let sound_files = openchime::SoundFiles {
        meeting_alert: meeting_sound,
        video_meeting_alert: video_sound,
        test_sound: test_sound,
    };
    
    manager.update_sound_files(sound_files).unwrap();
    
    // Test playback with actual files
    manager.play_alert(AlertType::Meeting).unwrap();
    manager.play_alert(AlertType::VideoMeeting).unwrap();
    manager.play_alert(AlertType::Test).unwrap();
}

#[tokio::test]
async fn test_audio_manager_concurrent_access() {
    let manager = std::sync::Arc::new(AudioManager::new().unwrap());
    let mut handles = vec![];
    
    // Test concurrent volume changes
    for i in 0..10 {
        let manager_clone = manager.clone();
        let handle = tokio::spawn(async move {
            let volume = (i as f32) / 10.0;
            manager_clone.set_volume(volume).unwrap();
            manager_clone.get_volume()
        });
        handles.push(handle);
    }
    
    // Wait for all operations to complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result >= 0.0 && result <= 1.0);
    }
}

#[tokio::test]
async fn test_audio_directory_creation() {
    let sounds_dir = AudioManager::ensure_sound_directory().unwrap();
    
    assert!(sounds_dir.exists());
    assert!(sounds_dir.is_dir());
    
    // Test that it doesn't fail if directory already exists
    let sounds_dir2 = AudioManager::ensure_sound_directory().unwrap();
    assert_eq!(sounds_dir, sounds_dir2);
}

#[test]
fn test_sound_files_struct() {
    let temp_dir = TempDir::new().unwrap();
    
    let sound_files = openchime::SoundFiles {
        meeting_alert: temp_dir.path().join("meeting.wav"),
        video_meeting_alert: temp_dir.path().join("video.wav"),
        test_sound: temp_dir.path().join("test.wav"),
    };
    
    assert!(sound_files.meeting_alert.ends_with("meeting.wav"));
    assert!(sound_files.video_meeting_alert.ends_with("video.wav"));
    assert!(sound_files.test_sound.ends_with("test.wav"));
}

#[test]
fn test_alert_type_matching() {
    let meeting_type = openchime::audio::AlertType::Meeting;
    let video_type = openchime::audio::AlertType::VideoMeeting;
    let test_type = openchime::audio::AlertType::Test;
    
    // Test that all variants can be matched
    let all_types = vec![meeting_type, video_type, test_type];
    for alert_type in all_types {
        match alert_type {
            AlertType::Meeting => assert!(true),
            AlertType::VideoMeeting => assert!(true),
            AlertType::SnoozeReminder => assert!(true),
            AlertType::Test => assert!(true),
        }
    }
}

#[tokio::test]
async fn test_audio_error_handling() {
    let manager = AudioManager::new().unwrap();
    
    // Test with invalid volume values
    let result = manager.set_volume(f32::NAN);
    assert!(result.is_ok()); // Should clamp to valid range
    
    let result = manager.set_volume(f32::INFINITY);
    assert!(result.is_ok()); // Should clamp to valid range
    
    // Test with very large positive and negative values
    let result = manager.set_volume(1000.0);
    assert!(result.is_ok());
    assert_eq!(manager.get_volume(), 1.0);
    
    let result = manager.set_volume(-1000.0);
    assert!(result.is_ok());
    assert_eq!(manager.get_volume(), 0.0);
}