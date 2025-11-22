#![allow(dead_code)]
use log::{LevelFilter, SetLoggerError, Level};
use env_logger::{Builder, Target};
use std::env;
use std::io::Write;

pub fn init_logging() -> Result<(), SetLoggerError> {
    let env = env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let log_level = match env.to_lowercase().as_str() {
        "error" => LevelFilter::Error,
        "warn" => LevelFilter::Warn,
        "info" => LevelFilter::Info,
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        _ => LevelFilter::Info,
    };

    let mut builder = Builder::from_default_env();
    
    // Customize format for better readability
    builder.format(|buf, record| {
        let timestamp = buf.timestamp();
        let target = record.target();
        let file = record.file().unwrap_or("unknown");
        let line = record.line().unwrap_or(0);
        
        match record.level() {
            Level::Error => {
                writeln!(buf,
                    "{} [ERROR] [{}:{}] {}: {}",
                    timestamp, file, line, target, record.args()
                )
            }
            Level::Warn => {
                writeln!(buf,
                    "{} [WARN] [{}:{}] {}: {}",
                    timestamp, file, line, target, record.args()
                )
            }
            Level::Info => {
                writeln!(buf,
                    "{} [INFO] [{}]: {}",
                    timestamp, target, record.args()
                )
            }
            Level::Debug => {
                writeln!(buf,
                    "{} [DEBUG] [{}:{}] {}: {}",
                    timestamp, file, line, target, record.args()
                )
            }
            Level::Trace => {
                writeln!(buf,
                    "{} [TRACE] [{}:{}] {}: {}",
                    timestamp, file, line, target, record.args()
                )
            }
        }
    });

    // Filter out noisy modules in production
    if env::var("RUST_ENV").unwrap_or_else(|_| "development".to_string()) == "production" {
        builder.filter_module("reqwest", LevelFilter::Warn);
        builder.filter_module("hyper", LevelFilter::Warn);
        builder.filter_module("tokio", LevelFilter::Info);
        builder.filter_module("sqlx", LevelFilter::Warn);
    }

    builder.filter_level(log_level)
           .target(Target::Stdout)
           .init();
    Ok(())
}

pub fn log_error_with_context(error: &anyhow::Error, context: &str) {
    log::error!("[{}] {}", context, error);
    
    // Log chain of causes for better debugging
    let mut source = error.source();
    while let Some(err) = source {
        log::error!("  Caused by: {}", err);
        source = err.source();
    }
}

pub fn log_network_error(operation: &str, error: &dyn std::error::Error) {
    log::warn!("[Network] {} failed: {}", operation, error);
}

pub fn log_calendar_sync(account_name: &str, events_count: usize, duration_ms: u64) {
    log::info!("[Calendar] Synced {} events for account '{}' in {}ms", 
               events_count, account_name, duration_ms);
}

pub fn log_database_operation(operation: &str, table: &str, duration_ms: u64) {
    log::debug!("[Database] {} on table {} took {}ms", operation, table, duration_ms);
}

pub fn log_auth_event(event: &str, account_name: &str) {
    log::info!("[Auth] {} for account '{}'", event, account_name);
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_log_level_parsing() {
        assert_eq!(LevelFilter::Error, 
                   match "error".to_lowercase().as_str() {
                       "error" => LevelFilter::Error,
                       "warn" => LevelFilter::Warn,
                       "info" => LevelFilter::Info,
                       "debug" => LevelFilter::Debug,
                       "trace" => LevelFilter::Trace,
                       _ => LevelFilter::Info,
                   });
    }
}