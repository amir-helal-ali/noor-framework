// ============================================================
// Idempotency - التساوي القوي
// ============================================================
// Ensures that multiple identical requests have the same effect
// as a single request. Prevents duplicate operations.
//
// يضمن أن الطلبات المتكررة لها نفس تأثير الطلب الواحد.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// Idempotency record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdempotencyRecord {
    pub key: String,
    pub status: IdempotencyStatus,
    pub response: Option<serde_json::Value>,
    pub created_at: i64,
    pub expires_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IdempotencyStatus {
    /// Request is being processed
    InProgress,
    /// Request completed successfully
    Completed,
    /// Request failed
    Failed,
}

impl IdempotencyRecord {
    pub fn is_expired(&self) -> bool {
        // A record whose `expires_at` equals the current second should be
        // considered expired (standard "expired at or after T" convention).
        // Using `>=` instead of `>` also makes TTL=0 records expire
        // immediately, which is the expected behaviour.
        chrono::Utc::now().timestamp() >= self.expires_at
    }
}

/// Idempotency manager
pub struct IdempotencyManager {
    records: Arc<RwLock<HashMap<String, IdempotencyRecord>>>,
    default_ttl: i64,
}

impl Default for IdempotencyManager {
    fn default() -> Self {
        Self::new(3600) // 1 hour default TTL
    }
}

impl IdempotencyManager {
    pub fn new(default_ttl_secs: i64) -> Self {
        Self {
            records: Arc::new(RwLock::new(HashMap::new())),
            default_ttl: default_ttl_secs,
        }
    }
    
    /// Check if a request is idempotent
    pub fn check(&self, key: &str) -> IdempotencyCheck {
        let mut records = self.records.write();
        
        // Check if record exists
        if let Some(record) = records.get(key) {
            if !record.is_expired() {
                match record.status {
                    IdempotencyStatus::InProgress => {
                        return IdempotencyCheck::InProgress;
                    }
                    IdempotencyStatus::Completed => {
                        return IdempotencyCheck::Completed(record.response.clone());
                    }
                    IdempotencyStatus::Failed => {
                        // Allow retry
                        let now = chrono::Utc::now().timestamp();
                        let record = IdempotencyRecord {
                            key: key.to_string(),
                            status: IdempotencyStatus::InProgress,
                            response: None,
                            created_at: now,
                            expires_at: now + self.default_ttl,
                        };
                        records.insert(key.to_string(), record);
                        return IdempotencyCheck::Proceed;
                    }
                }
            }
        }
        
        // Create new record
        let now = chrono::Utc::now().timestamp();
        let record = IdempotencyRecord {
            key: key.to_string(),
            status: IdempotencyStatus::InProgress,
            response: None,
            created_at: now,
            expires_at: now + self.default_ttl,
        };
        
        records.insert(key.to_string(), record);
        
        IdempotencyCheck::Proceed
    }
    
    /// Mark a request as completed
    pub fn complete(&self, key: &str, response: serde_json::Value) -> crate::NoorResult<()> {
        let mut records = self.records.write();
        
        if let Some(record) = records.get_mut(key) {
            record.status = IdempotencyStatus::Completed;
            record.response = Some(response);
            return Ok(());
        }
        
        Err(crate::NoorError::Internal(format!("Idempotency key '{}' not found", key)))
    }
    
    /// Mark a request as failed
    pub fn fail(&self, key: &str) -> crate::NoorResult<()> {
        let mut records = self.records.write();
        
        if let Some(record) = records.get_mut(key) {
            record.status = IdempotencyStatus::Failed;
            return Ok(());
        }
        
        Err(crate::NoorError::Internal(format!("Idempotency key '{}' not found", key)))
    }
    
    /// Execute a function idempotently
    pub fn execute<F, T>(&self, key: &str, f: F) -> crate::NoorResult<T>
    where
        T: Serialize + for<'de> Deserialize<'de>,
        F: FnOnce() -> crate::NoorResult<T>,
    {
        match self.check(key) {
            IdempotencyCheck::Proceed => {
                match f() {
                    Ok(result) => {
                        let response = serde_json::to_value(&result)?;
                        self.complete(key, response)?;
                        Ok(result)
                    }
                    Err(e) => {
                        self.fail(key).ok();
                        Err(e)
                    }
                }
            }
            IdempotencyCheck::InProgress => {
                Err(crate::NoorError::Internal("Request is already in progress".to_string()))
            }
            IdempotencyCheck::Completed(cached_response) => {
                let result: T = serde_json::from_value(cached_response.unwrap_or(serde_json::Value::Null))?;
                Ok(result)
            }
        }
    }
    
    /// Remove a key
    pub fn remove(&self, key: &str) -> bool {
        self.records.write().remove(key).is_some()
    }
    
    /// Clean up expired records
    pub fn cleanup(&self) -> usize {
        let mut records = self.records.write();
        let initial = records.len();
        records.retain(|_, r| !r.is_expired());
        initial - records.len()
    }
    
    /// Get record count
    pub fn count(&self) -> usize {
        self.records.read().len()
    }
    
    /// Clear all records
    pub fn clear(&self) {
        self.records.write().clear();
    }
}

/// Result of idempotency check
pub enum IdempotencyCheck {
    /// Proceed with the request
    Proceed,
    /// Request is already in progress
    InProgress,
    /// Request was already completed (with cached response)
    Completed(Option<serde_json::Value>),
}

/// Idempotency key extractor
pub fn extract_key(request: &crate::core::http::Request) -> Option<String> {
    request.header("idempotency-key").map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    
    #[test]
    fn test_first_request_proceeds() {
        let manager = IdempotencyManager::default();
        
        let check = manager.check("key1");
        
        match check {
            IdempotencyCheck::Proceed => {}
            _ => panic!("Expected Proceed"),
        }
    }
    
    #[test]
    fn test_duplicate_returns_cached() {
        let manager = IdempotencyManager::default();
        
        // First request
        manager.check("key1");
        manager.complete("key1", serde_json::json!({"result": "success"})).unwrap();
        
        // Second request with same key
        let check = manager.check("key1");
        
        match check {
            IdempotencyCheck::Completed(response) => {
                assert_eq!(response.unwrap()["result"], "success");
            }
            _ => panic!("Expected Completed"),
        }
    }
    
    #[test]
    fn test_in_progress_status() {
        let manager = IdempotencyManager::default();
        
        // First request starts
        manager.check("key1");
        
        // Second request should see InProgress
        let check = manager.check("key1");
        
        match check {
            IdempotencyCheck::InProgress => {}
            _ => panic!("Expected InProgress"),
        }
    }
    
    #[test]
    fn test_failed_allows_retry() {
        let manager = IdempotencyManager::default();
        
        // First request fails
        manager.check("key1");
        manager.fail("key1").unwrap();
        
        // Second request should be allowed
        let check = manager.check("key1");
        
        match check {
            IdempotencyCheck::Proceed => {}
            _ => panic!("Expected Proceed after failure"),
        }
    }
    
    #[test]
    fn test_execute_idempotent() {
        let manager = IdempotencyManager::default();
        let counter = Arc::new(AtomicU32::new(0));
        
        let counter_clone = counter.clone();
        let result1: i32 = manager.execute("key1", || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            Ok(42)
        }).unwrap();
        
        let counter_clone = counter.clone();
        let result2: i32 = manager.execute("key1", || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            Ok(99) // Should not be called
        }).unwrap();
        
        assert_eq!(result1, 42);
        assert_eq!(result2, 42); // Cached result
        assert_eq!(counter.load(Ordering::SeqCst), 1); // Function called only once
    }
    
    #[test]
    fn test_cleanup_expired() {
        let manager = IdempotencyManager::new(0); // 0 TTL = immediately expired
        
        manager.check("key1");
        manager.complete("key1", serde_json::json!({})).unwrap();
        
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        let cleaned = manager.cleanup();
        assert_eq!(cleaned, 1);
        assert_eq!(manager.count(), 0);
    }
}
