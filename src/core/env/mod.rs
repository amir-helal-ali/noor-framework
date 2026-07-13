// ============================================================
// Environment Loader - محمل البيئة
// ============================================================
// Load environment variables from .env files.
// Similar to dotenv in Node.js and phpdotenv in PHP.
//
// تحميل متغيرات البيئة من ملفات .env.
// ============================================================

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;

/// Environment loader
pub struct EnvLoader {
    variables: Arc<RwLock<HashMap<String, String>>>,
}

impl Default for EnvLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl EnvLoader {
    pub fn new() -> Self {
        Self {
            variables: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Load environment variables from a .env file
    pub fn load(&self, path: &str) -> crate::NoorResult<()> {
        let path = Path::new(path);
        
        if !path.exists() {
            tracing::debug!(".env file not found at {}", path.display());
            return Ok(());
        }
        
        let content = std::fs::read_to_string(path)?;
        self.parse(&content)
    }
    
    /// Parse environment file content
    fn parse(&self, content: &str) -> crate::NoorResult<()> {
        let mut vars = self.variables.write();
        
        for (line_num, line) in content.lines().enumerate() {
            let line = line.trim();
            
            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            // Parse KEY=VALUE
            if let Some(pos) = line.find('=') {
                let key = line[..pos].trim().to_string();
                let mut value = line[pos + 1..].trim().to_string();
                
                // Remove quotes if present
                if (value.starts_with('"') && value.ends_with('"')) ||
                   (value.starts_with('\'') && value.ends_with('\'')) {
                    value = value[1..value.len() - 1].to_string();
                }
                
                // Expand variable references (${VAR})
                value = self.expand_variables(&value, &vars);
                
                // Set in environment
                std::env::set_var(&key, &value);
                vars.insert(key, value);
            } else {
                tracing::warn!("Invalid .env line {}: {}", line_num + 1, line);
            }
        }
        
        Ok(())
    }
    
    /// Expand variable references in a value
    fn expand_variables(&self, value: &str, vars: &HashMap<String, String>) -> String {
        let mut result = value.to_string();
        
        // Replace ${VAR} with environment variable
        while let Some(start) = result.find("${") {
            if let Some(end) = result[start..].find('}') {
                let var_name = &result[start + 2..start + end];
                let var_value = vars.get(var_name)
                    .cloned()
                    .or_else(|| std::env::var(var_name).ok())
                    .unwrap_or_default();
                
                let placeholder = &result[start..=start + end];
                result = result.replacen(placeholder, &var_value, 1);
            } else {
                break;
            }
        }
        
        result
    }
    
    /// Load from default .env file
    pub fn load_default(&self) -> crate::NoorResult<()> {
        self.load(".env")?;
        Ok(())
    }
    
    /// Get a variable value
    pub fn get(&self, key: &str) -> Option<String> {
        self.variables
            .read()
            .get(key)
            .cloned()
            .or_else(|| std::env::var(key).ok())
    }
    
    /// Get a variable or default value
    pub fn get_or(&self, key: &str, default: &str) -> String {
        self.get(key).unwrap_or_else(|| default.to_string())
    }
    
    /// Get a variable as integer
    pub fn get_int(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(|v| v.parse().ok())
    }
    
    /// Get a variable as boolean
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(|v| match v.to_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => Some(true),
            "false" | "0" | "no" | "off" | "" => Some(false),
            _ => None,
        })
    }
    
    /// Set a variable
    pub fn set(&self, key: &str, value: &str) {
        std::env::set_var(key, value);
        self.variables.write().insert(key.to_string(), value.to_string());
    }
    
    /// Check if a variable exists
    pub fn has(&self, key: &str) -> bool {
        self.get(key).is_some()
    }
    
    /// Get all variables
    pub fn all(&self) -> HashMap<String, String> {
        self.variables.read().clone()
    }
    
    /// Get the count of loaded variables
    pub fn count(&self) -> usize {
        self.variables.read().len()
    }
    
    /// Save variables to a .env file
    pub fn save(&self, path: &str) -> crate::NoorResult<()> {
        let vars = self.variables.read();
        
        let content: String = vars
            .iter()
            .map(|(k, v)| {
                if v.contains(' ') || v.contains('"') {
                    format!("{}=\"{}\"", k, v)
                } else {
                    format!("{}={}", k, v)
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        
        std::fs::write(path, content)?;
        Ok(())
    }
}

/// Global environment loader
static GLOBAL_ENV: once_cell::sync::Lazy<Arc<EnvLoader>> = 
    once_cell::sync::Lazy::new(|| Arc::new(EnvLoader::new()));

/// Get the global environment loader
pub fn env() -> Arc<EnvLoader> {
    GLOBAL_ENV.clone()
}

/// Get an environment variable
pub fn env_value(key: &str) -> Option<String> {
    GLOBAL_ENV.get(key)
}

/// Get an environment variable or default
pub fn env_or(key: &str, default: &str) -> String {
    GLOBAL_ENV.get_or(key, default)
}

/// Get an environment variable as integer
pub fn env_int(key: &str) -> Option<i64> {
    GLOBAL_ENV.get_int(key)
}

/// Get an environment variable as boolean
pub fn env_bool(key: &str) -> Option<bool> {
    GLOBAL_ENV.get_bool(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_env_parse() {
        let loader = EnvLoader::new();
        
        let content = r#"
# This is a comment
APP_NAME=Noor
APP_ENV=production
APP_DEBUG=false

DATABASE_URL=postgres://user:pass@localhost:5432/db
QUOTED_VALUE="hello world"
EMPTY_VALUE=
"#;
        
        loader.parse(content).unwrap();
        
        assert_eq!(loader.get("APP_NAME"), Some("Noor".to_string()));
        assert_eq!(loader.get("APP_ENV"), Some("production".to_string()));
        assert_eq!(loader.get_bool("APP_DEBUG"), Some(false));
        assert_eq!(loader.get("QUOTED_VALUE"), Some("hello world".to_string()));
    }
    
    #[test]
    fn test_env_variable_expansion() {
        let loader = EnvLoader::new();
        
        let content = r#"
BASE_URL=http://localhost:8080
API_URL=${BASE_URL}/api
"#;
        
        loader.parse(content).unwrap();
        
        assert_eq!(loader.get("API_URL"), Some("http://localhost:8080/api".to_string()));
    }
    
    #[test]
    fn test_env_helpers() {
        let loader = EnvLoader::new();
        loader.set("TEST_VAR", "test_value");
        
        assert_eq!(loader.get("TEST_VAR"), Some("test_value".to_string()));
        assert_eq!(loader.get_or("MISSING", "default"), "default");
        
        loader.set("TEST_INT", "42");
        assert_eq!(loader.get_int("TEST_INT"), Some(42));
        
        loader.set("TEST_BOOL", "true");
        assert_eq!(loader.get_bool("TEST_BOOL"), Some(true));
    }
}
