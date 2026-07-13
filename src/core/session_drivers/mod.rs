// ============================================================
// Session Drivers - مزودات الجلسات
// ============================================================
// Multiple session storage backends:
// - File (default, for weak servers)
// - Memory (for testing)
// - Database (for production)
// - Redis (for distributed)
// - Encrypted (for sensitive data)
//
// مزودات متعددة لتخزين الجلسات.
// ============================================================

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// Session data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub id: String,
    pub data: HashMap<String, serde_json::Value>,
    pub created_at: i64,
    pub last_accessed: i64,
    pub expires_at: i64,
    pub user_id: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

impl SessionData {
    pub fn new(id: &str, lifetime: i64) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: id.to_string(),
            data: HashMap::new(),
            created_at: now,
            last_accessed: now,
            expires_at: now + lifetime,
            user_id: None,
            ip_address: None,
            user_agent: None,
        }
    }
    
    /// Check if the session is expired
    pub fn is_expired(&self) -> bool {
        chrono::Utc::now().timestamp() > self.expires_at
    }
    
    /// Touch the session (update last accessed)
    pub fn touch(&mut self) {
        self.last_accessed = chrono::Utc::now().timestamp();
    }
    
    /// Set a value
    pub fn set<T: Serialize>(&mut self, key: &str, value: &T) -> crate::NoorResult<()> {
        self.data.insert(key.to_string(), serde_json::to_value(value)?);
        Ok(())
    }
    
    /// Get a value
    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        self.data.get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
    
    /// Remove a value
    pub fn remove(&mut self, key: &str) -> Option<serde_json::Value> {
        self.data.remove(key)
    }
    
    /// Check if a key exists
    pub fn has(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }
    
    /// Clear all data
    pub fn clear(&mut self) {
        self.data.clear();
    }
}

/// Session driver trait
pub trait SessionDriver: Send + Sync {
    /// Read a session by ID
    fn read(&self, id: &str) -> Option<SessionData>;
    
    /// Write a session
    fn write(&self, session: &SessionData) -> crate::NoorResult<()>;
    
    /// Delete a session
    fn destroy(&self, id: &str) -> bool;
    
    /// Clean up expired sessions
    fn gc(&self) -> usize;
    
    /// Get the driver name
    fn name(&self) -> &str;
}

/// File-based session driver (default, for weak servers)
pub struct FileSessionDriver {
    storage_dir: PathBuf,
    lifetime: i64,
}

impl FileSessionDriver {
    pub fn new(storage_dir: &str, lifetime: i64) -> crate::NoorResult<Self> {
        let path = PathBuf::from(storage_dir);
        std::fs::create_dir_all(&path)?;
        
        Ok(Self {
            storage_dir: path,
            lifetime,
        })
    }
    
    fn session_path(&self, id: &str) -> PathBuf {
        // Use first 2 chars as directory to avoid too many files
        let dir = &id[..2.min(id.len())];
        self.storage_dir.join(dir).join(format!("{}.json", id))
    }
}

impl SessionDriver for FileSessionDriver {
    fn read(&self, id: &str) -> Option<SessionData> {
        let path = self.session_path(id);
        
        if !path.exists() {
            return None;
        }
        
        let content = std::fs::read_to_string(&path).ok()?;
        let session: SessionData = serde_json::from_str(&content).ok()?;
        
        if session.is_expired() {
            std::fs::remove_file(&path).ok();
            return None;
        }
        
        Some(session)
    }
    
    fn write(&self, session: &SessionData) -> crate::NoorResult<()> {
        let path = self.session_path(&session.id);
        
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let content = serde_json::to_string_pretty(session)?;
        std::fs::write(&path, content)?;
        
        Ok(())
    }
    
    fn destroy(&self, id: &str) -> bool {
        let path = self.session_path(id);
        
        if path.exists() {
            std::fs::remove_file(&path).is_ok()
        } else {
            false
        }
    }
    
    fn gc(&self) -> usize {
        let now = chrono::Utc::now().timestamp();
        let mut cleaned = 0;
        
        if let Ok(entries) = std::fs::read_dir(&self.storage_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    if let Ok(sub_entries) = std::fs::read_dir(entry.path()) {
                        for sub_entry in sub_entries.flatten() {
                            if let Ok(content) = std::fs::read_to_string(sub_entry.path()) {
                                if let Ok(session) = serde_json::from_str::<SessionData>(&content) {
                                    if session.expires_at < now {
                                        std::fs::remove_file(sub_entry.path()).ok();
                                        cleaned += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        cleaned
    }
    
    fn name(&self) -> &str {
        "file"
    }
}

/// Memory session driver (for testing)
pub struct MemorySessionDriver {
    sessions: Arc<RwLock<HashMap<String, SessionData>>>,
    lifetime: i64,
}

impl MemorySessionDriver {
    pub fn new(lifetime: i64) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            lifetime,
        }
    }
}

impl SessionDriver for MemorySessionDriver {
    fn read(&self, id: &str) -> Option<SessionData> {
        let mut sessions = self.sessions.write();
        
        if let Some(session) = sessions.get_mut(id) {
            if session.is_expired() {
                sessions.remove(id);
                return None;
            }
            
            session.touch();
            return Some(session.clone());
        }
        
        None
    }
    
    fn write(&self, session: &SessionData) -> crate::NoorResult<()> {
        self.sessions.write().insert(session.id.clone(), session.clone());
        Ok(())
    }
    
    fn destroy(&self, id: &str) -> bool {
        self.sessions.write().remove(id).is_some()
    }
    
    fn gc(&self) -> usize {
        let mut sessions = self.sessions.write();
        let initial = sessions.len();
        sessions.retain(|_, s| !s.is_expired());
        initial - sessions.len()
    }
    
    fn name(&self) -> &str {
        "memory"
    }
}

/// Session manager
pub struct SessionManager {
    driver: Arc<dyn SessionDriver>,
    lifetime: i64,
}

impl SessionManager {
    pub fn new(driver: Arc<dyn SessionDriver>, lifetime: i64) -> Self {
        Self { driver, lifetime }
    }
    
    /// Create a file-based session manager (for weak servers)
    pub fn file(storage_dir: &str, lifetime: i64) -> crate::NoorResult<Self> {
        let driver = Arc::new(FileSessionDriver::new(storage_dir, lifetime)?);
        Ok(Self::new(driver, lifetime))
    }
    
    /// Create a memory-based session manager (for testing)
    pub fn memory(lifetime: i64) -> Self {
        let driver = Arc::new(MemorySessionDriver::new(lifetime));
        Self::new(driver, lifetime)
    }
    
    /// Start a new session
    pub fn start(&self) -> crate::NoorResult<SessionData> {
        let id = crate::core::security::Encryption::new().random_string(40)?;
        let session = SessionData::new(&id, self.lifetime);
        self.driver.write(&session)?;
        Ok(session)
    }
    
    /// Get a session by ID
    pub fn get(&self, id: &str) -> Option<SessionData> {
        self.driver.read(id)
    }
    
    /// Save a session
    pub fn save(&self, session: &SessionData) -> crate::NoorResult<()> {
        self.driver.write(session)
    }
    
    /// Destroy a session
    pub fn destroy(&self, id: &str) -> bool {
        self.driver.destroy(id)
    }
    
    /// Run garbage collection
    pub fn gc(&self) -> usize {
        self.driver.gc()
    }
    
    /// Get the driver name
    pub fn driver_name(&self) -> &str {
        self.driver.name()
    }
    
    /// Set a value in a session
    pub fn put(&self, session_id: &str, key: &str, value: serde_json::Value) -> crate::NoorResult<()> {
        if let Some(mut session) = self.get(session_id) {
            session.data.insert(key.to_string(), value);
            self.save(&session)?;
        }
        Ok(())
    }
    
    /// Get a value from a session
    pub fn get_value(&self, session_id: &str, key: &str) -> Option<serde_json::Value> {
        self.get(session_id).and_then(|s| s.data.get(key).cloned())
    }
    
    /// Remove a value from a session
    pub fn forget(&self, session_id: &str, key: &str) -> crate::NoorResult<()> {
        if let Some(mut session) = self.get(session_id) {
            session.data.remove(key);
            self.save(&session)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_memory_session_driver() {
        let manager = SessionManager::memory(3600);
        
        // Start a session
        let mut session = manager.start().unwrap();
        
        // Set data
        session.set("user_id", &123i64).unwrap();
        session.set("name", &"John".to_string()).unwrap();
        manager.save(&session).unwrap();
        
        // Get data
        let retrieved = manager.get(&session.id).unwrap();
        assert_eq!(retrieved.get::<i64>("user_id"), Some(123));
        assert_eq!(retrieved.get::<String>("name"), Some("John".to_string()));
        
        // Update data
        manager.put(&session.id, "name", serde_json::json!("Jane")).unwrap();
        
        let updated = manager.get(&session.id).unwrap();
        assert_eq!(updated.get::<String>("name"), Some("Jane".to_string()));
        
        // Remove data
        manager.forget(&session.id, "name").unwrap();
        
        let after_remove = manager.get(&session.id).unwrap();
        assert!(!after_remove.has("name"));
        
        // Destroy session
        assert!(manager.destroy(&session.id));
        assert!(manager.get(&session.id).is_none());
    }
    
    #[test]
    fn test_session_data() {
        let mut session = SessionData::new("test123", 3600);
        
        assert!(!session.is_expired());
        assert!(!session.has("key"));
        
        session.set("key", &"value".to_string()).unwrap();
        assert!(session.has("key"));
        
        let value: String = session.get("key").unwrap();
        assert_eq!(value, "value");
        
        session.remove("key");
        assert!(!session.has("key"));
    }
}
