// ============================================================
// Feature Flags - أعلام الميزات
// ============================================================
// Toggle features on/off at runtime without redeployment.
// Supports per-user, per-percentage, and per-environment flags.
//
// تفعيل/تعطيل الميزات وقت التشغيل بدون إعادة النشر.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// Feature flag definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlag {
    pub key: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub strategy: FlagStrategy,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Strategy for evaluating a flag
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FlagStrategy {
    /// Simple on/off
    Boolean,
    /// Enabled for a percentage of users
    Percentage(u32),
    /// Enabled for specific users
    UserList(Vec<String>),
    /// Enabled for specific environments
    Environment(Vec<String>),
    /// Enabled based on custom attributes
    Custom(serde_json::Value),
}

impl Default for FlagStrategy {
    fn default() -> Self {
        Self::Boolean
    }
}

/// Feature flag manager
pub struct FeatureFlagManager {
    flags: Arc<RwLock<HashMap<String, FeatureFlag>>>,
}

impl Default for FeatureFlagManager {
    fn default() -> Self {
        Self::new()
    }
}

impl FeatureFlagManager {
    pub fn new() -> Self {
        Self {
            flags: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register a feature flag
    pub fn register(&self, flag: FeatureFlag) {
        self.flags.write().insert(flag.key.clone(), flag);
    }
    
    /// Create a simple boolean flag
    pub fn boolean(&self, key: &str, name: &str, enabled: bool) {
        let now = chrono::Utc::now().timestamp();
        self.register(FeatureFlag {
            key: key.to_string(),
            name: name.to_string(),
            description: String::new(),
            enabled,
            strategy: FlagStrategy::Boolean,
            created_at: now,
            updated_at: now,
        });
    }
    
    /// Create a percentage-based flag
    pub fn percentage(&self, key: &str, name: &str, percentage: u32, enabled: bool) {
        let now = chrono::Utc::now().timestamp();
        self.register(FeatureFlag {
            key: key.to_string(),
            name: name.to_string(),
            description: String::new(),
            enabled,
            strategy: FlagStrategy::Percentage(percentage),
            created_at: now,
            updated_at: now,
        });
    }
    
    /// Create a user-list flag
    pub fn for_users(&self, key: &str, name: &str, users: Vec<String>) {
        let now = chrono::Utc::now().timestamp();
        self.register(FeatureFlag {
            key: key.to_string(),
            name: name.to_string(),
            description: String::new(),
            enabled: true,
            strategy: FlagStrategy::UserList(users),
            created_at: now,
            updated_at: now,
        });
    }
    
    /// Check if a feature is enabled
    pub fn is_enabled(&self, key: &str) -> bool {
        let flags = self.flags.read();
        
        match flags.get(key) {
            Some(flag) if flag.enabled => {
                match &flag.strategy {
                    FlagStrategy::Boolean => true,
                    FlagStrategy::Percentage(pct) => {
                        // Use a hash of the key for deterministic randomness
                        let hash = crate::core::security::Encryption::sha256_hex(key.as_bytes());
                        let num = u32::from_str_radix(&hash[..8], 16).unwrap_or(0);
                        (num % 100) < *pct
                    }
                    FlagStrategy::Environment(envs) => {
                        let current_env = std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
                        envs.contains(&current_env)
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }
    
    /// Check if a feature is enabled for a specific user
    pub fn is_enabled_for(&self, key: &str, user_id: &str) -> bool {
        let flags = self.flags.read();
        
        match flags.get(key) {
            Some(flag) if flag.enabled => {
                match &flag.strategy {
                    FlagStrategy::Boolean => true,
                    FlagStrategy::Percentage(pct) => {
                        // Use a hash of user_id for deterministic assignment
                        let hash = crate::core::security::Encryption::sha256_hex(user_id.as_bytes());
                        let num = u32::from_str_radix(&hash[..8], 16).unwrap_or(0);
                        (num % 100) < *pct
                    }
                    FlagStrategy::UserList(users) => users.contains(&user_id.to_string()),
                    _ => self.is_enabled(key),
                }
            }
            _ => false,
        }
    }
    
    /// Enable a flag
    pub fn enable(&self, key: &str) {
        if let Some(flag) = self.flags.write().get_mut(key) {
            flag.enabled = true;
            flag.updated_at = chrono::Utc::now().timestamp();
        }
    }
    
    /// Disable a flag
    pub fn disable(&self, key: &str) {
        if let Some(flag) = self.flags.write().get_mut(key) {
            flag.enabled = false;
            flag.updated_at = chrono::Utc::now().timestamp();
        }
    }
    
    /// Toggle a flag
    pub fn toggle(&self, key: &str) {
        if let Some(flag) = self.flags.write().get_mut(key) {
            flag.enabled = !flag.enabled;
            flag.updated_at = chrono::Utc::now().timestamp();
        }
    }
    
    /// Get all flags
    pub fn list(&self) -> Vec<FeatureFlag> {
        self.flags.read().values().cloned().collect()
    }
    
    /// Get a specific flag
    pub fn get(&self, key: &str) -> Option<FeatureFlag> {
        self.flags.read().get(key).cloned()
    }
    
    /// Remove a flag
    pub fn remove(&self, key: &str) -> bool {
        self.flags.write().remove(key).is_some()
    }
    
    /// Get the number of registered flags
    pub fn count(&self) -> usize {
        self.flags.read().len()
    }
}

/// Macro for checking feature flags
#[macro_export]
macro_rules! feature {
    ($manager:expr, $key:expr) => {
        $manager.is_enabled($key)
    };
    ($manager:expr, $key:expr, $user:expr) => {
        $manager.is_enabled_for($key, $user)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_boolean_flag() {
        let manager = FeatureFlagManager::new();
        
        manager.boolean("new_ui", "New UI", true);
        
        assert!(manager.is_enabled("new_ui"));
        
        manager.disable("new_ui");
        assert!(!manager.is_enabled("new_ui"));
    }
    
    #[test]
    fn test_user_list_flag() {
        let manager = FeatureFlagManager::new();
        
        manager.for_users("beta_feature", "Beta Feature", vec![
            "user1".to_string(),
            "user2".to_string(),
        ]);
        
        assert!(manager.is_enabled_for("beta_feature", "user1"));
        assert!(!manager.is_enabled_for("beta_feature", "user3"));
    }
    
    #[test]
    fn test_percentage_flag() {
        let manager = FeatureFlagManager::new();
        
        manager.percentage("test_feature", "Test", 100, true);
        
        // With 100%, should always be enabled
        assert!(manager.is_enabled("test_feature"));
        
        manager.percentage("test_feature", "Test", 0, true);
        // With 0%, should be disabled
        assert!(!manager.is_enabled("test_feature"));
    }
    
    #[test]
    fn test_toggle() {
        let manager = FeatureFlagManager::new();
        
        manager.boolean("flag", "Flag", false);
        assert!(!manager.is_enabled("flag"));
        
        manager.toggle("flag");
        assert!(manager.is_enabled("flag"));
        
        manager.toggle("flag");
        assert!(!manager.is_enabled("flag"));
    }
}
