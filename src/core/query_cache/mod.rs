// ============================================================
// Query Result Cache - تخزين نتائج الاستعلام
// ============================================================
// Cache database query results to improve performance.
// Automatically invalidates on writes.
//
// تخزين نتائج استعلامات قاعدة البيانات.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::{Duration, Instant};

/// Cached query result
#[derive(Debug, Clone)]
struct CachedResult {
    data: serde_json::Value,
    cached_at: Instant,
    ttl: Duration,
}

impl CachedResult {
    fn is_expired(&self) -> bool {
        self.cached_at.elapsed() >= self.ttl
    }
}

/// Query result cache
pub struct QueryCache {
    cache: Arc<RwLock<HashMap<String, CachedResult>>>,
    default_ttl: Duration,
    /// Tables that have been modified (for invalidation)
    dirty_tables: Arc<RwLock<Vec<String>>>,
    /// Map of cache keys to tables they depend on
    key_tables: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Statistics
    hits: Arc<std::sync::atomic::AtomicU64>,
    misses: Arc<std::sync::atomic::AtomicU64>,
}

impl Default for QueryCache {
    fn default() -> Self {
        Self::new(Duration::from_secs(300)) // 5 minutes default
    }
}

impl QueryCache {
    pub fn new(default_ttl: Duration) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            default_ttl,
            dirty_tables: Arc::new(RwLock::new(Vec::new())),
            key_tables: Arc::new(RwLock::new(HashMap::new())),
            hits: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            misses: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }
    
    /// Generate a cache key from SQL and params
    pub fn make_key(sql: &str, params: &[serde_json::Value]) -> String {
        let params_str = serde_json::to_string(params).unwrap_or_default();
        crate::core::security::Encryption::sha256_hex(format!("{}|{}", sql, params_str).as_bytes())
    }
    
    /// Get a cached result
    pub fn get(&self, key: &str) -> Option<serde_json::Value> {
        let cache = self.cache.read();
        
        if let Some(cached) = cache.get(key) {
            if !cached.is_expired() {
                self.hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return Some(cached.data.clone());
            }
        }
        
        self.misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        None
    }
    
    /// Store a result in the cache
    pub fn put(&self, key: &str, data: serde_json::Value, tables: Vec<String>) {
        self.put_with_ttl(key, data, self.default_ttl, tables);
    }
    
    /// Store a result with a custom TTL
    pub fn put_with_ttl(&self, key: &str, data: serde_json::Value, ttl: Duration, tables: Vec<String>) {
        let cached = CachedResult {
            data,
            cached_at: Instant::now(),
            ttl,
        };
        
        self.cache.write().insert(key.to_string(), cached);
        self.key_tables.write().insert(key.to_string(), tables);
    }
    
    /// Invalidate cache for a specific table
    pub fn invalidate_table(&self, table: &str) -> usize {
        let mut invalidated = 0;
        
        let keys_to_remove: Vec<String> = {
            let key_tables = self.key_tables.read();
            key_tables
                .iter()
                .filter(|(_, tables)| tables.contains(&table.to_string()))
                .map(|(key, _)| key.clone())
                .collect()
        };
        
        for key in keys_to_remove {
            self.cache.write().remove(&key);
            self.key_tables.write().remove(&key);
            invalidated += 1;
        }
        
        invalidated
    }
    
    /// Invalidate a specific cache key
    pub fn forget(&self, key: &str) -> bool {
        let removed = self.cache.write().remove(key).is_some();
        self.key_tables.write().remove(key);
        removed
    }
    
    /// Clear all cached results
    pub fn flush(&self) -> usize {
        let count = self.cache.read().len();
        self.cache.write().clear();
        self.key_tables.write().clear();
        count
    }
    
    /// Mark a table as modified (triggers invalidation)
    pub fn table_modified(&self, table: &str) {
        self.invalidate_table(table);
        self.dirty_tables.write().push(table.to_string());
    }
    
    /// Get cache statistics
    pub fn stats(&self) -> QueryCacheStats {
        let hits = self.hits.load(std::sync::atomic::Ordering::Relaxed);
        let misses = self.misses.load(std::sync::atomic::Ordering::Relaxed);
        let total = hits + misses;
        
        QueryCacheStats {
            cached_queries: self.cache.read().len(),
            hits,
            misses,
            hit_rate: if total == 0 { 0.0 } else { (hits as f64 / total as f64) * 100.0 },
            dirty_tables: self.dirty_tables.read().len(),
        }
    }
    
    /// Clean up expired entries
    pub fn cleanup(&self) -> usize {
        let mut cleaned = 0;
        
        let keys_to_remove: Vec<String> = {
            let cache = self.cache.read();
            cache
                .iter()
                .filter(|(_, cached)| cached.is_expired())
                .map(|(key, _)| key.clone())
                .collect()
        };
        
        for key in keys_to_remove {
            self.cache.write().remove(&key);
            self.key_tables.write().remove(&key);
            cleaned += 1;
        }
        
        cleaned
    }
    
    /// Execute a query with caching
    pub fn remember<F>(&self, sql: &str, params: &[serde_json::Value], tables: Vec<String>, f: F) -> crate::NoorResult<serde_json::Value>
    where
        F: FnOnce() -> crate::NoorResult<serde_json::Value>,
    {
        let key = Self::make_key(sql, params);
        
        if let Some(cached) = self.get(&key) {
            return Ok(cached);
        }
        
        let result = f()?;
        self.put(&key, result.clone(), tables);
        
        Ok(result)
    }
    
    /// Execute a query with caching and custom TTL
    pub fn remember_for<F>(&self, sql: &str, params: &[serde_json::Value], ttl: Duration, tables: Vec<String>, f: F) -> crate::NoorResult<serde_json::Value>
    where
        F: FnOnce() -> crate::NoorResult<serde_json::Value>,
    {
        let key = Self::make_key(sql, params);
        
        if let Some(cached) = self.get(&key) {
            return Ok(cached);
        }
        
        let result = f()?;
        self.put_with_ttl(&key, result.clone(), ttl, tables);
        
        Ok(result)
    }
}

/// Query cache statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct QueryCacheStats {
    pub cached_queries: usize,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub dirty_tables: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cache_put_get() {
        let cache = QueryCache::default();
        
        cache.put("key1", serde_json::json!({"data": "test"}), vec!["users".to_string()]);
        
        let result = cache.get("key1");
        assert!(result.is_some());
        assert_eq!(result.unwrap()["data"], "test");
    }
    
    #[test]
    fn test_cache_miss() {
        let cache = QueryCache::default();
        
        let result = cache.get("nonexistent");
        assert!(result.is_none());
    }
    
    #[test]
    fn test_invalidate_table() {
        let cache = QueryCache::default();
        
        cache.put("key1", serde_json::json!({}), vec!["users".to_string()]);
        cache.put("key2", serde_json::json!({}), vec!["posts".to_string()]);
        
        assert_eq!(cache.stats().cached_queries, 2);
        
        let invalidated = cache.invalidate_table("users");
        assert_eq!(invalidated, 1);
        assert_eq!(cache.stats().cached_queries, 1);
    }
    
    #[test]
    fn test_remember() {
        let cache = QueryCache::default();
        let call_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let call_count_clone = call_count.clone();
        
        let result = cache.remember("SELECT * FROM users", &[], vec!["users".to_string()], || {
            call_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(serde_json::json!([{"id": 1}]))
        }).unwrap();
        
        assert_eq!(result[0]["id"], 1);
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 1);
        
        // Second call should use cache
        let result2 = cache.remember("SELECT * FROM users", &[], vec!["users".to_string()], || {
            call_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(serde_json::json!([{"id": 2}]))
        }).unwrap();
        
        assert_eq!(result2[0]["id"], 1); // Cached, not 2
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 1); // Not called again
    }
    
    #[test]
    fn test_table_modified_invalidates() {
        let cache = QueryCache::default();
        
        cache.put("key1", serde_json::json!({}), vec!["users".to_string()]);
        
        cache.table_modified("users");
        
        assert!(cache.get("key1").is_none());
    }
    
    #[test]
    fn test_stats() {
        let cache = QueryCache::default();
        
        cache.put("key1", serde_json::json!({}), vec!["users".to_string()]);
        
        cache.get("key1"); // Hit
        cache.get("key2"); // Miss
        
        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate - 50.0).abs() < 0.1);
    }
}
