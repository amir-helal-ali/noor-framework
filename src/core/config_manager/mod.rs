// ============================================================
// Config Manager - مدير الإعدادات المتقدم
// ============================================================
// Hierarchical configuration: defaults < file < env < runtime
// Supports hot reload, caching, and environment-specific configs.
//
// إعدادات هرمية: افتراضي < ملف < بيئة < وقت تشغيل
// ============================================================

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};
use serde_json::Value;

/// Configuration source priority (higher number = higher priority)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConfigSource {
    Default = 0,
    File = 1,
    Environment = 2,
    Runtime = 3,
}

/// Configuration manager
pub struct ConfigManager {
    /// Layered configuration values
    values: Arc<RwLock<HashMap<String, ConfigValue>>>,
    /// Configuration directory
    config_dir: PathBuf,
    /// Current environment
    environment: String,
    /// Whether config is cached
    cached: Arc<RwLock<bool>>,
}

/// Configuration value with source tracking
#[derive(Debug, Clone)]
struct ConfigValue {
    value: Value,
    source: ConfigSource,
}

impl ConfigManager {
    pub fn new(config_dir: &str, environment: &str) -> Self {
        Self {
            values: Arc::new(RwLock::new(HashMap::new())),
            config_dir: PathBuf::from(config_dir),
            environment: environment.to_string(),
            cached: Arc::new(RwLock::new(false)),
        }
    }
    
    /// Load configuration from all sources
    pub fn load(&self) -> crate::NoorResult<()> {
        // 1. Load defaults
        self.load_file("default")?;
        
        // 2. Load environment-specific
        self.load_file(&self.environment)?;
        
        // 3. Load .env file
        self.load_env_file()?;
        
        // 4. Load environment variables
        self.load_env_vars()?;
        
        *self.cached.write() = true;
        
        Ok(())
    }
    
    /// Load a TOML configuration file
    fn load_file(&self, name: &str) -> crate::NoorResult<()> {
        let path = self.config_dir.join(format!("{}.toml", name));
        
        if !path.exists() {
            tracing::debug!("Config file not found: {}", path.display());
            return Ok(());
        }
        
        let content = std::fs::read_to_string(&path)?;
        let value: Value = toml::from_str::<toml::Value>(&content)
            .map_err(|e| crate::NoorError::Config(format!("TOML parse error: {}", e)))
            .ok()
            .map(|v| serde_json::to_value(v).unwrap_or(Value::Null))
            .unwrap_or(Value::Null);
        
        // Flatten and store
        self.store_value("", &value, ConfigSource::File);
        
        Ok(())
    }
    
    /// Load .env file
    fn load_env_file(&self) -> crate::NoorResult<()> {
        let env_path = PathBuf::from(".env");
        
        if !env_path.exists() {
            return Ok(());
        }
        
        let content = std::fs::read_to_string(&env_path)?;
        
        for line in content.lines() {
            let line = line.trim();
            
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            if let Some(pos) = line.find('=') {
                let key = line[..pos].trim().to_lowercase();
                let value = line[pos + 1..].trim().trim_matches('"').trim_matches('\'');
                
                self.set(&key, Value::String(value.to_string()), ConfigSource::Environment);
            }
        }
        
        Ok(())
    }
    
    /// Load environment variables
    fn load_env_vars(&self) -> crate::NoorResult<()> {
        for (key, value) in std::env::vars() {
            let lower_key = key.to_lowercase();
            self.set(&lower_key, Value::String(value), ConfigSource::Environment);
        }
        
        Ok(())
    }
    
    /// Store a value with source tracking
    fn store_value(&self, prefix: &str, value: &Value, source: ConfigSource) {
        match value {
            Value::Object(map) => {
                for (k, v) in map {
                    let key = if prefix.is_empty() {
                        k.clone()
                    } else {
                        format!("{}.{}", prefix, k)
                    };
                    self.store_value(&key, v, source);
                }
            }
            _ => {
                self.set(prefix, value.clone(), source);
            }
        }
    }
    
    /// Set a configuration value
    pub fn set(&self, key: &str, value: Value, source: ConfigSource) {
        let mut values = self.values.write();
        
        // Only update if new source has higher or equal priority
        let should_update = match values.get(key) {
            Some(existing) => source >= existing.source,
            None => true,
        };
        
        if should_update {
            values.insert(key.to_string(), ConfigValue { value, source });
        }
    }
    
    /// Set a runtime value (highest priority)
    pub fn set_runtime(&self, key: &str, value: Value) {
        self.set(key, value, ConfigSource::Runtime);
    }
    
    /// Get a configuration value
    pub fn get(&self, key: &str) -> Option<Value> {
        self.values.read().get(key).map(|cv| cv.value.clone())
    }
    
    /// Get a string value
    pub fn get_string(&self, key: &str) -> Option<String> {
        self.get(key).and_then(|v| v.as_str().map(|s| s.to_string()))
    }
    
    /// Get an integer value
    pub fn get_int(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(|v| v.as_i64())
    }
    
    /// Get a float value
    pub fn get_float(&self, key: &str) -> Option<f64> {
        self.get(key).and_then(|v| v.as_f64())
    }
    
    /// Get a boolean value
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(|v| v.as_bool())
    }
    
    /// Get a value with default
    pub fn get_or(&self, key: &str, default: &str) -> String {
        self.get_string(key).unwrap_or_else(|| default.to_string())
    }
    
    /// Check if a key exists
    pub fn has(&self, key: &str) -> bool {
        self.values.read().contains_key(key)
    }
    
    /// Get all keys
    pub fn keys(&self) -> Vec<String> {
        self.values.read().keys().cloned().collect()
    }
    
    /// Get all configuration
    pub fn all(&self) -> HashMap<String, Value> {
        self.values
            .read()
            .iter()
            .map(|(k, cv)| (k.clone(), cv.value.clone()))
            .collect()
    }
    
    /// Get the current environment
    pub fn environment(&self) -> &str {
        &self.environment
    }
    
    /// Check if running in production
    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }
    
    /// Check if running in development
    pub fn is_development(&self) -> bool {
        self.environment == "development"
    }
    
    /// Check if running in testing
    pub fn is_testing(&self) -> bool {
        self.environment == "testing"
    }
    
    /// Export configuration to JSON
    pub fn to_json(&self) -> Value {
        Value::Object(
            self.values
                .read()
                .iter()
                .map(|(k, cv)| (k.clone(), cv.value.clone()))
                .collect()
        )
    }
    
    /// Clear all configuration
    pub fn clear(&self) {
        self.values.write().clear();
        *self.cached.write() = false;
    }
    
    /// Get the source of a configuration value
    pub fn source(&self, key: &str) -> Option<ConfigSource> {
        self.values.read().get(key).map(|cv| cv.source)
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new("config", "development")
    }
}

/// Global configuration instance
static GLOBAL_CONFIG: once_cell::sync::Lazy<Arc<ConfigManager>> = 
    once_cell::sync::Lazy::new(|| Arc::new(ConfigManager::default()));

/// Get the global configuration manager
pub fn config() -> Arc<ConfigManager> {
    GLOBAL_CONFIG.clone()
}

/// Get a configuration value
pub fn cfg(key: &str) -> Option<Value> {
    GLOBAL_CONFIG.get(key)
}

/// Get a string configuration value
pub fn cfg_string(key: &str) -> Option<String> {
    GLOBAL_CONFIG.get_string(key)
}

/// Get a configuration value or default
pub fn cfg_or(key: &str, default: &str) -> String {
    GLOBAL_CONFIG.get_or(key, default)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config_basic() {
        let config = ConfigManager::new("config", "testing");
        
        config.set("app.name", Value::String("Test App".to_string()), ConfigSource::Runtime);
        config.set("app.port", Value::Number(8080.into()), ConfigSource::Runtime);
        config.set("app.debug", Value::Bool(true), ConfigSource::Runtime);
        
        assert_eq!(config.get_string("app.name"), Some("Test App".to_string()));
        assert_eq!(config.get_int("app.port"), Some(8080));
        assert_eq!(config.get_bool("app.debug"), Some(true));
    }
    
    #[test]
    fn test_config_priority() {
        let config = ConfigManager::new("config", "testing");
        
        // Set from file (low priority)
        config.set("key", Value::String("file_value".to_string()), ConfigSource::File);
        assert_eq!(config.get_string("key"), Some("file_value".to_string()));
        
        // Set from env (higher priority - should override)
        config.set("key", Value::String("env_value".to_string()), ConfigSource::Environment);
        assert_eq!(config.get_string("key"), Some("env_value".to_string()));
        
        // Set from runtime (highest priority)
        config.set("key", Value::String("runtime_value".to_string()), ConfigSource::Runtime);
        assert_eq!(config.get_string("key"), Some("runtime_value".to_string()));
        
        // Try to set from file again (should NOT override)
        config.set("key", Value::String("file_again".to_string()), ConfigSource::File);
        assert_eq!(config.get_string("key"), Some("runtime_value".to_string()));
    }
    
    #[test]
    fn test_config_defaults() {
        let config = ConfigManager::new("config", "testing");
        
        assert_eq!(config.get_or("missing.key", "default_value"), "default_value");
    }
    
    #[test]
    fn test_config_environment() {
        let config = ConfigManager::new("config", "production");
        
        assert!(config.is_production());
        assert!(!config.is_development());
        
        let dev_config = ConfigManager::new("config", "development");
        assert!(dev_config.is_development());
        assert!(!dev_config.is_production());
    }
}
