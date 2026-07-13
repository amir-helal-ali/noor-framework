// ============================================================
// Input Validator - مدقق المدخلات
// ============================================================
// Fluent validation API for user input.
// واجهة تحقق سلسة لمدخلات المستخدم.
// ============================================================

use std::collections::HashMap;
use regex::Regex;

/// Input validator
/// مدقق المدخلات
pub struct Validator {
    errors: Vec<ValidationError>,
}

/// A validation error
/// خطأ تحقق
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub code: String,
}

impl Validator {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }
    
    /// Validate a single value
    /// تحقق من قيمة واحدة
    pub fn validate<T: ValidationRule>(value: &T, _rule: T::Rule) -> Result<(), ValidationError> {
        // The rule itself is responsible for validation - this is a placeholder
        // that allows users to plug in their own validation logic via the trait.
        let _ = value;
        Ok(())
    }
    
    /// Check if a string is a valid email
    /// فحص إذا كان النص بريداً إلكترونياً صحيحاً
    pub fn is_email(email: &str) -> bool {
        let email_re = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
        email_re.is_match(email)
    }
    
    /// Check if a string is a valid URL
    /// فحص إذا كان النص URL صحيحاً
    pub fn is_url(url: &str) -> bool {
        let url_re = Regex::new(r"^https?://[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}(/.*)?$").unwrap();
        url_re.is_match(url)
    }
    
    /// Check if a string is a valid UUID
    /// فحص إذا كان النص UUID صحيحاً
    pub fn is_uuid(uuid: &str) -> bool {
        let uuid_re = Regex::new(r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$").unwrap();
        uuid_re.is_match(uuid)
    }
    
    /// Check if a string is a valid phone number (basic)
    /// فحص إذا كان النص رقم هاتف صحيحاً
    pub fn is_phone(phone: &str) -> bool {
        let phone_re = Regex::new(r"^\+?[0-9]{10,15}$").unwrap();
        phone_re.is_match(phone)
    }
    
    /// Check if a string is alphanumeric
    /// فحص إذا كان النص حرفي رقمي
    pub fn is_alphanumeric(s: &str) -> bool {
        !s.is_empty() && s.chars().all(|c| c.is_alphanumeric())
    }
    
    /// Check if a string is a strong password
    /// فحص إذا كان النص كلمة مرور قوية
    pub fn is_strong_password(password: &str) -> bool {
        password.len() >= 8
            && password.chars().any(|c| c.is_lowercase())
            && password.chars().any(|c| c.is_uppercase())
            && password.chars().any(|c| c.is_numeric())
            && password.chars().any(|c| !c.is_alphanumeric())
    }
    
    /// Validate required field
    /// تحقق من حقل مطلوب
    pub fn required(value: &str, field: &str) -> Result<(), ValidationError> {
        if value.trim().is_empty() {
            Err(ValidationError {
                field: field.to_string(),
                message: format!("Field '{}' is required", field),
                code: "REQUIRED".to_string(),
            })
        } else {
            Ok(())
        }
    }
    
    /// Validate min length
    /// تحقق من الحد الأدنى للطول
    pub fn min_length(value: &str, field: &str, min: usize) -> Result<(), ValidationError> {
        if value.len() < min {
            Err(ValidationError {
                field: field.to_string(),
                message: format!("Field '{}' must be at least {} characters", field, min),
                code: "MIN_LENGTH".to_string(),
            })
        } else {
            Ok(())
        }
    }
    
    /// Validate max length
    /// تحقق من الحد الأقصى للطول
    pub fn max_length(value: &str, field: &str, max: usize) -> Result<(), ValidationError> {
        if value.len() > max {
            Err(ValidationError {
                field: field.to_string(),
                message: format!("Field '{}' must be at most {} characters", field, max),
                code: "MAX_LENGTH".to_string(),
            })
        } else {
            Ok(())
        }
    }
    
    /// Validate email format
    /// تحقق من صيغة البريد الإلكتروني
    pub fn email(value: &str, field: &str) -> Result<(), ValidationError> {
        if !Self::is_email(value) {
            Err(ValidationError {
                field: field.to_string(),
                message: format!("Field '{}' must be a valid email", field),
                code: "INVALID_EMAIL".to_string(),
            })
        } else {
            Ok(())
        }
    }
    
    /// Validate range
    /// تحقق من النطاق
    pub fn range<T: PartialOrd + std::fmt::Debug>(value: T, field: &str, min: T, max: T) -> Result<(), ValidationError> {
        if value < min || value > max {
            Err(ValidationError {
                field: field.to_string(),
                message: format!("Field '{}' must be between {:?} and {:?}", field, min, max),
                code: "OUT_OF_RANGE".to_string(),
            })
        } else {
            Ok(())
        }
    }
    
    /// Validate against a regex pattern
    /// تحقق باستخدام regex
    pub fn pattern(value: &str, field: &str, pattern: &str) -> Result<(), ValidationError> {
        let re = Regex::new(pattern)
            .map_err(|e| ValidationError {
                field: field.to_string(),
                message: format!("Invalid regex pattern: {}", e),
                code: "INVALID_PATTERN".to_string(),
            })?;
        
        if !re.is_match(value) {
            Err(ValidationError {
                field: field.to_string(),
                message: format!("Field '{}' has invalid format", field),
                code: "PATTERN_MISMATCH".to_string(),
            })
        } else {
            Ok(())
        }
    }
}

/// Trait for validation rules
pub trait ValidationRule {
    type Rule;
}

/// A validation result with multiple errors
/// نتيجة تحقق مع أخطاء متعددة
pub struct ValidationResult {
    pub errors: Vec<ValidationError>,
    pub data: HashMap<String, String>,
}

impl ValidationResult {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
    
    pub fn add_error(&mut self, field: &str, message: &str, code: &str) {
        self.errors.push(ValidationError {
            field: field.to_string(),
            message: message.to_string(),
            code: code.to_string(),
        });
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
    fn test_email_validation() {
        assert!(Validator::is_email("test@example.com"));
        assert!(!Validator::is_email("invalid-email"));
    }
    
    #[test]
    fn test_password_strength() {
        assert!(Validator::is_strong_password("Str0ng!Pass"));
        assert!(!Validator::is_strong_password("weak"));
    }
    
    #[test]
    fn test_required() {
        assert!(Validator::required("", "name").is_err());
        assert!(Validator::required("value", "name").is_ok());
    }
}
