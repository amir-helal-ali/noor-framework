// ============================================================
// Model Accessors & Mutators - المُلحقات والمُغيّرات
// ============================================================
// Transform attribute values when getting or setting them.
// e.g., encrypt password on set, format date on get.
//
// تحويل قيم السمات عند القراءة أو الكتابة.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// Accessor function type (model -> transformed value)
type AccessorFn = Arc<dyn Fn(&serde_json::Value) -> serde_json::Value + Send + Sync>;

/// Mutator function type (input -> stored value)
type MutatorFn = Arc<dyn Fn(&serde_json::Value) -> serde_json::Value + Send + Sync>;

/// Accessor/Mutator manager
pub struct AttributeManager {
    accessors: Arc<RwLock<HashMap<String, AccessorFn>>>,
    mutators: Arc<RwLock<HashMap<String, MutatorFn>>>,
    /// Hidden attributes (not serialized)
    hidden: Arc<RwLock<Vec<String>>>,
    /// Appended attributes (computed)
    appended: Arc<RwLock<Vec<String>>>,
}

impl Default for AttributeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AttributeManager {
    pub fn new() -> Self {
        Self {
            accessors: Arc::new(RwLock::new(HashMap::new())),
            mutators: Arc::new(RwLock::new(HashMap::new())),
            hidden: Arc::new(RwLock::new(Vec::new())),
            appended: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Register an accessor for an attribute
    pub fn accessor<F>(&self, name: &str, accessor: F) -> &Self
    where
        F: Fn(&serde_json::Value) -> serde_json::Value + Send + Sync + 'static,
    {
        self.accessors.write().insert(name.to_string(), Arc::new(accessor));
        self
    }
    
    /// Register a mutator for an attribute
    pub fn mutator<F>(&self, name: &str, mutator: F) -> &Self
    where
        F: Fn(&serde_json::Value) -> serde_json::Value + Send + Sync + 'static,
    {
        self.mutators.write().insert(name.to_string(), Arc::new(mutator));
        self
    }
    
    /// Hide an attribute from serialization
    pub fn hidden(&self, name: &str) -> &Self {
        self.hidden.write().push(name.to_string());
        self
    }
    
    /// Append a computed attribute
    pub fn appended(&self, name: &str) -> &Self {
        self.appended.write().push(name.to_string());
        self
    }
    
    /// Get an attribute value (with accessor applied)
    pub fn get_attribute(&self, name: &str, value: &serde_json::Value) -> serde_json::Value {
        let accessors = self.accessors.read();
        
        if let Some(accessor) = accessors.get(name) {
            accessor(value)
        } else {
            value.clone()
        }
    }
    
    /// Set an attribute value (with mutator applied)
    pub fn set_attribute(&self, name: &str, value: &serde_json::Value) -> serde_json::Value {
        let mutators = self.mutators.read();
        
        if let Some(mutator) = mutators.get(name) {
            mutator(value)
        } else {
            value.clone()
        }
    }
    
    /// Transform all attributes for output
    pub fn transform_for_output(&self, attributes: &HashMap<String, serde_json::Value>) -> serde_json::Value {
        let hidden = self.hidden.read();
        let accessors = self.accessors.read();
        
        let mut result = serde_json::Map::new();
        
        for (key, value) in attributes {
            // Skip hidden attributes
            if hidden.contains(key) {
                continue;
            }
            
            // Apply accessor
            let transformed = if let Some(accessor) = accessors.get(key) {
                accessor(value)
            } else {
                value.clone()
            };
            
            result.insert(key.clone(), transformed);
        }
        
        serde_json::Value::Object(result)
    }
    
    /// Transform all attributes for storage
    pub fn transform_for_storage(&self, attributes: &HashMap<String, serde_json::Value>) -> HashMap<String, serde_json::Value> {
        let mutators = self.mutators.read();
        
        let mut result = HashMap::new();
        
        for (key, value) in attributes {
            let transformed = if let Some(mutator) = mutators.get(key) {
                mutator(value)
            } else {
                value.clone()
            };
            
            result.insert(key.clone(), transformed);
        }
        
        result
    }
    
    /// Check if an attribute is hidden
    pub fn is_hidden(&self, name: &str) -> bool {
        self.hidden.read().contains(&name.to_string())
    }
    
    /// List all hidden attributes
    pub fn hidden_attributes(&self) -> Vec<String> {
        self.hidden.read().clone()
    }
    
    /// List all accessor attributes
    pub fn accessor_attributes(&self) -> Vec<String> {
        self.accessors.read().keys().cloned().collect()
    }
    
    /// List all mutator attributes
    pub fn mutator_attributes(&self) -> Vec<String> {
        self.mutators.read().keys().cloned().collect()
    }
}

/// Common accessor/mutator patterns
pub mod common {
    use super::*;
    
    /// Hash password mutator
    pub fn password_mutator() -> impl Fn(&serde_json::Value) -> serde_json::Value + Send + Sync + 'static {
        move |value| {
            if let Some(password) = value.as_str() {
                if password.is_empty() {
                    return serde_json::Value::Null;
                }
                
                // Don't re-hash if already hashed
                if password.starts_with("$argon2") {
                    return value.clone();
                }
                
                match crate::core::security::Encryption::hash_password(password) {
                    Ok(hash) => serde_json::Value::String(hash),
                    Err(_) => value.clone(),
                }
            } else {
                value.clone()
            }
        }
    }
    
    /// Format date accessor
    pub fn date_accessor(format: &'static str) -> impl Fn(&serde_json::Value) -> serde_json::Value + Send + Sync + 'static {
        move |value| {
            if let Some(timestamp) = value.as_i64() {
                let dt = chrono::DateTime::from_timestamp(timestamp, 0);
                if let Some(datetime) = dt {
                    return serde_json::Value::String(datetime.format(format).to_string());
                }
            }
            
            if let Some(s) = value.as_str() {
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
                    return serde_json::Value::String(dt.format(format).to_string());
                }
            }
            
            value.clone()
        }
    }
    
    /// Capitalize name accessor
    pub fn capitalize_name() -> impl Fn(&serde_json::Value) -> serde_json::Value + Send + Sync + 'static {
        move |value| {
            if let Some(s) = value.as_str() {
                let capitalized: String = s
                    .split_whitespace()
                    .map(|word| {
                        let mut chars = word.chars();
                        match chars.next() {
                            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                            None => String::new(),
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                
                return serde_json::Value::String(capitalized);
            }
            
            value.clone()
        }
    }
    
    /// Trim whitespace mutator
    pub fn trim_mutator() -> impl Fn(&serde_json::Value) -> serde_json::Value + Send + Sync + 'static {
        move |value| {
            if let Some(s) = value.as_str() {
                return serde_json::Value::String(s.trim().to_string());
            }
            value.clone()
        }
    }
    
    /// Lowercase email mutator
    pub fn lowercase_email_mutator() -> impl Fn(&serde_json::Value) -> serde_json::Value + Send + Sync + 'static {
        move |value| {
            if let Some(s) = value.as_str() {
                return serde_json::Value::String(s.to_lowercase());
            }
            value.clone()
        }
    }
    
    /// Boolean accessor (converts 0/1 to false/true)
    pub fn boolean_accessor() -> impl Fn(&serde_json::Value) -> serde_json::Value + Send + Sync + 'static {
        move |value| {
            match value {
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        return serde_json::Value::Bool(i != 0);
                    }
                }
                serde_json::Value::String(s) => {
                    return serde_json::Value::Bool(s == "true" || s == "1");
                }
                serde_json::Value::Bool(b) => {
                    return serde_json::Value::Bool(*b);
                }
                _ => {}
            }
            serde_json::Value::Bool(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_accessor() {
        let manager = AttributeManager::new();
        
        manager.accessor("name", |value| {
            if let Some(s) = value.as_str() {
                serde_json::Value::String(s.to_uppercase())
            } else {
                value.clone()
            }
        });
        
        let result = manager.get_attribute("name", &serde_json::json!("john"));
        assert_eq!(result, serde_json::json!("JOHN"));
    }
    
    #[test]
    fn test_mutator() {
        let manager = AttributeManager::new();
        
        manager.mutator("email", |value| {
            if let Some(s) = value.as_str() {
                serde_json::Value::String(s.to_lowercase())
            } else {
                value.clone()
            }
        });
        
        let result = manager.set_attribute("email", &serde_json::json!("JOHN@EXAMPLE.COM"));
        assert_eq!(result, serde_json::json!("john@example.com"));
    }
    
    #[test]
    fn test_hidden_attributes() {
        let manager = AttributeManager::new();
        
        manager.hidden("password");
        
        let mut attributes = HashMap::new();
        attributes.insert("name".to_string(), serde_json::json!("John"));
        attributes.insert("password".to_string(), serde_json::json!("secret"));
        
        let result = manager.transform_for_output(&attributes);
        
        assert!(result.get("name").is_some());
        assert!(result.get("password").is_none());
    }
    
    #[test]
    fn test_capitalize_name() {
        let accessor = common::capitalize_name();
        
        let result = accessor(&serde_json::json!("john doe"));
        assert_eq!(result, serde_json::json!("John Doe"));
    }
    
    #[test]
    fn test_lowercase_email_mutator() {
        let mutator = common::lowercase_email_mutator();
        
        let result = mutator(&serde_json::json!("JOHN@EXAMPLE.COM"));
        assert_eq!(result, serde_json::json!("john@example.com"));
    }
    
    #[test]
    fn test_boolean_accessor() {
        let accessor = common::boolean_accessor();
        
        assert_eq!(accessor(&serde_json::json!(1)), serde_json::json!(true));
        assert_eq!(accessor(&serde_json::json!(0)), serde_json::json!(false));
        assert_eq!(accessor(&serde_json::json!("true")), serde_json::json!(true));
        assert_eq!(accessor(&serde_json::json!("1")), serde_json::json!(true));
    }
}
