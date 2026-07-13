// ============================================================
// Distributed Lock - القفل الموزع
// ============================================================
// Distributed locking for coordinating access to shared resources
// across multiple instances. Supports file-based and Redis backends.
//
// قفل موزع لتنسيق الوصول للموارد المشتركة.
// ============================================================

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// Lock state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockInfo {
    pub key: String,
    pub owner: String,
    pub acquired_at: i64,
    pub expires_at: i64,
    pub timeout_secs: u64,
}

impl LockInfo {
    /// Check if the lock is expired
    pub fn is_expired(&self) -> bool {
        chrono::Utc::now().timestamp() > self.expires_at
    }
}

/// Lock acquisition result
pub enum LockResult {
    /// Lock was acquired
    Acquired(LockGuard),
    /// Lock is held by another owner
    LockedByOther(String),
    /// Failed to acquire lock
    Failed(String),
}

/// Lock guard (releases lock when dropped)
pub struct LockGuard {
    key: String,
    owner: String,
    lock_manager: Arc<dyn LockManager>,
}

impl LockGuard {
    /// Get the lock key
    pub fn key(&self) -> &str {
        &self.key
    }
    
    /// Get the lock owner
    pub fn owner(&self) -> &str {
        &self.owner
    }
    
    /// Release the lock
    pub fn release(self) {
        let _ = self.lock_manager.release(&self.key, &self.owner);
    }
    
    /// Extend the lock timeout
    pub fn extend(&self, additional_secs: u64) -> crate::NoorResult<()> {
        self.lock_manager.extend(&self.key, &self.owner, additional_secs)
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = self.lock_manager.release(&self.key, &self.owner);
    }
}

/// Lock manager trait
pub trait LockManager: Send + Sync {
    /// Acquire a lock
    fn acquire(&self, key: &str, owner: &str, timeout_secs: u64) -> crate::NoorResult<LockResult>;

    /// Release a lock
    fn release(&self, key: &str, owner: &str) -> crate::NoorResult<bool>;

    /// Extend a lock's timeout
    fn extend(&self, key: &str, owner: &str, additional_secs: u64) -> crate::NoorResult<()>;

    /// Get lock info
    fn get_info(&self, key: &str) -> Option<LockInfo>;

    /// Check if a lock is held
    fn is_locked(&self, key: &str) -> bool;

    /// Produce an `Arc<dyn LockManager>` handle to **this same manager**, so
    /// that a `LockGuard` can release the lock against the correct state when
    /// dropped. (Without this, guards would point at an empty freshly-built
    /// manager and never actually release anything.)
    fn arc_clone(&self) -> Arc<dyn LockManager>;
}

/// In-memory lock manager
///
/// `Clone` is cheap because the lock table lives behind an `Arc`, so clones
/// share the same state. This is what lets a `LockGuard` hold its own handle
/// back to the manager that actually owns its lock.
#[derive(Clone)]
pub struct MemoryLockManager {
    locks: Arc<RwLock<HashMap<String, LockInfo>>>,
}

impl Default for MemoryLockManager {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryLockManager {
    pub fn new() -> Self {
        Self {
            locks: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl LockManager for MemoryLockManager {
    fn acquire(&self, key: &str, owner: &str, timeout_secs: u64) -> crate::NoorResult<LockResult> {
        let mut locks = self.locks.write();
        
        // Check if lock exists and is not expired
        if let Some(existing) = locks.get(key) {
            if !existing.is_expired() {
                if existing.owner == owner {
                    // Same owner - extend the lock
                    let new_expiry = chrono::Utc::now().timestamp() + timeout_secs as i64;
                    let info = LockInfo {
                        key: key.to_string(),
                        owner: owner.to_string(),
                        acquired_at: existing.acquired_at,
                        expires_at: new_expiry,
                        timeout_secs,
                    };
                    locks.insert(key.to_string(), info.clone());
                    
                    return Ok(LockResult::Acquired(LockGuard {
                        key: key.to_string(),
                        owner: owner.to_string(),
                        lock_manager: self.arc_clone(),
                    }));
                } else {
                    return Ok(LockResult::LockedByOther(existing.owner.clone()));
                }
            }
        }

        // Acquire new lock
        let now = chrono::Utc::now().timestamp();
        let info = LockInfo {
            key: key.to_string(),
            owner: owner.to_string(),
            acquired_at: now,
            expires_at: now + timeout_secs as i64,
            timeout_secs,
        };

        locks.insert(key.to_string(), info);

        Ok(LockResult::Acquired(LockGuard {
            key: key.to_string(),
            owner: owner.to_string(),
            lock_manager: self.arc_clone(),
        }))
    }

    fn arc_clone(&self) -> Arc<dyn LockManager> {
        Arc::new(self.clone())
    }
    
    fn release(&self, key: &str, owner: &str) -> crate::NoorResult<bool> {
        let mut locks = self.locks.write();
        
        if let Some(lock) = locks.get(key) {
            if lock.owner == owner {
                locks.remove(key);
                return Ok(true);
            }
        }
        
        Ok(false)
    }
    
    fn extend(&self, key: &str, owner: &str, additional_secs: u64) -> crate::NoorResult<()> {
        let mut locks = self.locks.write();
        
        if let Some(lock) = locks.get_mut(key) {
            if lock.owner == owner {
                lock.expires_at += additional_secs as i64;
                lock.timeout_secs += additional_secs;
                return Ok(());
            }
        }
        
        Err(crate::NoorError::Internal("Lock not found or not owned by this owner".to_string()))
    }
    
    fn get_info(&self, key: &str) -> Option<LockInfo> {
        let locks = self.locks.read();
        locks.get(key).filter(|l| !l.is_expired()).cloned()
    }
    
    fn is_locked(&self, key: &str) -> bool {
        self.get_info(key).is_some()
    }
}

/// File-based lock manager (for distributed systems)
#[derive(Clone)]
pub struct FileLockManager {
    lock_dir: PathBuf,
}

impl FileLockManager {
    pub fn new(lock_dir: &str) -> crate::NoorResult<Self> {
        let path = PathBuf::from(lock_dir);
        std::fs::create_dir_all(&path)?;
        Ok(Self { lock_dir: path })
    }
    
    fn lock_path(&self, key: &str) -> PathBuf {
        let safe_key = key.replace('/', "_").replace('\\', "_");
        self.lock_dir.join(format!("{}.lock", safe_key))
    }
}

impl LockManager for FileLockManager {
    fn acquire(&self, key: &str, owner: &str, timeout_secs: u64) -> crate::NoorResult<LockResult> {
        let path = self.lock_path(key);
        
        // Check if lock file exists and is valid
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let lock_info: LockInfo = serde_json::from_str(&content)?;
            
            if !lock_info.is_expired() {
                if lock_info.owner == owner {
                    // Extend the lock
                    let now = chrono::Utc::now().timestamp();
                    let new_info = LockInfo {
                        key: key.to_string(),
                        owner: owner.to_string(),
                        acquired_at: lock_info.acquired_at,
                        expires_at: now + timeout_secs as i64,
                        timeout_secs,
                    };
                    std::fs::write(&path, serde_json::to_string(&new_info)?)?;
                    
                    return Ok(LockResult::Acquired(LockGuard {
                        key: key.to_string(),
                        owner: owner.to_string(),
                        lock_manager: self.arc_clone(),
                    }));
                } else {
                    return Ok(LockResult::LockedByOther(lock_info.owner));
                }
            }

            // Lock is expired, remove it
            std::fs::remove_file(&path).ok();
        }

        // Create new lock file
        let now = chrono::Utc::now().timestamp();
        let info = LockInfo {
            key: key.to_string(),
            owner: owner.to_string(),
            acquired_at: now,
            expires_at: now + timeout_secs as i64,
            timeout_secs,
        };

        std::fs::write(&path, serde_json::to_string(&info)?)?;

        Ok(LockResult::Acquired(LockGuard {
            key: key.to_string(),
            owner: owner.to_string(),
            lock_manager: self.arc_clone(),
        }))
    }

    fn arc_clone(&self) -> Arc<dyn LockManager> {
        Arc::new(self.clone())
    }
    
    fn release(&self, key: &str, owner: &str) -> crate::NoorResult<bool> {
        let path = self.lock_path(key);
        
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let lock_info: LockInfo = serde_json::from_str(&content)?;
            
            if lock_info.owner == owner {
                std::fs::remove_file(&path)?;
                return Ok(true);
            }
        }
        
        Ok(false)
    }
    
    fn extend(&self, key: &str, owner: &str, additional_secs: u64) -> crate::NoorResult<()> {
        let path = self.lock_path(key);
        
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let mut lock_info: LockInfo = serde_json::from_str(&content)?;
            
            if lock_info.owner == owner {
                lock_info.expires_at += additional_secs as i64;
                lock_info.timeout_secs += additional_secs;
                std::fs::write(&path, serde_json::to_string(&lock_info)?)?;
                return Ok(());
            }
        }
        
        Err(crate::NoorError::Internal("Lock not found or not owned".to_string()))
    }
    
    fn get_info(&self, key: &str) -> Option<LockInfo> {
        let path = self.lock_path(key);
        
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(info) = serde_json::from_str::<LockInfo>(&content) {
                    if !info.is_expired() {
                        return Some(info);
                    }
                    // Clean up expired lock
                    std::fs::remove_file(&path).ok();
                }
            }
        }
        
        None
    }
    
    fn is_locked(&self, key: &str) -> bool {
        self.get_info(key).is_some()
    }
}

/// Distributed lock helper for common patterns
pub struct DistributedLock;

impl DistributedLock {
    /// Execute a function with a lock
    pub fn with_lock<F, T>(
        manager: &dyn LockManager,
        key: &str,
        owner: &str,
        timeout_secs: u64,
        f: F,
    ) -> crate::NoorResult<T>
    where
        F: FnOnce() -> crate::NoorResult<T>,
    {
        match manager.acquire(key, owner, timeout_secs)? {
            LockResult::Acquired(guard) => {
                let result = f();
                guard.release();
                result
            }
            LockResult::LockedByOther(owner) => {
                Err(crate::NoorError::Internal(format!("Lock '{}' held by {}", key, owner)))
            }
            LockResult::Failed(msg) => {
                Err(crate::NoorError::Internal(msg))
            }
        }
    }
    
    /// Try to acquire a lock with retries
    pub fn try_with_retries<F, T>(
        manager: &dyn LockManager,
        key: &str,
        owner: &str,
        timeout_secs: u64,
        max_retries: u32,
        retry_delay_ms: u64,
        f: F,
    ) -> crate::NoorResult<T>
    where
        F: FnOnce() -> crate::NoorResult<T>,
    {
        let mut retries = 0;
        
        loop {
            match manager.acquire(key, owner, timeout_secs)? {
                LockResult::Acquired(guard) => {
                    let result = f();
                    guard.release();
                    return result;
                }
                LockResult::LockedByOther(_) => {
                    retries += 1;
                    if retries >= max_retries {
                        return Err(crate::NoorError::Internal(
                            format!("Failed to acquire lock after {} retries", max_retries)
                        ));
                    }
                    std::thread::sleep(Duration::from_millis(retry_delay_ms));
                }
                LockResult::Failed(msg) => {
                    return Err(crate::NoorError::Internal(msg));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_acquire_and_release() {
        let manager = MemoryLockManager::new();
        
        let result = manager.acquire("resource1", "worker1", 60).unwrap();
        
        match result {
            LockResult::Acquired(guard) => {
                assert!(manager.is_locked("resource1"));
                guard.release();
            }
            _ => panic!("Expected Acquired"),
        }
        
        assert!(!manager.is_locked("resource1"));
    }
    
    #[test]
    fn test_locked_by_other() {
        let manager = MemoryLockManager::new();
        
        let _guard = manager.acquire("resource1", "worker1", 60).unwrap();
        
        let result = manager.acquire("resource1", "worker2", 60).unwrap();
        
        match result {
            LockResult::LockedByOther(owner) => {
                assert_eq!(owner, "worker1");
            }
            _ => panic!("Expected LockedByOther"),
        }
    }
    
    #[test]
    fn test_lock_extension() {
        let manager = MemoryLockManager::new();
        
        let guard = match manager.acquire("resource1", "worker1", 60).unwrap() {
            LockResult::Acquired(g) => g,
            _ => panic!("Failed to acquire"),
        };
        
        guard.extend(30).unwrap();
        
        let info = manager.get_info("resource1").unwrap();
        assert_eq!(info.timeout_secs, 90);
    }
    
    #[test]
    fn test_with_lock() {
        let manager = MemoryLockManager::new();
        
        let result = DistributedLock::with_lock(&manager, "resource", "worker", 60, || {
            Ok(42)
        }).unwrap();
        
        assert_eq!(result, 42);
        assert!(!manager.is_locked("resource"));
    }
    
    #[test]
    fn test_try_with_retries() {
        let manager = MemoryLockManager::new();
        
        // Lock the resource first
        let _guard = manager.acquire("resource", "owner1", 60).unwrap();
        
        // Try to acquire with retries (should fail)
        let result = DistributedLock::try_with_retries(
            &manager,
            "resource",
            "owner2",
            60,
            3,
            10,
            || Ok(42),
        );
        
        assert!(result.is_err());
    }
    
    #[test]
    fn test_file_lock_manager() {
        let manager = FileLockManager::new("/tmp/noor_locks_test").unwrap();
        
        let result = manager.acquire("test_resource", "worker1", 60).unwrap();
        
        match result {
            LockResult::Acquired(guard) => {
                assert!(manager.is_locked("test_resource"));
                guard.release();
            }
            _ => panic!("Expected Acquired"),
        }
        
        assert!(!manager.is_locked("test_resource"));
    }
}
