use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use anyhow::Result;
use log::{warn, info};

#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout: Duration::from_secs(60),
        }
    }
}

#[derive(Debug, Clone)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug)]
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: Arc<RwLock<CircuitState>>,
    failure_count: Arc<RwLock<u32>>,
    success_count: Arc<RwLock<u32>>,
    last_failure_time: Arc<RwLock<Option<Instant>>>,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(RwLock::new(0)),
            success_count: Arc::new(RwLock::new(0)),
            last_failure_time: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn execute<F, T, Fut>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>> + Send,
    {
        // Check if circuit is open
        let should_try_half_open = {
            let state = self.state.read().await;
            let last_failure = self.last_failure_time.read().await;
            
            match *state {
                CircuitState::Open => {
                    if let Some(failure_time) = *last_failure {
                        failure_time.elapsed() > self.config.timeout
                    } else {
                        false
                    }
                }
                CircuitState::HalfOpen => false, // Already half-open, allow through
                CircuitState::Closed => false,
            }
        };

        if should_try_half_open {
             // Transition to half-open
             let mut state = self.state.write().await;
             // Double check state hasn't changed
             if matches!(*state, CircuitState::Open) {
                 *state = CircuitState::HalfOpen;
                 *self.success_count.write().await = 0;
                 info!("Circuit breaker transitioning to half-open after timeout");
             }
        }

        // Re-check state for blocking
        {
            let state = self.state.read().await;
            if matches!(*state, CircuitState::Open) {
                 return Err(anyhow::anyhow!("Circuit breaker is open"));
            }
        }

        // Execute the operation
        let result = operation().await;
        
        // Update circuit state based on result
        match result {
            Ok(_) => {
                self.on_success().await;
            }
            Err(_) => {
                self.on_failure().await;
            }
        }
        
        result
    }

    async fn on_success(&self) {
        let mut state = self.state.write().await;
        let mut success_count = self.success_count.write().await;
        
        match *state {
            CircuitState::HalfOpen => {
                *success_count += 1;
                if *success_count >= self.config.success_threshold {
                    *state = CircuitState::Closed;
                    *self.failure_count.write().await = 0;
                    info!("Circuit breaker closing after {} successful calls", *success_count);
                }
            }
            CircuitState::Closed => {
                // Reset failure count on success in closed state
                *self.failure_count.write().await = 0;
            }
            CircuitState::Open => {
                // Shouldn't happen, but handle gracefully
                *state = CircuitState::Closed;
                *self.failure_count.write().await = 0;
            }
        }
    }

    async fn on_failure(&self) {
        let mut state = self.state.write().await;
        let mut failure_count = self.failure_count.write().await;
        let mut last_failure_time = self.last_failure_time.write().await;
        
        *failure_count += 1;
        *last_failure_time = Some(Instant::now());
        
        match *state {
            CircuitState::Closed | CircuitState::HalfOpen => {
                if *failure_count >= self.config.failure_threshold {
                    *state = CircuitState::Open;
                    warn!("Circuit breaker opening after {} failures", *failure_count);
                }
            }
            CircuitState::Open => {
                // Already open, just update failure time
            }
        }
    }

    pub async fn get_state(&self) -> CircuitState {
        self.state.read().await.clone()
    }

    pub async fn get_stats(&self) -> CircuitBreakerStats {
        let state = self.state.read().await;
        let failure_count = *self.failure_count.read().await;
        let success_count = *self.success_count.read().await;
        let last_failure = *self.last_failure_time.read().await;
        
        CircuitBreakerStats {
            state: state.clone(),
            failure_count,
            success_count,
            last_failure_time: last_failure,
        }
    }
}

#[derive(Debug)]
pub struct CircuitBreakerStats {
    pub state: CircuitState,
    pub failure_count: u32,
    pub success_count: u32,
    pub last_failure_time: Option<Instant>,
}

// Global circuit breaker registry for different services
pub struct CircuitBreakerRegistry {
    breakers: Arc<RwLock<HashMap<String, Arc<CircuitBreaker>>>>,
}

impl Default for CircuitBreakerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CircuitBreakerRegistry {
    pub fn new() -> Self {
        Self {
            breakers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_breaker(&self, service_name: &str) -> Arc<CircuitBreaker> {
        let mut breakers = self.breakers.write().await;
        
        if !breakers.contains_key(service_name) {
            let config = match service_name {
                "google_calendar" => CircuitBreakerConfig {
                    failure_threshold: 3,
                    success_threshold: 2,
                    timeout: Duration::from_secs(30),
                },
                "proton_calendar" => CircuitBreakerConfig {
                    failure_threshold: 5,
                    success_threshold: 3,
                    timeout: Duration::from_secs(60),
                },
                _ => CircuitBreakerConfig::default(),
            };
            
            breakers.insert(service_name.to_string(), Arc::new(CircuitBreaker::new(config)));
            info!("Created circuit breaker for service: {}", service_name);
        }
        
        breakers.get(service_name).unwrap().clone()
    }

    pub async fn get_all_stats(&self) -> HashMap<String, CircuitBreakerStats> {
        let breakers = self.breakers.read().await;
        let mut stats = HashMap::new();
        
        for (service_name, breaker) in breakers.iter() {
            stats.insert(service_name.clone(), breaker.get_stats().await);
        }
        
        stats
    }
}

// Global instance
lazy_static::lazy_static! {
    pub static ref CIRCUIT_BREAKER_REGISTRY: CircuitBreakerRegistry = CircuitBreakerRegistry::new();
}

pub async fn get_circuit_breaker(service_name: &str) -> Arc<CircuitBreaker> {
    CIRCUIT_BREAKER_REGISTRY.get_breaker(service_name).await
}

pub async fn get_all_circuit_breaker_stats() -> HashMap<String, CircuitBreakerStats> {
    CIRCUIT_BREAKER_REGISTRY.get_all_stats().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_circuit_breaker_opens_on_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 1,
            timeout: Duration::from_millis(100),
        };
        
        let breaker = CircuitBreaker::new(config);
        
        // First failure
        let result: Result<&str, _> = breaker.execute(|| async {
            Err(anyhow::anyhow!("Test failure"))
        }).await;
        assert!(result.is_err());
        assert!(matches!(breaker.get_state().await, CircuitState::Closed));
        
        // Second failure should open circuit
        let result: Result<&str, _> = breaker.execute(|| async {
            Err(anyhow::anyhow!("Test failure"))
        }).await;
        assert!(result.is_err());
        assert!(matches!(breaker.get_state().await, CircuitState::Open));
        
        // Third call should fail immediately
        let result: Result<&str, _> = breaker.execute(|| async {
            Ok("success")
        }).await;
        assert!(result.is_err());
        assert!(matches!(breaker.get_state().await, CircuitState::Open));
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_state() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 1,
            timeout: Duration::from_millis(50),
        };
        
        let breaker = CircuitBreaker::new(config);
        
        // Cause circuit to open
        for _ in 0..2 {
            let _: Result<&str, _> = breaker.execute(|| async {
                Err(anyhow::anyhow!("Test failure"))
            }).await;
        }
        
        assert!(matches!(breaker.get_state().await, CircuitState::Open));
        
        // Wait for timeout
        sleep(Duration::from_millis(60)).await;
        
        // Next call should go to half-open and succeed
        let result = breaker.execute(|| async {
            Ok("success")
        }).await;
        assert!(result.is_ok());
        assert!(matches!(breaker.get_state().await, CircuitState::Closed));
    }
}