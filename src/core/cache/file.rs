// ============================================================
// File Cache - التخزين المؤقت الملفي
// ============================================================
// File-based cache that works on any server, even shared hosting.
// Uses a hash of the key as the filename.
// Each cache entry is a JSON file with value + expiry.
//
// تخزين مؤقت ملفي يعمل على أي سيرفر.
// ============================================================

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use crate::core::cache::Cache;

/// File-based cache entry
/// إدخال cache ملفي
#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry {
    value: Vec<u8>,
    expires_at: i64,
}

/// File-based cache driver
/// driver cache ملفي
pub struct FileCache {
    cache_dir: PathBuf,
    prefix: String,
    /// In-memory index of keys to file paths (for faster lookups)
    index: Arc<RwLock<HashMap<String, PathBuf>>>,
}

impl FileCache {
    /// Create a new file cache
    pub fn new(cache_dir: &str, prefix: &str) -> crate::NoorResult<Self> {
        let path = PathBuf::from(cache_dir);
        std::fs::create_dir_all(&path)?;
        
        Ok(Self {
            cache_dir: path,
            prefix: prefix.to_string(),
            index: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// Get the file path for a cache key
    fn key_to_path(&self, key: &str) -> PathBuf {
        let hash = crate::core::security::Encryption::sha256_hex(key.as_bytes());
        // Use first 2 chars as directory to avoid too many files in one dir
        let dir = &hash[..2];
        let filename = &hash[2..];
        
        let mut path = self.cache_dir.clone();
        path.push(dir);
        path.push(format!("{}.cache", filename));
        path
    }
    
    /// Build the full cache key with prefix
    fn build_key(&self, key: &str) -> String {
        format!("{}{}", self.prefix, key)
    }
    
    /// Read a cache entry from disk
    fn read_entry(&self, path: &Path) -> Option<CacheEntry> {
        let content = std::fs::read(path).ok()?;
        serde_json::from_slice(&content).ok()
    }
    
    /// Write a cache entry to disk
    fn write_entry(&self, path: &Path, entry: &CacheEntry) -> crate::NoorResult<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_vec(entry)?;
        std::fs::write(path, content)?;
        Ok(())
    }
    
    /// Clean up expired entries (call periodically to save disk space)
    /// تنظيف الإدخالات المنتهية
    pub fn gc(&self) -> crate::NoorResult<usize> {
        let now = chrono::Utc::now().timestamp();
        let mut cleaned = 0;
        
        self.gc_dir(&self.cache_dir, now, &mut cleaned)?;
        
        Ok(cleaned)
    }
    
    fn gc_dir(&self, dir: &Path, now: i64, cleaned: &mut usize) -> crate::NoorResult<()> {
        if !dir.exists() {
            return Ok(());
        }
        
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                self.gc_dir(&path, now, cleaned)?;
            } else if path.extension().map(|e| e == "cache").unwrap_or(false) {
                if let Some(entry) = self.read_entry(&path) {
                    if entry.expires_at < now {
                        std::fs::remove_file(&path).ok();
                        *cleaned += 1;
                    }
                }
            }
        }
        
        Ok(())
    }
}

impl Cache for FileCache {
    fn get(&self, key: &str) -> Option<Vec<u8>> {
        let full_key = self.build_key(key);
        let path = self.key_to_path(&full_key);
        
        let entry = self.read_entry(&path)?;
        
        // Check expiry
        let now = chrono::Utc::now().timestamp();
        if entry.expires_at < now {
            // Delete expired entry
            std::fs::remove_file(&path).ok();
            return None;
        }
        
        Some(entry.value)
    }
    
    fn set(&self, key: &str, value: &[u8], ttl_secs: u64) -> crate::NoorResult<()> {
        let full_key = self.build_key(key);
        let path = self.key_to_path(&full_key);
        
        let entry = CacheEntry {
            value: value.to_vec(),
            expires_at: chrono::Utc::now().timestamp() + ttl_secs as i64,
        };
        
        self.write_entry(&path, &entry)?;
        
        // Update index
        self.index.write().insert(full_key, path);
        
        Ok(())
    }
    
    fn delete(&self, key: &str) -> crate::NoorResult<()> {
        let full_key = self.build_key(key);
        let path = self.key_to_path(&full_key);
        
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        
        self.index.write().remove(&full_key);
        
        Ok(())
    }
    
    fn clear(&self) -> crate::NoorResult<()> {
        if self.cache_dir.exists() {
            std::fs::remove_dir_all(&self.cache_dir)?;
            std::fs::create_dir_all(&self.cache_dir)?;
        }
        self.index.write().clear();
        Ok(())
    }
    
    fn driver_name(&self) -> &str {
        "file"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_file_cache() {
        let cache = FileCache::new("/tmp/noor_test_cache", "test:").unwrap();
        
        cache.set("key1", b"value1", 60).unwrap();
        assert_eq!(cache.get("key1"), Some(b"value1".to_vec()));
        
        cache.delete("key1").unwrap();
        assert_eq!(cache.get("key1"), None);
    }
}
