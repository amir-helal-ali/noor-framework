// ============================================================
// Cache Manager - مدير التخزين المؤقت
// ============================================================
// Manages multiple cache drivers with automatic fallback.
// Memory cache first, file cache as fallback.
//
// يدير عدة drivers مع fallback تلقائي.
// ============================================================

use std::sync::Arc;
use crate::core::cache::{Cache, FileCache, MemoryCache};

/// Cache manager with automatic driver selection
/// مدير cache مع اختيار تلقائي للـ driver
pub struct CacheManager {
    primary: Arc<dyn Cache>,
    fallback: Option<Arc<dyn Cache>>,
}

impl CacheManager {
    /// Create a cache manager optimized for weak servers
    /// إنشاء مدير cache محسن للسيرفرات الضعيفة
    pub fn for_weak_server(cache_dir: &str) -> crate::NoorResult<Self> {
        let file_cache = Arc::new(FileCache::new(cache_dir, "noor:")?);
        Ok(Self {
            primary: file_cache,
            fallback: None,
        })
    }
    
    /// Create a cache manager with memory + file fallback
    /// إنشاء مدير cache مع memory + file fallback
    pub fn with_fallback(cache_dir: &str, memory_size: usize) -> crate::NoorResult<Self> {
        let memory_cache = Arc::new(MemoryCache::new(memory_size, "noor:"));
        let file_cache = Arc::new(FileCache::new(cache_dir, "noor:")?);
        
        Ok(Self {
            primary: memory_cache,
            fallback: Some(file_cache),
        })
    }
    
    /// Create with only memory cache (high performance)
    pub fn memory_only(memory_size: usize) -> Self {
        Self {
            primary: Arc::new(MemoryCache::new(memory_size, "noor:")),
            fallback: None,
        }
    }
    
    /// Get a value, checking primary then fallback
    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        if let Some(value) = self.primary.get(key) {
            return Some(value);
        }
        
        if let Some(ref fallback) = self.fallback {
            if let Some(value) = fallback.get(key) {
                // Cache it in primary for next time
                let _ = self.primary.set(key, &value, 3600);
                return Some(value);
            }
        }
        
        None
    }
    
    /// Get a value and deserialize it
    pub fn get_json<T: for<'de> serde::Deserialize<'de>>(&self, key: &str) -> Option<T> {
        self.get(key)
            .and_then(|v| serde_json::from_slice(&v).ok())
    }
    
    /// Set a value in both primary and fallback
    pub fn set(&self, key: &str, value: &[u8], ttl_secs: u64) -> crate::NoorResult<()> {
        self.primary.set(key, value, ttl_secs)?;
        
        if let Some(ref fallback) = self.fallback {
            fallback.set(key, value, ttl_secs)?;
        }
        
        Ok(())
    }
    
    /// Set a serializable value
    pub fn set_json<T: serde::Serialize>(&self, key: &str, value: &T, ttl_secs: u64) -> crate::NoorResult<()> {
        let bytes = serde_json::to_vec(value)?;
        self.set(key, &bytes, ttl_secs)
    }
    
    /// Delete a value from both caches
    pub fn delete(&self, key: &str) -> crate::NoorResult<()> {
        self.primary.delete(key)?;
        
        if let Some(ref fallback) = self.fallback {
            fallback.delete(key)?;
        }
        
        Ok(())
    }
    
    /// Check if a key exists in any cache
    pub fn has(&self, key: &str) -> bool {
        if self.primary.has(key) {
            return true;
        }
        
        if let Some(ref fallback) = self.fallback {
            return fallback.has(key);
        }
        
        false
    }
    
    /// Clear all caches
    pub fn clear(&self) -> crate::NoorResult<()> {
        self.primary.clear()?;
        
        if let Some(ref fallback) = self.fallback {
            fallback.clear()?;
        }
        
        Ok(())
    }
    
    /// Get or compute a value (cache-aside pattern)
    pub fn remember<T, F>(&self, key: &str, ttl_secs: u64, compute: F) -> crate::NoorResult<T>
    where
        T: serde::Serialize + for<'de> serde::Deserialize<'de>,
        F: FnOnce() -> crate::NoorResult<T>,
    {
        if let Some(value) = self.get_json::<T>(key) {
            return Ok(value);
        }
        
        let value = compute()?;
        self.set_json(key, &value, ttl_secs)?;
        
        Ok(value)
    }
    
    /// Get the primary driver name
    pub fn driver_name(&self) -> &str {
        self.primary.driver_name()
    }
}
