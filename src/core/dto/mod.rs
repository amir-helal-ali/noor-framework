// ============================================================
// DTO (Data Transfer Objects) - كائنات نقل البيانات
// ============================================================
// Typed objects for transferring data between layers.
// Provides validation, serialization, and documentation.
//
// كائنات منسقة لنقل البيانات بين الطبقات.
// ============================================================

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// DTO trait
pub trait Dto: Serialize + for<'de> Deserialize<'de> + Send + Sync {
    /// Validate the DTO fields
    fn validate(&self) -> Result<(), Vec<DtoError>>;
    
    /// Get the DTO name
    fn dto_name() -> &'static str;
}

/// DTO validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DtoError {
    pub field: String,
    pub message: String,
    pub code: String,
}

impl DtoError {
    pub fn new(field: &str, message: &str, code: &str) -> Self {
        Self {
            field: field.to_string(),
            message: message.to_string(),
            code: code.to_string(),
        }
    }
    
    pub fn required(field: &str) -> Self {
        Self::new(field, &format!("{} is required", field), "REQUIRED")
    }
    
    pub fn invalid(field: &str, message: &str) -> Self {
        Self::new(field, message, "INVALID")
    }
}

/// DTO collection for list responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DtoCollection<T> {
    pub data: Vec<T>,
    pub total: usize,
    pub page: u32,
    pub per_page: u32,
}

impl<T: Serialize> DtoCollection<T> {
    pub fn new(data: Vec<T>, total: usize, page: u32, per_page: u32) -> Self {
        Self { data, total, page, per_page }
    }
}

/// Pagination DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationDto {
    pub page: u32,
    pub per_page: u32,
    pub sort: Option<String>,
    pub order: Option<String>,
}

impl Dto for PaginationDto {
    fn validate(&self) -> Result<(), Vec<DtoError>> {
        let mut errors = Vec::new();
        
        if self.page == 0 {
            errors.push(DtoError::invalid("page", "Page must be greater than 0"));
        }
        
        if self.per_page == 0 || self.per_page > 100 {
            errors.push(DtoError::invalid("per_page", "Per page must be between 1 and 100"));
        }
        
        if let Some(ref order) = self.order {
            if order != "asc" && order != "desc" {
                errors.push(DtoError::invalid("order", "Order must be 'asc' or 'desc'"));
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    fn dto_name() -> &'static str {
        "PaginationDto"
    }
}

/// User creation DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserDto {
    pub name: String,
    pub email: String,
    pub password: String,
    pub role: Option<String>,
}

impl Dto for CreateUserDto {
    fn validate(&self) -> Result<(), Vec<DtoError>> {
        let mut errors = Vec::new();
        
        if self.name.trim().is_empty() {
            errors.push(DtoError::required("name"));
        }
        
        if self.name.len() < 2 {
            errors.push(DtoError::invalid("name", "Name must be at least 2 characters"));
        }
        
        if self.email.trim().is_empty() {
            errors.push(DtoError::required("email"));
        } else if !crate::core::security::Validator::is_email(&self.email) {
            errors.push(DtoError::invalid("email", "Must be a valid email address"));
        }
        
        if self.password.len() < 8 {
            errors.push(DtoError::invalid("password", "Password must be at least 8 characters"));
        }
        
        if !crate::core::security::Validator::is_strong_password(&self.password) {
            errors.push(DtoError::invalid("password", "Password must contain uppercase, lowercase, number, and special character"));
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    fn dto_name() -> &'static str {
        "CreateUserDto"
    }
}

/// User update DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserDto {
    pub name: Option<String>,
    pub email: Option<String>,
    pub role: Option<String>,
    pub active: Option<bool>,
}

impl Dto for UpdateUserDto {
    fn validate(&self) -> Result<(), Vec<DtoError>> {
        let mut errors = Vec::new();
        
        if let Some(ref name) = self.name {
            if name.len() < 2 {
                errors.push(DtoError::invalid("name", "Name must be at least 2 characters"));
            }
        }
        
        if let Some(ref email) = self.email {
            if !crate::core::security::Validator::is_email(email) {
                errors.push(DtoError::invalid("email", "Must be a valid email address"));
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    fn dto_name() -> &'static str {
        "UpdateUserDto"
    }
}

/// Login DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginDto {
    pub email: String,
    pub password: String,
    pub remember: Option<bool>,
}

impl Dto for LoginDto {
    fn validate(&self) -> Result<(), Vec<DtoError>> {
        let mut errors = Vec::new();
        
        if self.email.trim().is_empty() {
            errors.push(DtoError::required("email"));
        } else if !crate::core::security::Validator::is_email(&self.email) {
            errors.push(DtoError::invalid("email", "Must be a valid email address"));
        }
        
        if self.password.is_empty() {
            errors.push(DtoError::required("password"));
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    fn dto_name() -> &'static str {
        "LoginDto"
    }
}

/// Response DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseDto<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
    pub errors: Option<Vec<DtoError>>,
    pub timestamp: i64,
}

impl<T: Serialize> ResponseDto<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
            errors: None,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
    
    pub fn error(message: &str, errors: Vec<DtoError>) -> Self {
        Self {
            success: false,
            data: None,
            message: Some(message.to_string()),
            errors: Some(errors),
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
    
    pub fn with_message(mut self, message: &str) -> Self {
        self.message = Some(message.to_string());
        self
    }
}

/// Macro for creating DTOs
#[macro_export]
macro_rules! dto {
    ($name:ident {
        $($field:ident: $type:ty),* $(,)?
    }) => {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct $name {
            $(pub $field: $type,)*
        }
        
        impl $crate::core::dto::Dto for $name {
            fn validate(&self) -> Result<(), Vec<$crate::core::dto::DtoError>> {
                Ok(()) // Override for custom validation
            }
            
            fn dto_name() -> &'static str {
                stringify!($name)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_user_dto_valid() {
        let dto = CreateUserDto {
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            password: "Str0ng!Pass".to_string(),
            role: Some("user".to_string()),
        };
        
        assert!(dto.validate().is_ok());
    }
    
    #[test]
    fn test_create_user_dto_invalid() {
        let dto = CreateUserDto {
            name: "J".to_string(),
            email: "invalid-email".to_string(),
            password: "weak".to_string(),
            role: None,
        };
        
        let result = dto.validate();
        
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.field == "name"));
        assert!(errors.iter().any(|e| e.field == "email"));
        assert!(errors.iter().any(|e| e.field == "password"));
    }
    
    #[test]
    fn test_login_dto() {
        let dto = LoginDto {
            email: "user@example.com".to_string(),
            password: "password".to_string(),
            remember: Some(true),
        };
        
        assert!(dto.validate().is_ok());
        
        let invalid_dto = LoginDto {
            email: "".to_string(),
            password: "".to_string(),
            remember: None,
        };
        
        assert!(invalid_dto.validate().is_err());
    }
    
    #[test]
    fn test_pagination_dto() {
        let valid = PaginationDto {
            page: 1,
            per_page: 20,
            sort: Some("name".to_string()),
            order: Some("asc".to_string()),
        };
        
        assert!(valid.validate().is_ok());
        
        let invalid = PaginationDto {
            page: 0,
            per_page: 200,
            sort: None,
            order: Some("invalid".to_string()),
        };
        
        assert!(invalid.validate().is_err());
    }
    
    #[test]
    fn test_response_dto() {
        let success = ResponseDto::success(serde_json::json!({"id": 1}));
        assert!(success.success);
        
        let error = ResponseDto::<serde_json::Value>::error("Validation failed", vec![
            DtoError::required("email"),
        ]);
        assert!(!error.success);
        assert!(error.errors.is_some());
    }
}
