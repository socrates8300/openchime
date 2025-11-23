//! HTTP client configuration module
//! 
//! This module provides centralized configuration for HTTP clients,
//! including timeouts, retry policies, and connection settings.

use reqwest::{Client, ClientBuilder};
use std::time::Duration;

/// HTTP client configuration
#[derive(Debug, Clone)]
pub struct HttpConfig {
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Read timeout
    pub read_timeout: Duration,
    /// Total request timeout
    pub timeout: Duration,
    /// Maximum number of retries
    pub max_retries: u32,
    /// Base delay for exponential backoff
    pub base_retry_delay: Duration,
    /// Maximum retry delay
    pub max_retry_delay: Duration,
    /// Backoff multiplier for exponential backoff
    pub backoff_multiplier: f64,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(30),
            timeout: Duration::from_secs(45),
            max_retries: 3,
            base_retry_delay: Duration::from_millis(500),
            max_retry_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
        }
    }
}

impl HttpConfig {
    /// Create default HTTP config
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Create HTTP config optimized for calendar API calls
    pub fn calendar_api() -> Self {
        Self {
            connect_timeout: Duration::from_secs(15),
            read_timeout: Duration::from_secs(60), // Calendar APIs can be slow
            timeout: Duration::from_secs(90),
            max_retries: 3,
            base_retry_delay: Duration::from_millis(1000),
            max_retry_delay: Duration::from_secs(20),
            backoff_multiplier: 2.0,
        }
    }
    
    /// Create HTTP config for ICS data fetching
    pub fn ics_fetch() -> Self {
        Self {
            connect_timeout: Duration::from_secs(20),
            read_timeout: Duration::from_secs(120), // ICS files can be large
            timeout: Duration::from_secs(150),
            max_retries: 2,
            base_retry_delay: Duration::from_millis(2000),
            max_retry_delay: Duration::from_secs(30),
            backoff_multiplier: 1.5,
        }
    }
    
    /// Create HTTP config for OAuth operations
    pub fn oauth() -> Self {
        Self {
            connect_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(30),
            timeout: Duration::from_secs(45),
            max_retries: 2, // OAuth should fail fast
            base_retry_delay: Duration::from_millis(500),
            max_retry_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
        }
    }
    
    /// Build a reqwest client with this configuration
    pub fn build_client(&self) -> Result<Client, Box<dyn std::error::Error + Send + Sync>> {
        Ok(ClientBuilder::new()
            .connect_timeout(self.connect_timeout)
            .timeout(self.timeout) // Use unified timeout instead of separate read_timeout
            .tcp_keepalive(Duration::from_secs(30))
            .http2_adaptive_window(true)
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(2)
            .build()?)
    }
    
    /// Create retry config for external use
    pub fn to_retry_config(&self) -> crate::utils::retry::RetryConfig {
        crate::utils::retry::RetryConfig {
            max_attempts: self.max_retries,
            base_delay: self.base_retry_delay,
            max_delay: self.max_retry_delay,
            backoff_multiplier: self.backoff_multiplier,
        }
    }
}

/// HTTP client factory for creating pre-configured clients
pub struct HttpClientFactory {
    default_config: HttpConfig,
}

impl HttpClientFactory {
    /// Create new HTTP client factory
    pub fn new() -> Self {
        Self {
            default_config: HttpConfig::default(),
        }
    }
    
    /// Get default client for general use
    pub fn default_client(&self) -> Result<Client, Box<dyn std::error::Error + Send + Sync>> {
        self.default_config.build_client()
    }
    
    /// Get client optimized for calendar API calls
    pub fn calendar_client(&self) -> Result<Client, Box<dyn std::error::Error + Send + Sync>> {
        HttpConfig::calendar_api().build_client()
    }
    
    /// Get client for ICS data fetching
    pub fn ics_client(&self) -> Result<Client, Box<dyn std::error::Error + Send + Sync>> {
        HttpConfig::ics_fetch().build_client()
    }
    
    /// Get client for OAuth operations
    pub fn oauth_client(&self) -> Result<Client, Box<dyn std::error::Error + Send + Sync>> {
        HttpConfig::oauth().build_client()
    }
}

impl Default for HttpClientFactory {
    fn default() -> Self {
        Self::new()
    }
}
