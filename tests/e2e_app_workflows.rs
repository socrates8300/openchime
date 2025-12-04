use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;
use tempfile::TempDir;
use std::fs;

// Helper function to check if the app can be built
fn build_app() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("cargo")
        .args(&["build", "--release"])
        .output()?;
    
    if !output.status.success() {
        eprintln!("Build failed: {}", String::from_utf8_lossy(&output.stderr));
        return Err("Build failed".into());
    }
    
    Ok(())
}

// Helper function to run the app in headless mode for testing
async fn run_app_for_test() -> Result<(), Box<dyn std::error::Error>> {
    // First ensure the app builds
    build_app()?;
    
    // Create a temporary directory for test data
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_openchime.db");
    
    // Set environment variables for testing
    std::env::set_var("OPENCHIME_DB_PATH", db_path.to_string_lossy().as_ref());
    std::env::set_var("OPENCHIME_TEST_MODE", "true");
    
    // Run the app (this would normally require a display, but we're testing startup)
    let mut child = Command::new("./target/release/openchime")
        .env("OPENCHIME_DB_PATH", db_path.to_string_lossy().as_ref())
        .env("OPENCHIME_TEST_MODE", "true")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    
    // Give it a moment to start up
    sleep(Duration::from_secs(2)).await;
    
    // Check if it's still running (successful startup)
    match child.try_wait()? {
        Some(status) => {
            if !status.success() {
                let stderr = match child.stderr {
                    Some(stderr) => {
                        use std::io::Read;
                        let mut buffer = Vec::new();
                        let mut stderr_handle = stderr;
                        let _ = stderr_handle.read_to_end(&mut buffer);
                        String::from_utf8_lossy(&buffer).to_string()
                    },
                    None => "No stderr captured".to_string(),
                };
                eprintln!("App exited with error: {}", stderr);
                return Err("App failed to start".into());
            }
        }
        None => {
            // App is still running, which is good
            println!("App started successfully");
        }
    }
    
    // Clean shutdown
    child.kill()?;
    child.wait()?;
    
    Ok(())
}

#[tokio::test]
#[ignore] // Mark as ignored since it requires GUI environment
async fn test_app_startup_and_shutdown() {
    // This test verifies the app can start up and shut down cleanly
    let result = run_app_for_test().await;
    assert!(result.is_ok(), "App should start and shutdown without errors");
}

#[tokio::test]
#[ignore] // Mark as ignored since it requires GUI environment
async fn test_database_initialization() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test_openchime.db");
    
    std::env::set_var("OPENCHIME_DB_PATH", db_path.to_string_lossy().as_ref());
    
    // Run the app briefly to initialize database
    let mut child = Command::new("cargo")
        .args(&["run", "--", "--test-mode"])
        .env("OPENCHIME_DB_PATH", db_path.to_string_lossy().as_ref())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    
    sleep(Duration::from_secs(3)).await;
    child.kill().unwrap();
    child.wait().unwrap();
    
    // Verify database was created and has expected tables
    assert!(db_path.exists(), "Database file should be created");
    
    // Connect to the database and verify schema
    let pool = sqlx::SqlitePool::connect(&format!("sqlite:file:{}?mode=rwc", db_path.to_string_lossy()))
        .await
        .unwrap();
    
    // Check that tables exist
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    
    assert!(tables.contains(&"accounts".to_string()));
    assert!(tables.contains(&"events".to_string()));
    assert!(tables.contains(&"settings".to_string()));
    
    // Check that default settings were inserted
    let settings_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM settings")
        .fetch_one(&pool)
        .await
        .unwrap();
    
    assert!(settings_count > 0, "Default settings should be inserted");
}

#[tokio::test]
#[ignore] // Mark as ignored since it requires GUI environment
async fn test_audio_system_initialization() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test_openchime.db");
    
    std::env::set_var("OPENCHIME_DB_PATH", db_path.to_string_lossy().as_ref());
    
    // Test that audio system can initialize without crashing
    let audio_manager = openchime::AudioManager::new();
    assert!(audio_manager.is_ok(), "Audio manager should initialize successfully");
    
    let manager = audio_manager.unwrap();
    
    // Test basic audio operations
    assert_eq!(manager.get_volume(), 0.7, "Default volume should be 0.7");
    
    manager.set_volume(0.5).unwrap();
    assert_eq!(manager.get_volume(), 0.5, "Volume should be updated");
    
    // Test alert playback (should not panic even without sound files)
    manager.test_audio().unwrap();
}

#[tokio::test]
async fn test_configuration_file_handling() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("openchime");
    fs::create_dir_all(&config_dir).unwrap();
    
    std::env::set_var("OPENCHIME_CONFIG_DIR", config_dir.to_string_lossy().as_ref());
    
    // Test that the app can handle missing configuration files gracefully
    let result = openchime::AudioManager::ensure_sound_directory();
    assert!(result.is_ok(), "Should handle missing sound directory gracefully");
    
    let sounds_dir = result.unwrap();
    assert!(sounds_dir.exists(), "Should create sounds directory");
}

#[tokio::test]
async fn test_error_recovery() {
    // Test various error conditions and recovery scenarios
    
    // Test with invalid database path
    let invalid_path = "/invalid/path/openchime.db";
    std::env::set_var("OPENCHIME_DB_PATH", invalid_path);
    
    // The app should handle this gracefully or provide a clear error
    let pool_result = sqlx::SqlitePool::connect(&format!("sqlite:file:{}?mode=rwc", invalid_path)).await;
    
    // This might fail, but should fail gracefully with a clear error
    match pool_result {
        Ok(_) => println!("Unexpected success with invalid path"),
        Err(e) => {
            println!("Expected error with invalid path: {}", e);
            assert!(e.to_string().contains("no such file") || 
                   e.to_string().contains("permission") ||
                   e.to_string().contains("unable to open database file"), 
                   "Error should be descriptive");
        }
    }
}

#[tokio::test]
async fn test_concurrent_operations() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test_openchime.db");
    
    let pool = sqlx::SqlitePool::connect(&format!("sqlite:file:{}?mode=rwc", db_path.to_string_lossy()))
        .await
        .unwrap();
    
    // Run schema
    let schema = include_str!("../src/database/schema.sql");
    sqlx::query(schema).execute(&pool).await.unwrap();
    
    // Test concurrent database operations
    let mut handles = vec![];
    
    for i in 0..10 {
        let pool_clone = pool.clone();
        let handle = tokio::spawn(async move {
            // Insert test account
            sqlx::query(
                "INSERT INTO accounts (provider, account_name, auth_data) VALUES (?, ?, ?)"
            )
            .bind("google")
            .bind(format!("test{}@gmail.com", i))
            .bind(format!("auth_data_{}", i))
            .execute(&pool_clone)
            .await
            .unwrap();
            
            // Read settings
            let settings: Vec<(String, String)> = sqlx::query_as(
                "SELECT key, value FROM settings"
            )
            .fetch_all(&pool_clone)
            .await
            .unwrap();
            
            assert!(!settings.is_empty(), "Settings should be available");
        });
        handles.push(handle);
    }
    
    // Wait for all operations to complete
    for handle in handles {
        handle.await.unwrap();
    }
    
    // Verify all accounts were added
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM accounts")
        .fetch_one(&pool)
        .await
        .unwrap();
    
    assert_eq!(count, 10, "All accounts should be added successfully");
}

// Helper test to verify the test environment
#[test]
fn test_test_environment() {
    // Verify we're in a test environment
    assert!(cfg!(test), "Should be running in test mode");
    
    // Verify required test directories exist
    let current_dir = std::env::current_dir().unwrap();
    let src_dir = current_dir.join("src");
    let tests_dir = current_dir.join("tests");
    
    assert!(src_dir.exists(), "Source directory should exist");
    assert!(tests_dir.exists(), "Tests directory should exist");
}