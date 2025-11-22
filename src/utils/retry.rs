use std::time::Duration;
use anyhow::Result;
use log::{warn, info, debug};

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(1000),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
        }
    }
}

pub async fn retry_with_exponential_backoff<T, F, Fut>(
    config: &RetryConfig,
    operation: F,
) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T>> + Send + 'static,
{
    let mut delay = config.base_delay;
    
    for attempt in 1..=config.max_attempts {
        let result = operation().await;
        match result {
            Ok(value) => {
                if attempt > 1 {
                    info!("Operation succeeded on attempt {}", attempt);
                }
                return Ok(value);
            }
            Err(e) => {
                if attempt == config.max_attempts {
                    warn!("Operation failed after {} attempts: {}", config.max_attempts, e);
                    return Err(anyhow::anyhow!("Failed after {} retry attempts: {}", config.max_attempts, e));
                }
                
                if is_transient_error(&e) {
                    debug!("Attempt {} failed transiently, retrying in {:?}: {}", attempt, delay, e);
                    tokio::time::sleep(delay).await;
                    delay = std::cmp::min(
                        Duration::from_millis((delay.as_millis() as f64 * config.backoff_multiplier) as u64),
                        config.max_delay,
                    );
                } else {
                    debug!("Attempt {} failed with non-transient error, not retrying: {}", attempt, e);
                    return Err(e);
                }
            }
        }
    }
    
    unreachable!()
}

fn is_transient_error(error: &anyhow::Error) -> bool {
    let error_str = error.to_string().to_lowercase();
    
    // Network-related transient errors
    error_str.contains("timeout") ||
    error_str.contains("connection") ||
    error_str.contains("network") ||
    error_str.contains("temporary") ||
    error_str.contains("rate limit") ||
    error_str.contains("too many requests") ||
    error_str.contains("service unavailable") ||
    error_str.contains("internal server error") ||
    error_str.contains("bad gateway") ||
    error_str.contains("gateway timeout") ||
    // HTTP status codes that are typically transient
    error_str.contains("429") || // Too Many Requests
    error_str.contains("502") || // Bad Gateway
    error_str.contains("503") || // Service Unavailable
    error_str.contains("504")    // Gateway Timeout
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_retry_success_on_second_attempt() {
        let attempt_count = Arc::new(AtomicU32::new(0));
        let config = RetryConfig::default();
        let attempt_count_clone = attempt_count.clone();
        
        let result = retry_with_exponential_backoff(&config, || {
            let count_clone = attempt_count_clone.clone();
            Box::pin(async move {
                let count = count_clone.fetch_add(1, Ordering::SeqCst);
                if count == 0 {
                    Err(anyhow::anyhow!("Temporary failure"))
                } else {
                    Ok("success")
                }
            })
        }).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert_eq!(attempt_count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_retry_non_transient_error() {
        let attempt_count = Arc::new(AtomicU32::new(0));
        let config = RetryConfig::default();
        let attempt_count_clone = attempt_count.clone();
        
        let result: Result<&str, _> = retry_with_exponential_backoff(&config, || {
            let count_clone = attempt_count_clone.clone();
            Box::pin(async move {
                count_clone.fetch_add(1, Ordering::SeqCst);
                Err(anyhow::anyhow!("Authentication failed"))
            })
        }).await;
        
        assert!(result.is_err());
        assert_eq!(attempt_count.load(Ordering::SeqCst), 1);
    }
}