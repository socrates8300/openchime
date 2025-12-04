// OpenChime - Cross-platform meeting reminder app
// Main entry point for iced application

use log::{info, error, warn};
use std::sync::Arc;
use iced::{Application, Settings as IcedSettings};

use openchime::database::Database;
use openchime::audio::AudioManager;
use openchime::app::OpenChimeApp;
use openchime::config;

fn main() -> iced::Result {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("Starting OpenChime with iced UI");

    // Create a Tokio runtime for async operations
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime");

    // Initialize core components within the runtime
    let (db, audio) = rt.block_on(async {
    // Validate configuration
    if let Err(e) = config::validate_config() {
        error!("Configuration validation failed: {}", e);
        eprintln!("\n❌ Configuration Error:\n");
        eprintln!("{}\n", e);
        eprintln!("Please check your configuration and try again.");
        std::process::exit(1);
    }

        // Initialize core components
        let db = match Database::new().await {
            Ok(database) => Arc::new(database),
            Err(e) => {
                error!("Failed to initialize database: {}", e);
                eprintln!("Failed to initialize database: {}", e);
                eprintln!("Please check your system and try again.");
                std::process::exit(1);
            }
        };
        
        let audio = match AudioManager::new() {
            Ok(audio_manager) => Arc::new(audio_manager),
            Err(e) => {
                warn!("Failed to initialize audio system: {}", e);
                warn!("Continuing without audio - audio features will be disabled");
                // Continue without audio - create a dummy audio manager
                Arc::new(AudioManager::new_dummy())
            }
        };

        (db, audio)
    });

    // Run iced application
    // The runtime 'rt' stays alive here, allowing background tasks (like DB pool) to function.
    let result = OpenChimeApp::run(IcedSettings {
        flags: (db, audio),
        window: iced::window::Settings {
            size: iced::Size::new(800.0, 600.0),
            resizable: true,
            ..Default::default()
        },
        id: None,
        fonts: vec![],
        default_font: Default::default(),
        default_text_size: iced::Pixels(16.0),
        antialiasing: false,
    });

    // Explicitly drop runtime to ensure clean shutdown of background tasks
    drop(rt);

    result
}