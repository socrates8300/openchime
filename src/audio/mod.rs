use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use log::{info, error, warn, debug};
use anyhow::{Result, Context};

#[derive(Clone)]
pub struct AudioManager {
    volume: Arc<Mutex<f32>>,
    sound_files: Arc<Mutex<SoundFiles>>,
}

#[derive(Debug, Clone)]
pub struct SoundFiles {
    pub meeting_alert: PathBuf,
    pub video_meeting_alert: PathBuf,
    pub test_sound: PathBuf,
    pub alert_30m: PathBuf,
    pub alert_10m: PathBuf,
    pub alert_5m: PathBuf,
    pub alert_1m: PathBuf,
}

pub use crate::models::AlertType;

impl AudioManager {
    pub fn new() -> Result<Self> {
        info!("Initializing audio system");
        
        let volume = Arc::new(Mutex::new(0.7)); // Default volume 70%
        let sound_files = Arc::new(Mutex::new(Self::default_sound_files()?));
        
        Ok(AudioManager {
            volume,
            sound_files,
        })
    }
    
    /// Create a dummy audio manager that does nothing
    /// Used when audio system initialization fails
    pub fn new_dummy() -> Self {
        warn!("Using dummy audio manager - audio features will be disabled");
        
        AudioManager {
            volume: Arc::new(Mutex::new(0.0)), // Silent by default
            sound_files: Arc::new(Mutex::new(SoundFiles {
                meeting_alert: PathBuf::new(),
                video_meeting_alert: PathBuf::new(),
                test_sound: PathBuf::new(),
                alert_30m: PathBuf::new(),
                alert_10m: PathBuf::new(),
                alert_5m: PathBuf::new(),
                alert_1m: PathBuf::new(),
            })),
        }
    }
    
    fn default_sound_files() -> Result<SoundFiles> {
        // Use absolute path to project root alarms for now if dirs fail, but we want to be portable.
        // Assuming we run from project root in dev.
        // But for release, we construct it.
        
        let app_data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("openchime");
        
        let sounds_dir = app_data_dir.join("sounds");
        // Also check ./alarms for development
        let dev_alarms = PathBuf::from("alarms");
        
        let resolve = |name: &str, dev_name: &str| {
            if dev_alarms.join(dev_name).exists() {
                dev_alarms.join(dev_name)
            } else {
                sounds_dir.join(name)
            }
        };
        
        Ok(SoundFiles {
            meeting_alert: resolve("meeting_alert.wav", "5_minutes.mp3"), // Default to 5m sound?
            video_meeting_alert: resolve("video_meeting_alert.wav", "1_minutes.mp3"),
            test_sound: resolve("test_sound.wav", "1_minutes.mp3"),
            alert_30m: resolve("30m.mp3", "30_minutes.mp3"),
            alert_10m: resolve("10m.mp3", "10_minutes.mp3"),
            alert_5m: resolve("5m.mp3", "5_minutes.mp3"),
            alert_1m: resolve("1m.mp3", "1_minutes.mp3"),
        })
    }
    
    pub fn set_volume(&self, volume: f32) -> Result<()> {
        let vol = volume.clamp(0.0, 1.0);
        *self.volume.lock().unwrap() = vol;
        info!("Set audio volume to {:.0}%", vol * 100.0);
        Ok(())
    }
    
    pub fn get_volume(&self) -> f32 {
        *self.volume.lock().unwrap()
    }
    
    pub fn update_sound_files(&self, sound_files: SoundFiles) -> Result<()> {
        *self.sound_files.lock().unwrap() = sound_files;
        info!("Updated sound file paths");
        Ok(())
    }
    
    pub fn play_alert(&self, alert_type: AlertType) -> Result<()> {
        let sound_files = self.sound_files.lock().unwrap();
        let sound_path = match alert_type {
            AlertType::Meeting => &sound_files.meeting_alert,
            AlertType::VideoMeeting => &sound_files.video_meeting_alert,
            AlertType::SnoozeReminder => &sound_files.meeting_alert, // Use meeting sound for snooze
            AlertType::Test => &sound_files.test_sound,
            AlertType::Warning30m => &sound_files.alert_30m,
            AlertType::Warning10m => &sound_files.alert_10m,
            AlertType::Warning5m => &sound_files.alert_5m,
            AlertType::Warning1m => &sound_files.alert_1m,
        };
        
        let volume = *self.volume.lock().unwrap();
        let sound_path = sound_path.clone();
        
        tokio::task::spawn_blocking(move || {
            if let Err(e) = Self::play_sound_file(&sound_path, volume) {
                error!("Failed to play sound {:?}: {}", sound_path, e);
            }
        });
        
        Ok(())
    }
    
    fn play_sound_file(
        sound_path: &Path,
        volume: f32,
    ) -> Result<()> {
        // Create output stream on each call (OutputStream is not Send + Sync)
        let (stream, stream_handle) = OutputStream::try_default()
            .context("Failed to create audio output stream")?;
        
        if !sound_path.exists() {
            warn!("Sound file does not exist: {:?}", sound_path);
            return Self::play_default_sound(&stream_handle, volume);
        }
        
        debug!("Playing sound file: {:?}", sound_path);
        
        let file = File::open(sound_path)
            .context("Failed to open sound file")?;
        let reader = BufReader::new(file);
        
        let source = Decoder::new(reader)?
            .convert_samples::<f32>()
            .amplify(volume);
        
        let sink = Sink::try_new(&stream_handle)?;
        sink.append(source);
        
        // Wait for the sound to finish playing
        sink.sleep_until_end();
        
        // Keep stream alive until sound finishes
        drop(stream);
        
        Ok(())
    }
    
    fn play_default_sound(stream_handle: &OutputStreamHandle, volume: f32) -> Result<()> {
        warn!("Playing default sine wave tone (no sound file found)");
        
        // Generate a simple sine wave as fallback using rodio's SineWave
        let source = rodio::source::SineWave::new(440.0) // A4 note
            .take_duration(Duration::from_millis(500))
            .amplify(volume * 0.3); // Lower volume for sine wave
        
        let sink = Sink::try_new(stream_handle)?;
        sink.append(source);
        
        sink.sleep_until_end();
        
        Ok(())
    }
    
    pub fn test_audio(&self) -> Result<()> {
        info!("Testing audio system");
        self.play_alert(AlertType::Test)
    }
    
    pub fn ensure_sound_directory() -> Result<PathBuf> {
        let app_data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("openchime");
        
        let sounds_dir = app_data_dir.join("sounds");
        
        if !sounds_dir.exists() {
            std::fs::create_dir_all(&sounds_dir)
                .context("Failed to create sounds directory")?;
            info!("Created sounds directory: {:?}", sounds_dir);
        }
        
        Ok(sounds_dir)
    }
}

// Add the missing dirs dependency
impl Default for SoundFiles {
    fn default() -> Self {
        let app_data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("openchime");
        
        let sounds_dir = app_data_dir.join("sounds");
        
        SoundFiles {
            meeting_alert: sounds_dir.join("meeting_alert.wav"),
            video_meeting_alert: sounds_dir.join("video_meeting_alert.wav"),
            test_sound: sounds_dir.join("test_sound.wav"),
            alert_30m: sounds_dir.join("30m.mp3"),
            alert_10m: sounds_dir.join("10m.mp3"),
            alert_5m: sounds_dir.join("5m.mp3"),
            alert_1m: sounds_dir.join("1m.mp3"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use tempfile::TempDir;

    #[test]
    fn test_sound_files_struct() {
        let _temp_dir = TempDir::new().unwrap();
        
        let sound_files = crate::audio::SoundFiles {
            meeting_alert: _temp_dir.path().join("meeting.wav"),
            video_meeting_alert: _temp_dir.path().join("video.wav"),
            test_sound: _temp_dir.path().join("test.wav"),
            alert_30m: _temp_dir.path().join("30.wav"),
            alert_10m: _temp_dir.path().join("10.wav"),
            alert_5m: _temp_dir.path().join("5.wav"),
            alert_1m: _temp_dir.path().join("1.wav"),
        };
        
        assert!(sound_files.meeting_alert.ends_with("meeting.wav"));
        assert!(sound_files.video_meeting_alert.ends_with("video.wav"));
        assert!(sound_files.test_sound.ends_with("test.wav"));
        assert!(sound_files.alert_30m.ends_with("30.wav"));
    }

    #[test]
    fn test_set_volume() {
        let manager = AudioManager::new().unwrap();
        
        manager.set_volume(0.5).unwrap();
        assert_eq!(manager.get_volume(), 0.5);
        
        // Test volume clamping
        manager.set_volume(1.5).unwrap();
        assert_eq!(manager.get_volume(), 1.0);
        
        manager.set_volume(-0.5).unwrap();
        assert_eq!(manager.get_volume(), 0.0);
    }

    #[test]
    fn test_update_sound_files() {
        let manager = AudioManager::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        
        let new_sound_files = SoundFiles {
            meeting_alert: temp_dir.path().join("meeting.wav"),
            video_meeting_alert: temp_dir.path().join("video.wav"),
            test_sound: temp_dir.path().join("test.wav"),
            alert_30m: temp_dir.path().join("30.wav"),
            alert_10m: temp_dir.path().join("10.wav"),
            alert_5m: temp_dir.path().join("5.wav"),
            alert_1m: temp_dir.path().join("1.wav"),
        };
        
        manager.update_sound_files(new_sound_files).unwrap();
    }

    #[test]
    fn test_sound_files_default() {
        let sound_files = SoundFiles::default();
        assert!(sound_files.meeting_alert.ends_with("meeting_alert.wav"));
        assert!(sound_files.video_meeting_alert.ends_with("video_meeting_alert.wav"));
        assert!(sound_files.test_sound.ends_with("test_sound.wav"));
    }

    #[test]
    fn test_ensure_sound_directory() {
        let _temp_dir = TempDir::new().unwrap();
        let sounds_dir = AudioManager::ensure_sound_directory().unwrap();
        
        assert!(sounds_dir.exists());
        assert!(sounds_dir.is_dir());
    }

    #[test]
    fn test_alert_type_variants() {
        let meeting_type = AlertType::Meeting;
        let video_type = AlertType::VideoMeeting;
        let test_type = AlertType::Test;
        
        // Test that we can match on them
        match meeting_type {
            AlertType::Meeting => assert!(true),
            AlertType::VideoMeeting => assert!(false),
            AlertType::SnoozeReminder => assert!(false),
            AlertType::Test => assert!(false),
            _ => assert!(false),
        }
        
        match video_type {
            AlertType::Meeting => assert!(false),
            AlertType::VideoMeeting => assert!(true),
            AlertType::SnoozeReminder => assert!(false),
            AlertType::Test => assert!(false),
            _ => assert!(false),
        }
        
        match test_type {
            AlertType::Meeting => assert!(false),
            AlertType::VideoMeeting => assert!(false),
            AlertType::SnoozeReminder => assert!(false),
            AlertType::Test => assert!(true),
            _ => assert!(false),
        }
    }

    #[tokio::test]
    async fn test_play_alert_returns_result() {
        let manager = AudioManager::new().unwrap();
        
        // These should not panic, even if sound files don't exist
        let result = manager.play_alert(AlertType::Test);
        assert!(result.is_ok());
        
        let result = manager.play_alert(AlertType::Meeting);
        assert!(result.is_ok());
        
        let result = manager.play_alert(AlertType::VideoMeeting);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_test_audio() {
        let manager = AudioManager::new().unwrap();
        let result = manager.test_audio();
        assert!(result.is_ok());
    }

    #[test]
    fn test_default_sound_files() {
        let result = AudioManager::default_sound_files();
        assert!(result.is_ok());
        
        let sound_files = result.unwrap();
        assert!(sound_files.meeting_alert.to_string_lossy().contains("openchime"));
        assert!(sound_files.video_meeting_alert.to_string_lossy().contains("openchime"));
        assert!(sound_files.test_sound.to_string_lossy().contains("openchime"));
    }
}