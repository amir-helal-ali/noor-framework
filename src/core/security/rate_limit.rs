// ============================================================
// Rate Limiting - تحديد المعدل
// ============================================================
// Protects against brute force and DDoS attacks by limiting
// the number of requests per IP within a time window.
//
// يحمي من هجمات brute force و DDoS.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::Mutex;
use crate::NoorResult;

/// Rate limiter with sliding window algorithm
/// محدد المعدل بنافذة منزلقة
pub struct RateLimit {
    /// Per-IP request tracking
    buckets: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
    /// Maximum requests allowed in the window
    max_requests: u32,
    /// Time window in seconds
    window_secs: u64,
}

impl RateLimit {
    pub fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            buckets: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            window_secs,
        }
    }
    
    /// Check if a request from the given IP is allowed
    /// فحص إذا كان الطلب من IP مسموح به
    pub fn check(&self, ip: &str) -> RateLimitResult {
        let window = Duration::from_secs(self.window_secs);
        let now = Instant::now();
        
        let mut buckets = self.buckets.lock();
        let requests = buckets.entry(ip.to_string()).or_insert_with(Vec::new);
        
        // Remove requests outside the window
        requests.retain(|t| now.duration_since(*t) < window);
        
        if requests.len() as u32 >= self.max_requests {
            let retry_after = self.window_secs - now.duration_since(requests[0]).as_secs();
            return RateLimitResult {
                allowed: false,
                remaining: 0,
                retry_after: Some(retry_after),
            };
        }
        
        requests.push(now);
        
        RateLimitResult {
            allowed: true,
            remaining: self.max_requests - requests.len() as u32,
            retry_after: None,
        }
    }
    
    /// Clean up old entries to save memory (especially important for weak servers)
    /// تنظيف الإدخالات القديمة لتوفير الذاكرة
    pub fn cleanup(&self) {
        let window = Duration::from_secs(self.window_secs);
        let now = Instant::now();
        
        let mut buckets = self.buckets.lock();
        buckets.retain(|_, requests| {
            requests.retain(|t| now.duration_since(*t) < window);
            !requests.is_empty()
        });
    }
    
    /// Get current usage for an IP
    /// الحصول على الاستخدام الحالي لـ IP
    pub fn current_usage(&self, ip: &str) -> u32 {
        let buckets = self.buckets.lock();
        buckets.get(ip).map(|v| v.len() as u32).unwrap_or(0)
    }
}

/// Result of a rate limit check
/// نتيجة فحص تحديد المعدل
#[derive(Debug, Clone)]
pub struct RateLimitResult {
    pub allowed: bool,
    pub remaining: u32,
    pub retry_after: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_rate_limit() {
        let limiter = RateLimit::new(3, 60);
        let ip = "192.168.1.1";
        
        assert!(limiter.check(ip).allowed);
        assert!(limiter.check(ip).allowed);
        assert!(limiter.check(ip).allowed);
        assert!(!limiter.check(ip).allowed);
    }
}
