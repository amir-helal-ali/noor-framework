// ============================================================
// Advanced Rate Limiter - محدد المعدل المتقدم
// ============================================================
// Multi-strategy rate limiting:
// - Fixed Window
// - Sliding Window
// - Token Bucket
// - Leaky Bucket
// - Concurrent
//
// تحديد معدل متعدد الاستراتيجيات.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// Rate limiting strategy
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RateLimitStrategy {
    /// Fixed window counter
    FixedWindow,
    /// Sliding window counter
    SlidingWindow,
    /// Token bucket
    TokenBucket,
    /// Leaky bucket
    LeakyBucket,
    /// Concurrent request limit
    Concurrent,
}

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub strategy: RateLimitStrategy,
    pub max_requests: u32,
    pub window_secs: u64,
    /// For token bucket: refill rate (tokens per second)
    pub refill_rate: Option<f64>,
    /// For concurrent: max simultaneous requests
    pub max_concurrent: Option<u32>,
    /// Key to identify the client (IP, user_id, etc.)
    pub key_prefix: String,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            strategy: RateLimitStrategy::SlidingWindow,
            max_requests: 60,
            window_secs: 60,
            refill_rate: None,
            max_concurrent: None,
            key_prefix: "rl:".to_string(),
        }
    }
}

/// Rate limit result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitResult {
    pub allowed: bool,
    pub remaining: u32,
    pub limit: u32,
    pub retry_after: Option<u64>,
    pub reset_at: Option<i64>,
}

impl RateLimitResult {
    pub fn allowed(limit: u32, remaining: u32) -> Self {
        Self {
            allowed: true,
            remaining,
            limit,
            retry_after: None,
            reset_at: None,
        }
    }
    
    pub fn denied(limit: u32, retry_after: u64) -> Self {
        Self {
            allowed: false,
            remaining: 0,
            limit,
            retry_after: Some(retry_after),
            reset_at: Some(chrono::Utc::now().timestamp() + retry_after as i64),
        }
    }
}

/// Fixed window state
#[derive(Debug, Clone)]
struct FixedWindowState {
    count: u32,
    window_start: Instant,
}

/// Sliding window state
#[derive(Debug, Clone)]
struct SlidingWindowState {
    timestamps: Vec<Instant>,
}

/// Token bucket state
#[derive(Debug, Clone)]
struct TokenBucketState {
    tokens: f64,
    last_refill: Instant,
}

/// Concurrent state
#[derive(Debug, Clone)]
struct ConcurrentState {
    current: u32,
}

/// Advanced rate limiter
pub struct AdvancedRateLimiter {
    config: RateLimitConfig,
    /// Fixed window states
    fixed_window: Arc<RwLock<HashMap<String, FixedWindowState>>>,
    /// Sliding window states
    sliding_window: Arc<RwLock<HashMap<String, SlidingWindowState>>>,
    /// Token bucket states
    token_bucket: Arc<RwLock<HashMap<String, TokenBucketState>>>,
    /// Concurrent states
    concurrent: Arc<RwLock<HashMap<String, ConcurrentState>>>,
    /// Total allowed requests
    total_allowed: Arc<AtomicU64>,
    /// Total denied requests
    total_denied: Arc<AtomicU64>,
}

impl AdvancedRateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            fixed_window: Arc::new(RwLock::new(HashMap::new())),
            sliding_window: Arc::new(RwLock::new(HashMap::new())),
            token_bucket: Arc::new(RwLock::new(HashMap::new())),
            concurrent: Arc::new(RwLock::new(HashMap::new())),
            total_allowed: Arc::new(AtomicU64::new(0)),
            total_denied: Arc::new(AtomicU64::new(0)),
        }
    }
    
    /// Check if a request is allowed
    pub fn check(&self, identifier: &str) -> RateLimitResult {
        let key = format!("{}{}", self.config.key_prefix, identifier);
        
        let result = match self.config.strategy {
            RateLimitStrategy::FixedWindow => self.check_fixed_window(&key),
            RateLimitStrategy::SlidingWindow => self.check_sliding_window(&key),
            RateLimitStrategy::TokenBucket => self.check_token_bucket(&key),
            RateLimitStrategy::LeakyBucket => self.check_leaky_bucket(&key),
            RateLimitStrategy::Concurrent => self.check_concurrent(&key),
        };
        
        if result.allowed {
            self.total_allowed.fetch_add(1, Ordering::Relaxed);
        } else {
            self.total_denied.fetch_add(1, Ordering::Relaxed);
        }
        
        result
    }
    
    /// Release a concurrent slot (for Concurrent strategy)
    pub fn release(&self, identifier: &str) {
        if self.config.strategy == RateLimitStrategy::Concurrent {
            let key = format!("{}{}", self.config.key_prefix, identifier);
            let mut states = self.concurrent.write();
            
            if let Some(state) = states.get_mut(&key) {
                if state.current > 0 {
                    state.current -= 1;
                }
            }
        }
    }
    
    /// Fixed window algorithm
    fn check_fixed_window(&self, key: &str) -> RateLimitResult {
        let window_duration = Duration::from_secs(self.config.window_secs);
        let now = Instant::now();
        
        let mut states = self.fixed_window.write();
        
        let state = states.entry(key.to_string()).or_insert(FixedWindowState {
            count: 0,
            window_start: now,
        });
        
        // Check if window has expired
        if now.duration_since(state.window_start) >= window_duration {
            state.count = 0;
            state.window_start = now;
        }
        
        if state.count >= self.config.max_requests {
            let elapsed = now.duration_since(state.window_start);
            let retry_after = window_duration.saturating_sub(elapsed).as_secs();
            return RateLimitResult::denied(self.config.max_requests, retry_after);
        }
        
        state.count += 1;
        RateLimitResult::allowed(self.config.max_requests, self.config.max_requests - state.count)
    }
    
    /// Sliding window algorithm
    fn check_sliding_window(&self, key: &str) -> RateLimitResult {
        let window_duration = Duration::from_secs(self.config.window_secs);
        let now = Instant::now();
        
        let mut states = self.sliding_window.write();
        
        let state = states.entry(key.to_string()).or_insert(SlidingWindowState {
            timestamps: Vec::new(),
        });
        
        // Remove timestamps outside the window
        state.timestamps.retain(|&t| now.duration_since(t) < window_duration);
        
        if state.timestamps.len() as u32 >= self.config.max_requests {
            let oldest = state.timestamps[0];
            let retry_after = window_duration.saturating_sub(now.duration_since(oldest)).as_secs();
            return RateLimitResult::denied(self.config.max_requests, retry_after);
        }
        
        state.timestamps.push(now);
        RateLimitResult::allowed(self.config.max_requests, self.config.max_requests - state.timestamps.len() as u32)
    }
    
    /// Token bucket algorithm
    fn check_token_bucket(&self, key: &str) -> RateLimitResult {
        let now = Instant::now();
        let refill_rate = self.config.refill_rate.unwrap_or(1.0);
        let max_tokens = self.config.max_requests as f64;
        
        let mut states = self.token_bucket.write();
        
        let state = states.entry(key.to_string()).or_insert(TokenBucketState {
            tokens: max_tokens,
            last_refill: now,
        });
        
        // Refill tokens
        let elapsed = now.duration_since(state.last_refill).as_secs_f64();
        state.tokens = (state.tokens + elapsed * refill_rate).min(max_tokens);
        state.last_refill = now;
        
        if state.tokens < 1.0 {
            let needed = 1.0 - state.tokens;
            let retry_after = (needed / refill_rate).ceil() as u64;
            return RateLimitResult::denied(self.config.max_requests, retry_after);
        }
        
        state.tokens -= 1.0;
        RateLimitResult::allowed(self.config.max_requests, state.tokens.floor() as u32)
    }
    
    /// Leaky bucket algorithm
    fn check_leaky_bucket(&self, key: &str) -> RateLimitResult {
        // Leaky bucket is similar to token bucket but inverted
        // We use the sliding window for this
        self.check_sliding_window(key)
    }
    
    /// Concurrent request limit
    fn check_concurrent(&self, key: &str) -> RateLimitResult {
        let max_concurrent = self.config.max_concurrent.unwrap_or(self.config.max_requests);
        
        let mut states = self.concurrent.write();
        
        let state = states.entry(key.to_string()).or_insert(ConcurrentState {
            current: 0,
        });
        
        if state.current >= max_concurrent {
            return RateLimitResult::denied(max_concurrent, 1);
        }
        
        state.current += 1;
        RateLimitResult::allowed(max_concurrent, max_concurrent - state.current)
    }
    
    /// Clean up expired entries
    pub fn cleanup(&self) -> usize {
        let window_duration = Duration::from_secs(self.config.window_secs);
        let now = Instant::now();
        let mut cleaned = 0;
        
        // Clean sliding window
        {
            let mut states = self.sliding_window.write();
            states.retain(|_, state| {
                state.timestamps.retain(|&t| now.duration_since(t) < window_duration);
                let keep = !state.timestamps.is_empty();
                if !keep { cleaned += 1; }
                keep
            });
        }
        
        // Clean fixed window
        {
            let mut states = self.fixed_window.write();
            states.retain(|_, state| {
                let keep = now.duration_since(state.window_start) < window_duration;
                if !keep { cleaned += 1; }
                keep
            });
        }
        
        cleaned
    }
    
    /// Get statistics
    pub fn stats(&self) -> RateLimiterStats {
        RateLimiterStats {
            strategy: format!("{:?}", self.config.strategy),
            max_requests: self.config.max_requests,
            window_secs: self.config.window_secs,
            total_allowed: self.total_allowed.load(Ordering::Relaxed),
            total_denied: self.total_denied.load(Ordering::Relaxed),
            tracked_keys: match self.config.strategy {
                RateLimitStrategy::FixedWindow => self.fixed_window.read().len(),
                RateLimitStrategy::SlidingWindow | RateLimitStrategy::LeakyBucket => self.sliding_window.read().len(),
                RateLimitStrategy::TokenBucket => self.token_bucket.read().len(),
                RateLimitStrategy::Concurrent => self.concurrent.read().len(),
            },
        }
    }
    
    /// Reset all rate limit data
    pub fn reset(&self) {
        self.fixed_window.write().clear();
        self.sliding_window.write().clear();
        self.token_bucket.write().clear();
        self.concurrent.write().clear();
        self.total_allowed.store(0, Ordering::Relaxed);
        self.total_denied.store(0, Ordering::Relaxed);
    }
}

/// Rate limiter statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimiterStats {
    pub strategy: String,
    pub max_requests: u32,
    pub window_secs: u64,
    pub total_allowed: u64,
    pub total_denied: u64,
    pub tracked_keys: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fixed_window() {
        let limiter = AdvancedRateLimiter::new(RateLimitConfig {
            strategy: RateLimitStrategy::FixedWindow,
            max_requests: 3,
            window_secs: 60,
            ..Default::default()
        });
        
        assert!(limiter.check("user1").allowed);
        assert!(limiter.check("user1").allowed);
        assert!(limiter.check("user1").allowed);
        assert!(!limiter.check("user1").allowed);
    }
    
    #[test]
    fn test_sliding_window() {
        let limiter = AdvancedRateLimiter::new(RateLimitConfig {
            strategy: RateLimitStrategy::SlidingWindow,
            max_requests: 3,
            window_secs: 60,
            ..Default::default()
        });
        
        assert!(limiter.check("user1").allowed);
        assert!(limiter.check("user1").allowed);
        assert!(limiter.check("user1").allowed);
        assert!(!limiter.check("user1").allowed);
        
        // Different user should have separate limit
        assert!(limiter.check("user2").allowed);
    }
    
    #[test]
    fn test_token_bucket() {
        let limiter = AdvancedRateLimiter::new(RateLimitConfig {
            strategy: RateLimitStrategy::TokenBucket,
            max_requests: 5,
            window_secs: 60,
            refill_rate: Some(1.0), // 1 token per second
            ..Default::default()
        });
        
        // Should allow 5 requests immediately
        for _ in 0..5 {
            assert!(limiter.check("user1").allowed);
        }
        
        // 6th should be denied
        assert!(!limiter.check("user1").allowed);
    }
    
    #[test]
    fn test_concurrent() {
        let limiter = AdvancedRateLimiter::new(RateLimitConfig {
            strategy: RateLimitStrategy::Concurrent,
            max_requests: 2,
            max_concurrent: Some(2),
            ..Default::default()
        });
        
        assert!(limiter.check("user1").allowed);
        assert!(limiter.check("user1").allowed);
        assert!(!limiter.check("user1").allowed);
        
        // Release one
        limiter.release("user1");
        
        // Should be allowed again
        assert!(limiter.check("user1").allowed);
    }
    
    #[test]
    fn test_stats() {
        let limiter = AdvancedRateLimiter::new(RateLimitConfig::default());
        
        limiter.check("user1");
        limiter.check("user1");
        limiter.check("user2");
        
        let stats = limiter.stats();
        assert_eq!(stats.total_allowed, 3);
        assert_eq!(stats.total_denied, 0);
    }
    
    #[test]
    fn test_reset() {
        let limiter = AdvancedRateLimiter::new(RateLimitConfig::default());
        
        limiter.check("user1");
        limiter.check("user2");
        
        let stats = limiter.stats();
        assert!(stats.tracked_keys > 0);
        
        limiter.reset();
        
        let stats = limiter.stats();
        assert_eq!(stats.tracked_keys, 0);
    }
}
