#![allow(dead_code)]
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Authentication error: {0}")]
    Auth(String),
    
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("Error: {0}")]
    Anyhow(#[from] anyhow::Error),
    
    #[error("Calendar error: {0}")]
    Calendar(String),
    
    #[error("Audio error: {0}")]
    Audio(String),
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Operation failed: {0}")]
    OperationFailed(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}

impl AppError {
    pub fn auth<S: Into<String>>(msg: S) -> Self {
        Self::Auth(msg.into())
    }
    
    pub fn calendar<S: Into<String>>(msg: S) -> Self {
        Self::Calendar(msg.into())
    }
    
    pub fn audio<S: Into<String>>(msg: S) -> Self {
        Self::Audio(msg.into())
    }
    
    pub fn invalid_input<S: Into<String>>(msg: S) -> Self {
        Self::InvalidInput(msg.into())
    }
    
    pub fn config<S: Into<String>>(msg: S) -> Self {
        Self::Config(msg.into())
    }
    
    pub fn operation_failed<S: Into<String>>(msg: S) -> Self {
        Self::OperationFailed(msg.into())
    }
    
    pub fn not_found<S: Into<String>>(msg: S) -> Self {
        Self::NotFound(msg.into())
    }
    
    pub fn permission_denied<S: Into<String>>(msg: S) -> Self {
        Self::PermissionDenied(msg.into())
    }
    
    pub fn is_pii_safe(&self) -> bool {
        match self {
            Self::Database(_) | Self::Network(_) | Self::Anyhow(_) => false,
            Self::Auth(_) | Self::Calendar(_) | Self::Audio(_) 
            | Self::InvalidInput(_) | Self::Config(_) 
            | Self::OperationFailed(_) | Self::NotFound(_) 
            | Self::PermissionDenied(_) => true,
        }
    }
    
    pub fn to_safe_string(&self) -> String {
        if self.is_pii_safe() {
            self.to_string()
        } else {
            match self {
                Self::Database(_) => "Database operation failed".to_string(),
                Self::Network(_) => "Network request failed".to_string(),
                Self::Anyhow(_) => "Operation failed".to_string(),
                _ => self.to_string(),
            }
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;