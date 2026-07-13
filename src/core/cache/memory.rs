// ============================================================
// Memory Cache - التخزين المؤقت في الذاكرة
// ============================================================
// Fast in-memory cache with LRU eviction.
// Best for high-frequency reads on small datasets.
//
// تخزين مؤقت في الذاكرة مع إخلاء LRU.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::core::cache::Cache;

/// Memory cache entry
struct MemoryEntry {
    value: Vec<u8>,
    expires_at: i64,
    last_accessed: i64,
}

/// In-memory cache driver
pub struct MemoryCache {
    entries: Arc<RwLock<HashMap<String, MemoryEntry>>>,
    max_size: usize,
    prefix: String,
}

impl MemoryCache {
    pub fn new(max_size: usize, prefix: &str) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            max_size,
            prefix: prefix.to_string(),
        }
    }
    
    fn build_key(&self, key: &str) -> String {
        format!("{}{}", self.prefix, key)
    }
    
    /// Evict expired entries
    fn evict_expired(&self, entries: &mut HashMap<String, MemoryEntry>) {
        let now = chrono::Utc::now().timestamp();
        entries.retain(|_, entry| entry.expires_at > now);
    }
    
    /// Evict least recently used entries if over capacity
    fn evict_lru(&self, entries: &mut HashMap<String, MemoryEntry>) {
        while entries.len() > self.max_size {
            // Find LRU entry
            if let Some(lru_key) = entries
                .iter()
                .min_by_key(|(_, e)| e.last_accessed)
                .map(|(k, _)| k.clone())
            {
                entries.remove(&lru_key);
            } else {
                break;
            }
        }
    }
}

impl Cache for MemoryCache {
    fn get(&self, key: &str) -> Option<Vec<u8>> {
        let full_key = self.build_key(key);
        let now = chrono::Utc::now().timestamp();
        
        let mut entries = self.entries.write();
        
        if let Some(entry) = entries.get_mut(&full_key) {
            if entry.expires_at < now {
                entries.remove(&full_key);
                return None;
            }
            entry.last_accessed = now;
            return Some(entry.value.clone());
        }
        
        None
    }
    
    fn set(&self, key: &str, value: &[u8], ttl_secs: u64) -> crate::NoorResult<()> {
        let full_key = self.build_key(key);
        let now = chrono::Utc::now().timestamp();
        
        let mut entries = self.entries.write();
        
        // Evict if over capacity
        self.evict_expired(&mut entries);
        self.evict_lru(&mut entries);
        
        entries.insert(full_key, MemoryEntry {
            value: value.to_vec(),
            expires_at: now + ttl_secs as i64,
            last_accessed: now,
        });
        
        Ok(())
    }
    
    fn delete(&self, key: &str) -> crate::NoorResult<()> {
        let full_key = self.build_key(key);
        self.entries.write().remove(&full_key);
        Ok(())
    }
    
    fn clear(&self) -> crate::NoorResult<()> {
        self.entries.write().clear();
        Ok(())
    }
    
    fn driver_name(&self) -> &str {
        "memory"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_memory_cache() {
        let cache = MemoryCache::new(100, "test:");
        
        cache.set("key1", b"value1", 60).unwrap();
        assert_eq!(cache.get("key1"), Some(b"value1".to_vec()));
        
        cache.delete("key1").unwrap();
        assert_eq!(cache.get("key1"), None);
    }
}
