// ============================================================
// التحقق المتقدم من الطلبات | Request Validation
// ============================================================

use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// قاعدة التحقق
#[derive(Debug, Clone)]
pub struct Rule {
    pub field: String,
    pub rules: Vec<ValidationRule>,
}

#[derive(Debug, Clone)]
pub struct ValidationRule {
    pub rule_type: RuleType,
    pub message: String,
    pub params: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuleType {
    Required,
    Optional,
    String,
    Integer,
    Float,
    Boolean,
    Email,
    Url,
    Min(usize),
    Max(usize),
    Between(usize, usize),
    In(Vec<String>),
    NotIn(Vec<String>),
    Same(String),
    Different(String),
    Regex(String),
    Date,
    DateAfter(String),
    DateBefore(String),
    Custom(String),
}

/// نتيجة التحقق
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: HashMap<String, Vec<String>>,
    pub validated: HashMap<String, serde_json::Value>,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self {
            valid: true,
            errors: HashMap::new(),
            validated: HashMap::new(),
        }
    }
    
    pub fn add_error(&mut self, field: &str, message: &str) {
        self.valid = false;
        self.errors
            .entry(field.to_string())
            .or_insert_with(Vec::new)
            .push(message.to_string());
    }
    
    pub fn has_errors(&self) -> bool {
        !self.valid
    }
    
    pub fn first_error(&self, field: &str) -> Option<String> {
        self.errors.get(field).and_then(|errors| errors.first().cloned())
    }
    
    pub fn all_errors(&self) -> Vec<String> {
        self.errors.values().flat_map(|v| v.iter().cloned()).collect()
    }
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

/// منشئ القواعد
pub struct RuleBuilder {
    rules: Vec<ValidationRule>,
}

impl RuleBuilder {
    pub fn new() -> Self {
        Self { rules: vec![] }
    }
    
    pub fn required(mut self) -> Self {
        self.rules.push(ValidationRule {
            rule_type: RuleType::Required,
            message: "This field is required".to_string(),
            params: vec![],
        });
        self
    }
    
    pub fn email(mut self) -> Self {
        self.rules.push(ValidationRule {
            rule_type: RuleType::Email,
            message: "Must be a valid email address".to_string(),
            params: vec![],
        });
        self
    }
    
    pub fn min(mut self, min: usize) -> Self {
        self.rules.push(ValidationRule {
            rule_type: RuleType::Min(min),
            message: format!("Must be at least {} characters", min),
            params: vec![min.to_string()],
        });
        self
    }
    
    pub fn max(mut self, max: usize) -> Self {
        self.rules.push(ValidationRule {
            rule_type: RuleType::Max(max),
            message: format!("Must not exceed {} characters", max),
            params: vec![max.to_string()],
        });
        self
    }
    
    pub fn in_values(mut self, values: Vec<String>) -> Self {
        self.rules.push(ValidationRule {
            rule_type: RuleType::In(values.clone()),
            message: format!("Must be one of: {}", values.join(", ")),
            params: values,
        });
        self
    }
    
    pub fn integer(mut self) -> Self {
        self.rules.push(ValidationRule {
            rule_type: RuleType::Integer,
            message: "Must be an integer".to_string(),
            params: vec![],
        });
        self
    }
    
    pub fn url(mut self) -> Self {
        self.rules.push(ValidationRule {
            rule_type: RuleType::Url,
            message: "Must be a valid URL".to_string(),
            params: vec![],
        });
        self
    }
    
    pub fn regex(mut self, pattern: &str) -> Self {
        self.rules.push(ValidationRule {
            rule_type: RuleType::Regex(pattern.to_string()),
            message: "Invalid format".to_string(),
            params: vec![pattern.to_string()],
        });
        self
    }
    
    pub fn message(mut self, msg: &str) -> Self {
        if let Some(last) = self.rules.last_mut() {
            last.message = msg.to_string();
        }
        self
    }
    
    pub fn build(self) -> Vec<ValidationRule> {
        self.rules
    }
}

impl Default for RuleBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// المدقق
pub struct Validator {
    rules: Vec<Rule>,
}

impl Validator {
    pub fn new() -> Self {
        Self { rules: vec![] }
    }
    
    pub fn rule(mut self, field: &str, rules: Vec<ValidationRule>) -> Self {
        self.rules.push(Rule {
            field: field.to_string(),
            rules,
        });
        self
    }
    
    /// التحقق من بيانات JSON
    pub fn validate(&self, data: &serde_json::Value) -> ValidationResult {
        let mut result = ValidationResult::new();
        
        for rule in &self.rules {
            let value = data.get(&rule.field);
            
            let mut has_required = false;
            for r in &rule.rules {
                if r.rule_type == RuleType::Required {
                    has_required = true;
                    break;
                }
            }
            
            // التحقق من الحقول المطلوبة
            if value.is_none() || value == Some(&serde_json::Value::Null) {
                if has_required {
                    result.add_error(&rule.field, "This field is required");
                }
                continue;
            }
            
            // التحقق من القيمة
            for r in &rule.rules {
                if let Some(err) = Self::validate_value(value.unwrap(), r) {
                    result.add_error(&rule.field, &err);
                }
            }
            
            // إضافة للقيم المتحقق منها
            result.validated.insert(rule.field.clone(), value.unwrap().clone());
        }
        
        result
    }
    
    fn validate_value(value: &serde_json::Value, rule: &ValidationRule) -> Option<String> {
        match &rule.rule_type {
            RuleType::Required => {
                if value.is_null() {
                    return Some(rule.message.clone());
                }
            }
            RuleType::Email => {
                if let Some(s) = value.as_str() {
                    if !crate::core::security::Validator::is_email(s) {
                        return Some(rule.message.clone());
                    }
                }
            }
            RuleType::Url => {
                if let Some(s) = value.as_str() {
                    if !crate::core::security::Validator::is_url(s) {
                        return Some(rule.message.clone());
                    }
                }
            }
            RuleType::Min(min) => {
                if let Some(s) = value.as_str() {
                    if s.len() < *min {
                        return Some(rule.message.clone());
                    }
                }
            }
            RuleType::Max(max) => {
                if let Some(s) = value.as_str() {
                    if s.len() > *max {
                        return Some(rule.message.clone());
                    }
                }
            }
            RuleType::Integer => {
                if value.as_i64().is_none() {
                    return Some(rule.message.clone());
                }
            }
            RuleType::In(values) => {
                let val_str = match value {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    _ => value.to_string(),
                };
                if !values.contains(&val_str) {
                    return Some(rule.message.clone());
                }
            }
            RuleType::Regex(pattern) => {
                if let Some(s) = value.as_str() {
                    if let Ok(re) = regex::Regex::new(pattern) {
                        if !re.is_match(s) {
                            return Some(rule.message.clone());
                        }
                    }
                }
            }
            _ => {}
        }
        
        None
    }
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_required_validation() {
        let validator = Validator::new()
            .rule("name", RuleBuilder::new().required().build())
            .rule("email", RuleBuilder::new().required().email().build());
        
        let data = serde_json::json!({"name": "John", "email": "invalid"});
        let result = validator.validate(&data);
        
        assert!(!result.valid);
        assert!(result.errors.contains_key("email"));
    }
    
    #[test]
    fn test_email_validation() {
        let validator = Validator::new()
            .rule("email", RuleBuilder::new().email().build());
        
        let data = serde_json::json!({"email": "valid@example.com"});
        let result = validator.validate(&data);
        assert!(result.valid);
        
        let data = serde_json::json!({"email": "invalid"});
        let result = validator.validate(&data);
        assert!(!result.valid);
    }
    
    #[test]
    fn test_min_max_validation() {
        let validator = Validator::new()
            .rule("password", RuleBuilder::new().min(8).max(20).build());
        
        let result = validator.validate(&serde_json::json!({"password": "short"}));
        assert!(!result.valid);
        
        let result = validator.validate(&serde_json::json!({"password": "validpassword123"}));
        assert!(result.valid);
    }
    
    #[test]
    fn test_in_validation() {
        let validator = Validator::new()
            .rule("status", RuleBuilder::new().in_values(vec![
                "draft".to_string(),
                "published".to_string(),
            ]).build());
        
        let result = validator.validate(&serde_json::json!({"status": "draft"}));
        assert!(result.valid);
        
        let result = validator.validate(&serde_json::json!({"status": "archived"}));
        assert!(!result.valid);
    }
}
