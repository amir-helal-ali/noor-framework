// ============================================================
// Circuit Breaker - قاطع الدائرة
// ============================================================
// Protects against cascading failures when calling external services.
// Implements the circuit breaker pattern with three states:
// Closed, Open, and Half-Open.
//
// يحمي من الفشل المتسلسل عند استدعاء خدمات خارجية.
// ============================================================

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicI64, AtomicBool, Ordering};
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitState {
    /// Normal operation - requests are allowed
    Closed,
    /// Circuit is open - requests are blocked
    Open,
    /// Testing if the service has recovered
    HalfOpen,
}

impl std::fmt::Display for CircuitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Closed => write!(f, "closed"),
            Self::Open => write!(f, "open"),
            Self::HalfOpen => write!(f, "half-open"),
        }
    }
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitConfig {
    /// Number of failures before opening the circuit
    pub failure_threshold: u32,
    /// Number of successes needed to close the circuit (from half-open)
    pub success_threshold: u32,
    /// Time to wait before transitioning from Open to Half-Open (seconds)
    pub reset_timeout: u64,
    /// Time window for counting failures (seconds)
    pub failure_window: u64,
    /// Maximum timeout for a request (milliseconds)
    pub request_timeout: u64,
}

impl Default for CircuitConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            reset_timeout: 60,
            failure_window: 60,
            request_timeout: 5000,
        }
    }
}

/// Circuit breaker
pub struct CircuitBreaker {
    name: String,
    config: CircuitConfig,
    state: Arc<RwLock<CircuitState>>,
    failure_count: Arc<AtomicU32>,
    success_count: Arc<AtomicU32>,
    last_failure_time: Arc<AtomicI64>,
    opened_at: Arc<RwLock<Option<Instant>>>,
    /// Whether the circuit is currently tripped
    tripped: Arc<AtomicBool>,
}

impl CircuitBreaker {
    pub fn new(name: &str, config: CircuitConfig) -> Self {
        Self {
            name: name.to_string(),
            config,
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(AtomicU32::new(0)),
            success_count: Arc::new(AtomicU32::new(0)),
            last_failure_time: Arc::new(AtomicI64::new(0)),
            opened_at: Arc::new(RwLock::new(None)),
            tripped: Arc::new(AtomicBool::new(false)),
        }
    }
    
    /// Get the circuit name
    pub fn name(&self) -> &str {
        &self.name
    }
    
    /// Get the current state
    pub fn state(&self) -> CircuitState {
        self.check_state_transition();
        *self.state.read()
    }
    
    /// Check if a request is allowed
    pub fn allow_request(&self) -> bool {
        self.check_state_transition();
        
        let state = *self.state.read();
        
        match state {
            CircuitState::Closed => true,
            CircuitState::Open => false,
            CircuitState::HalfOpen => true, // Allow one test request
        }
    }
    
    /// Record a successful request
    pub fn record_success(&self) {
        self.check_state_transition();
        
        let state = *self.state.read();
        
        match state {
            CircuitState::HalfOpen => {
                let successes = self.success_count.fetch_add(1, Ordering::SeqCst) + 1;
                
                if successes >= self.config.success_threshold {
                    // Close the circuit
                    self.transition_to(CircuitState::Closed);
                    self.failure_count.store(0, Ordering::SeqCst);
                    self.success_count.store(0, Ordering::SeqCst);
                    self.tripped.store(false, Ordering::SeqCst);
                    tracing::info!("Circuit '{}' closed (recovered)", self.name);
                }
            }
            CircuitState::Closed => {
                // Reset failure count on success
                self.failure_count.store(0, Ordering::SeqCst);
            }
            _ => {}
        }
    }
    
    /// Record a failed request
    pub fn record_failure(&self) {
        self.check_state_transition();
        
        let state = *self.state.read();
        
        match state {
            CircuitState::Closed => {
                let failures = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
                self.last_failure_time.store(chrono::Utc::now().timestamp(), Ordering::SeqCst);
                
                if failures >= self.config.failure_threshold {
                    self.transition_to(CircuitState::Open);
                    *self.opened_at.write() = Some(Instant::now());
                    self.tripped.store(true, Ordering::SeqCst);
                    tracing::warn!("Circuit '{}' opened ({} failures)", self.name, failures);
                }
            }
            CircuitState::HalfOpen => {
                // Failure during half-open -> back to open
                self.transition_to(CircuitState::Open);
                *self.opened_at.write() = Some(Instant::now());
                self.success_count.store(0, Ordering::SeqCst);
                tracing::warn!("Circuit '{}' re-opened (failure during half-open)", self.name);
            }
            _ => {}
        }
    }
    
    /// Execute a function with circuit breaker protection
    pub fn execute<F, T>(&self, operation: F) -> Result<T, CircuitBreakerError>
    where
        F: FnOnce() -> crate::NoorResult<T>,
    {
        if !self.allow_request() {
            return Err(CircuitBreakerError::CircuitOpen);
        }
        
        match operation() {
            Ok(result) => {
                self.record_success();
                Ok(result)
            }
            Err(e) => {
                self.record_failure();
                Err(CircuitBreakerError::OperationFailed(e.to_string()))
            }
        }
    }
    
    /// Check and perform state transitions
    fn check_state_transition(&self) {
        let state = *self.state.read();
        
        if state == CircuitState::Open {
            if let Some(opened_at) = *self.opened_at.read() {
                let elapsed = opened_at.elapsed();
                
                if elapsed >= Duration::from_secs(self.config.reset_timeout) {
                    // Transition to half-open
                    self.transition_to(CircuitState::HalfOpen);
                    self.success_count.store(0, Ordering::SeqCst);
                    tracing::info!("Circuit '{}' half-open (testing recovery)", self.name);
                }
            }
        }
    }
    
    /// Transition to a new state
    fn transition_to(&self, new_state: CircuitState) {
        let mut state = self.state.write();
        if *state != new_state {
            *state = new_state;
        }
    }
    
    /// Get the current failure count
    pub fn failure_count(&self) -> u32 {
        self.failure_count.load(Ordering::SeqCst)
    }
    
    /// Get the current success count
    pub fn success_count(&self) -> u32 {
        self.success_count.load(Ordering::SeqCst)
    }
    
    /// Check if the circuit has been tripped
    pub fn is_tripped(&self) -> bool {
        self.tripped.load(Ordering::SeqCst)
    }
    
    /// Reset the circuit breaker
    pub fn reset(&self) {
        self.transition_to(CircuitState::Closed);
        self.failure_count.store(0, Ordering::SeqCst);
        self.success_count.store(0, Ordering::SeqCst);
        *self.opened_at.write() = None;
        self.tripped.store(false, Ordering::SeqCst);
        tracing::info!("Circuit '{}' manually reset", self.name);
    }
    
    /// Get circuit statistics
    pub fn stats(&self) -> CircuitStats {
        CircuitStats {
            name: self.name.clone(),
            state: self.state(),
            failure_count: self.failure_count(),
            success_count: self.success_count(),
            tripped: self.is_tripped(),
        }
    }
}

/// Circuit breaker error
#[derive(Debug, Clone)]
pub enum CircuitBreakerError {
    /// The circuit is open and requests are blocked
    CircuitOpen,
    /// The operation failed
    OperationFailed(String),
}

impl std::fmt::Display for CircuitBreakerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CircuitOpen => write!(f, "Circuit breaker is open"),
            Self::OperationFailed(msg) => write!(f, "Operation failed: {}", msg),
        }
    }
}

/// Circuit breaker statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitStats {
    pub name: String,
    pub state: CircuitState,
    pub failure_count: u32,
    pub success_count: u32,
    pub tripped: bool,
}

/// Registry for managing multiple circuit breakers
pub struct CircuitBreakerRegistry {
    breakers: Arc<RwLock<std::collections::HashMap<String, Arc<CircuitBreaker>>>>,
}

impl Default for CircuitBreakerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CircuitBreakerRegistry {
    pub fn new() -> Self {
        Self {
            breakers: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }
    
    /// Register a circuit breaker
    pub fn register(&self, breaker: Arc<CircuitBreaker>) {
        self.breakers.write().insert(breaker.name().to_string(), breaker);
    }
    
    /// Get a circuit breaker by name
    pub fn get(&self, name: &str) -> Option<Arc<CircuitBreaker>> {
        self.breakers.read().get(name).cloned()
    }
    
    /// Get or create a circuit breaker
    pub fn get_or_create(&self, name: &str, config: CircuitConfig) -> Arc<CircuitBreaker> {
        let mut breakers = self.breakers.write();
        
        breakers
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(CircuitBreaker::new(name, config.clone())))
            .clone()
    }
    
    /// Get all circuit breakers
    pub fn all(&self) -> Vec<Arc<CircuitBreaker>> {
        self.breakers.read().values().cloned().collect()
    }
    
    /// Get statistics for all circuit breakers
    pub fn stats(&self) -> Vec<CircuitStats> {
        self.breakers.read().values().map(|b| b.stats()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_circuit_starts_closed() {
        let breaker = CircuitBreaker::new("test", CircuitConfig::default());
        
        assert_eq!(breaker.state(), CircuitState::Closed);
        assert!(breaker.allow_request());
    }
    
    #[test]
    fn test_circuit_opens_after_failures() {
        let config = CircuitConfig {
            failure_threshold: 3,
            ..Default::default()
        };
        
        let breaker = CircuitBreaker::new("test", config);
        
        // Record 3 failures
        breaker.record_failure();
        breaker.record_failure();
        
        assert_eq!(breaker.state(), CircuitState::Closed);
        
        breaker.record_failure();
        
        assert_eq!(breaker.state(), CircuitState::Open);
        assert!(!breaker.allow_request());
    }
    
    #[test]
    fn test_circuit_execute_success() {
        let breaker = CircuitBreaker::new("test", CircuitConfig::default());
        
        let result = breaker.execute(|| Ok(42));
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }
    
    #[test]
    fn test_circuit_execute_blocked_when_open() {
        let config = CircuitConfig {
            failure_threshold: 1,
            ..Default::default()
        };
        
        let breaker = CircuitBreaker::new("test", config);
        
        // Trigger one failure to open the circuit
        breaker.record_failure();
        
        let result = breaker.execute(|| Ok(42));
        
        assert!(matches!(result, Err(CircuitBreakerError::CircuitOpen)));
    }
    
    #[test]
    fn test_circuit_reset() {
        let config = CircuitConfig {
            failure_threshold: 1,
            ..Default::default()
        };
        
        let breaker = CircuitBreaker::new("test", config);
        
        breaker.record_failure();
        assert_eq!(breaker.state(), CircuitState::Open);
        
        breaker.reset();
        assert_eq!(breaker.state(), CircuitState::Closed);
    }
    
    #[test]
    fn test_circuit_registry() {
        let registry = CircuitBreakerRegistry::new();
        
        let breaker = registry.get_or_create("api", CircuitConfig::default());
        
        assert_eq!(breaker.name(), "api");
        
        let same_breaker = registry.get("api").unwrap();
        assert!(Arc::ptr_eq(&breaker, &same_breaker));
    }
}
