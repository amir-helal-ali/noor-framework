// ============================================================
// Audit Trail - سجل التدقيق
// ============================================================
// Track all important actions for compliance and debugging.
// Records who did what, when, and from where.
//
// تتبع جميع الإجراءات المهمة للامتثال وتصحيح الأخطاء.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub user_id: Option<String>,
    pub user_name: Option<String>,
    pub action: String,
    pub module: String,
    pub description: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub old_values: Option<serde_json::Value>,
    pub new_values: Option<serde_json::Value>,
    pub metadata: HashMap<String, String>,
    pub severity: AuditSeverity,
    pub created_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditSeverity {
    Info,
    Warning,
    Critical,
}

impl AuditSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Critical => "critical",
        }
    }
}

/// Audit action types
pub mod actions {
    pub const LOGIN: &str = "auth.login";
    pub const LOGOUT: &str = "auth.logout";
    pub const LOGIN_FAILED: &str = "auth.login_failed";
    pub const PASSWORD_CHANGE: &str = "auth.password_change";
    pub const PASSWORD_RESET: &str = "auth.password_reset";
    pub const USER_CREATE: &str = "user.create";
    pub const USER_UPDATE: &str = "user.update";
    pub const USER_DELETE: &str = "user.delete";
    pub const ROLE_ASSIGN: &str = "role.assign";
    pub const ROLE_REVOKE: &str = "role.revoke";
    pub const POST_CREATE: &str = "post.create";
    pub const POST_UPDATE: &str = "post.update";
    pub const POST_DELETE: &str = "post.delete";
    pub const POST_PUBLISH: &str = "post.publish";
    pub const SETTINGS_UPDATE: &str = "settings.update";
    pub const EXPORT: &str = "data.export";
    pub const IMPORT: &str = "data.import";
    pub const BACKUP_CREATE: &str = "backup.create";
    pub const BACKUP_RESTORE: &str = "backup.restore";
}

/// Audit logger
pub struct AuditLogger {
    entries: Arc<RwLock<Vec<AuditEntry>>>,
    /// Whether to also log to tracing
    log_to_tracing: bool,
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditLogger {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
            log_to_tracing: true,
        }
    }
    
    /// Log an audit entry
    pub fn log(&self, entry: AuditEntry) {
        if self.log_to_tracing {
            match entry.severity {
                AuditSeverity::Info => {
                    tracing::info!(
                        user = ?entry.user_id,
                        action = %entry.action,
                        module = %entry.module,
                        "Audit: {} - {}",
                        entry.action, entry.description
                    );
                }
                AuditSeverity::Warning => {
                    tracing::warn!(
                        user = ?entry.user_id,
                        action = %entry.action,
                        "Audit: {} - {}",
                        entry.action, entry.description
                    );
                }
                AuditSeverity::Critical => {
                    tracing::error!(
                        user = ?entry.user_id,
                        action = %entry.action,
                        "CRITICAL Audit: {} - {}",
                        entry.action, entry.description
                    );
                }
            }
        }
        
        self.entries.write().push(entry);
    }
    
    /// Log a simple action
    pub fn log_action(
        &self,
        user_id: Option<&str>,
        action: &str,
        module: &str,
        description: &str,
    ) {
        self.log(AuditEntry {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: user_id.map(|s| s.to_string()),
            user_name: None,
            action: action.to_string(),
            module: module.to_string(),
            description: description.to_string(),
            ip_address: None,
            user_agent: None,
            old_values: None,
            new_values: None,
            metadata: HashMap::new(),
            severity: AuditSeverity::Info,
            created_at: chrono::Utc::now().timestamp(),
        });
    }
    
    /// Log a model change
    pub fn log_change(
        &self,
        user_id: &str,
        action: &str,
        module: &str,
        description: &str,
        old_values: serde_json::Value,
        new_values: serde_json::Value,
    ) {
        self.log(AuditEntry {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: Some(user_id.to_string()),
            user_name: None,
            action: action.to_string(),
            module: module.to_string(),
            description: description.to_string(),
            ip_address: None,
            user_agent: None,
            old_values: Some(old_values),
            new_values: Some(new_values),
            metadata: HashMap::new(),
            severity: AuditSeverity::Info,
            created_at: chrono::Utc::now().timestamp(),
        });
    }
    
    /// Log a security event
    pub fn log_security(
        &self,
        user_id: Option<&str>,
        action: &str,
        description: &str,
        ip_address: Option<&str>,
    ) {
        self.log(AuditEntry {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: user_id.map(|s| s.to_string()),
            user_name: None,
            action: action.to_string(),
            module: "security".to_string(),
            description: description.to_string(),
            ip_address: ip_address.map(|s| s.to_string()),
            user_agent: None,
            old_values: None,
            new_values: None,
            metadata: HashMap::new(),
            severity: AuditSeverity::Warning,
            created_at: chrono::Utc::now().timestamp(),
        });
    }
    
    /// Log a critical security event
    pub fn log_critical(
        &self,
        user_id: Option<&str>,
        action: &str,
        description: &str,
        ip_address: Option<&str>,
    ) {
        self.log(AuditEntry {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: user_id.map(|s| s.to_string()),
            user_name: None,
            action: action.to_string(),
            module: "security".to_string(),
            description: description.to_string(),
            ip_address: ip_address.map(|s| s.to_string()),
            user_agent: None,
            old_values: None,
            new_values: None,
            metadata: HashMap::new(),
            severity: AuditSeverity::Critical,
            created_at: chrono::Utc::now().timestamp(),
        });
    }
    
    /// Get all entries
    pub fn all(&self) -> Vec<AuditEntry> {
        self.entries.read().clone()
    }
    
    /// Get entries by user
    pub fn by_user(&self, user_id: &str) -> Vec<AuditEntry> {
        self.entries
            .read()
            .iter()
            .filter(|e| e.user_id.as_deref() == Some(user_id))
            .cloned()
            .collect()
    }
    
    /// Get entries by action
    pub fn by_action(&self, action: &str) -> Vec<AuditEntry> {
        self.entries
            .read()
            .iter()
            .filter(|e| e.action == action)
            .cloned()
            .collect()
    }
    
    /// Get entries by module
    pub fn by_module(&self, module: &str) -> Vec<AuditEntry> {
        self.entries
            .read()
            .iter()
            .filter(|e| e.module == module)
            .cloned()
            .collect()
    }
    
    /// Get entries by severity
    pub fn by_severity(&self, severity: AuditSeverity) -> Vec<AuditEntry> {
        self.entries
            .read()
            .iter()
            .filter(|e| e.severity == severity)
            .cloned()
            .collect()
    }
    
    /// Get recent entries
    pub fn recent(&self, limit: usize) -> Vec<AuditEntry> {
        let entries = self.entries.read();
        entries
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }
    
    /// Search entries
    pub fn search(&self, query: &str) -> Vec<AuditEntry> {
        let lower_query = query.to_lowercase();
        
        self.entries
            .read()
            .iter()
            .filter(|e| {
                e.action.to_lowercase().contains(&lower_query)
                    || e.description.to_lowercase().contains(&lower_query)
                    || e.module.to_lowercase().contains(&lower_query)
                    || e.user_name.as_ref().map(|n| n.to_lowercase().contains(&lower_query)).unwrap_or(false)
            })
            .cloned()
            .collect()
    }
    
    /// Clear all entries
    pub fn clear(&self) {
        self.entries.write().clear();
    }
    
    /// Get the count of entries
    pub fn count(&self) -> usize {
        self.entries.read().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_audit_log() {
        let logger = AuditLogger::new();
        
        logger.log_action(Some("user1"), actions::LOGIN, "auth", "User logged in");
        
        assert_eq!(logger.count(), 1);
        
        let entries = logger.by_user("user1");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].action, actions::LOGIN);
    }
    
    #[test]
    fn test_audit_change() {
        let logger = AuditLogger::new();
        
        logger.log_change(
            "user1",
            actions::USER_UPDATE,
            "users",
            "Updated user profile",
            serde_json::json!({"name": "Old Name"}),
            serde_json::json!({"name": "New Name"}),
        );
        
        let entries = logger.all();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].old_values.is_some());
        assert!(entries[0].new_values.is_some());
    }
    
    #[test]
    fn test_audit_security() {
        let logger = AuditLogger::new();
        
        logger.log_security(
            Some("user1"),
            actions::LOGIN_FAILED,
            "Failed login attempt",
            Some("192.168.1.1"),
        );
        
        let entries = logger.by_severity(AuditSeverity::Warning);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].ip_address, Some("192.168.1.1".to_string()));
    }
    
    #[test]
    fn test_audit_critical() {
        let logger = AuditLogger::new();
        
        logger.log_critical(
            None,
            "security.breach",
            "Multiple failed login attempts detected",
            Some("10.0.0.1"),
        );
        
        let critical = logger.by_severity(AuditSeverity::Critical);
        assert_eq!(critical.len(), 1);
    }
    
    #[test]
    fn test_audit_search() {
        let logger = AuditLogger::new();
        
        logger.log_action(Some("user1"), actions::POST_CREATE, "posts", "Created new post");
        logger.log_action(Some("user2"), actions::USER_DELETE, "users", "Deleted user");
        
        let results = logger.search("post");
        assert_eq!(results.len(), 1);
        
        let results = logger.search("user");
        assert_eq!(results.len(), 1);
    }
}
