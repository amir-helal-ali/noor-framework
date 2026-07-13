// ============================================================
// Session Management - إدارة الجلسات
// ============================================================
// File-based session storage for weak servers (no Redis needed).
// Uses signed cookies to prevent tampering.
//
// تخزين الجلسات في ملفات للسيرفرات الضعيفة.
// ============================================================

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use crate::core::security::Encryption;

/// Session data
/// بيانات الجلسة
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub id: String,
    pub data: HashMap<String, serde_json::Value>,
    pub created_at: i64,
    pub last_accessed: i64,
    pub expires_at: i64,
}

/// Session manager with file-based storage
/// مدير الجلسات بتخزين ملفي
pub struct SessionManager {
    storage_dir: PathBuf,
    lifetime: i64,
    encryption: Arc<Encryption>,
}

impl SessionManager {
    pub fn new(storage_dir: &str, lifetime_secs: i64) -> crate::NoorResult<Self> {
        let path = PathBuf::from(storage_dir);
        std::fs::create_dir_all(&path)?;
        
        Ok(Self {
            storage_dir: path,
            lifetime: lifetime_secs,
            encryption: Arc::new(Encryption::new()),
        })
    }
    
    /// Create a new session
    /// إنشاء جلسة جديدة
    pub fn create(&self) -> crate::NoorResult<SessionData> {
        let now = chrono::Utc::now().timestamp();
        let session = SessionData {
            id: self.encryption.random_string(32)?,
            data: HashMap::new(),
            created_at: now,
            last_accessed: now,
            expires_at: now + self.lifetime,
        };
        
        self.save(&session)?;
        Ok(session)
    }
    
    /// Get a session by ID
    /// الحصول على جلسة بالـ ID
    pub fn get(&self, id: &str) -> crate::NoorResult<Option<SessionData>> {
        let path = self.session_path(id);
        if !path.exists() {
            return Ok(None);
        }
        
        let content = std::fs::read_to_string(&path)?;
        let session: SessionData = serde_json::from_str(&content)?;
        
        // Check if expired
        let now = chrono::Utc::now().timestamp();
        if session.expires_at < now {
            // Delete expired session
            std::fs::remove_file(&path).ok();
            return Ok(None);
        }
        
        Ok(Some(session))
    }
    
    /// Save a session
    /// حفظ الجلسة
    pub fn save(&self, session: &SessionData) -> crate::NoorResult<()> {
        let path = self.session_path(&session.id);
        let content = serde_json::to_string_pretty(session)?;
        std::fs::write(&path, content)?;
        Ok(())
    }
    
    /// Destroy a session
    /// تدمير الجلسة
    pub fn destroy(&self, id: &str) -> crate::NoorResult<()> {
        let path = self.session_path(id);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }
    
    /// Set a value in the session
    /// تعيين قيمة في الجلسة
    pub fn put(&self, id: &str, key: &str, value: serde_json::Value) -> crate::NoorResult<()> {
        if let Some(mut session) = self.get(id)? {
            session.data.insert(key.to_string(), value);
            session.last_accessed = chrono::Utc::now().timestamp();
            self.save(&session)?;
        }
        Ok(())
    }
    
    /// Get a value from the session
    /// الحصول على قيمة من الجلسة
    pub fn get_value(&self, id: &str, key: &str) -> crate::NoorResult<Option<serde_json::Value>> {
        if let Some(session) = self.get(id)? {
            return Ok(session.data.get(key).cloned());
        }
        Ok(None)
    }
    
    /// Remove a value from the session
    /// إزالة قيمة من الجلسة
    pub fn forget(&self, id: &str, key: &str) -> crate::NoorResult<()> {
        if let Some(mut session) = self.get(id)? {
            session.data.remove(key);
            self.save(&session)?;
        }
        Ok(())
    }
    
    /// Clean up expired sessions (important for weak servers)
    /// تنظيف الجلسات المنتهية
    pub fn gc(&self) -> crate::NoorResult<usize> {
        let now = chrono::Utc::now().timestamp();
        let mut cleaned = 0;
        
        if let Ok(entries) = std::fs::read_dir(&self.storage_dir) {
            for entry in entries.flatten() {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    if let Ok(session) = serde_json::from_str::<SessionData>(&content) {
                        if session.expires_at < now {
                            std::fs::remove_file(entry.path()).ok();
                            cleaned += 1;
                        }
                    }
                }
            }
        }
        
        Ok(cleaned)
    }
    
    fn session_path(&self, id: &str) -> PathBuf {
        self.storage_dir.join(format!("{}.json", id))
    }
}

/// Session facade for easy access
/// واجهة Session للوصول السهل
pub struct Session {
    pub manager: Arc<SessionManager>,
    pub id: Option<String>,
}

impl Session {
    pub fn new(manager: Arc<SessionManager>) -> Self {
        Self { manager, id: None }
    }
    
    pub fn start(&mut self) -> crate::NoorResult<String> {
        let session = self.manager.create()?;
        self.id = Some(session.id.clone());
        Ok(session.id)
    }
    
    pub fn set(&self, key: &str, value: impl Serialize) -> crate::NoorResult<()> {
        if let Some(id) = &self.id {
            let value = serde_json::to_value(value)?;
            self.manager.put(id, key, value)?;
        }
        Ok(())
    }
    
    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> crate::NoorResult<Option<T>> {
        if let Some(id) = &self.id {
            if let Some(value) = self.manager.get_value(id, key)? {
                return Ok(Some(serde_json::from_value(value)?));
            }
        }
        Ok(None)
    }
    
    pub fn destroy(&self) -> crate::NoorResult<()> {
        if let Some(id) = &self.id {
            self.manager.destroy(id)?;
        }
        Ok(())
    }
}
