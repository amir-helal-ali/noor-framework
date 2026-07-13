// ============================================================
// Form Requests - طلبات النماذج
// ============================================================
// Encapsulate request validation logic in dedicated classes.
// Separates validation rules from controllers.
//
// تغليف منطق التحقق في فئات مخصصة.
// ============================================================

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::core::validation::{Validator, RuleBuilder, ValidationResult};
use crate::core::http::Request;

/// Form request trait
pub trait FormRequest: Send + Sync {
    /// Get validation rules
    fn rules(&self) -> Vec<(String, Vec<crate::core::validation::ValidationRule>)>;
    
    /// Get custom error messages
    fn messages(&self) -> HashMap<String, String> {
        HashMap::new()
    }
    
    /// Get authorized check
    fn authorize(&self, request: &Request) -> bool {
        let _ = request;
        true
    }
    
    /// Validate the request
    fn validate(&self, request: &Request) -> FormRequestResult {
        // Check authorization
        if !self.authorize(request) {
            return FormRequestResult {
                authorized: false,
                valid: false,
                errors: HashMap::new(),
                data: serde_json::Value::Null,
            };
        }
        
        // Build validator
        let mut validator = Validator::new();
        
        for (field, rules) in self.rules() {
            validator = validator.rule(&field, rules);
        }
        
        // Extract data from request
        let data = if request.content_type().map(|c| c.contains("json")).unwrap_or(false) {
            request.json::<serde_json::Value>().unwrap_or(serde_json::Value::Null)
        } else {
            let form = request.form();
            let map: serde_json::Map<String, serde_json::Value> = form
                .into_iter()
                .map(|(k, v)| (k, serde_json::Value::String(v)))
                .collect();
            serde_json::Value::Object(map)
        };
        
        // Validate
        let result = validator.validate(&data);
        
        FormRequestResult {
            authorized: true,
            valid: result.valid,
            errors: result.errors,
            data,
        }
    }
}

/// Form request validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormRequestResult {
    pub authorized: bool,
    pub valid: bool,
    pub errors: HashMap<String, Vec<String>>,
    pub data: serde_json::Value,
}

impl FormRequestResult {
    /// Check if the request passed validation
    pub fn passes(&self) -> bool {
        self.authorized && self.valid
    }
    
    /// Check if validation failed
    pub fn fails(&self) -> bool {
        !self.passes()
    }
    
    /// Get the first error message
    pub fn first_error(&self) -> Option<String> {
        self.errors.values().next().and_then(|v| v.first().cloned())
    }
    
    /// Get all error messages as a flat list
    pub fn all_errors(&self) -> Vec<String> {
        self.errors.values().flat_map(|v| v.iter().cloned()).collect()
    }
}

/// Helper macro for defining form requests
#[macro_export]
macro_rules! form_request {
    ($name:ident {
        $($field:literal => { $($rule:ident $(( $($arg:expr),* ))?);* $(;)? }),* $(,)?
    }) => {
        pub struct $name;
        
        impl $crate::core::form_request::FormRequest for $name {
            fn rules(&self) -> Vec<(String, Vec<$crate::core::validation::ValidationRule>)> {
                vec![
                    $(
                        (
                            $field.to_string(),
                            {
                                let mut builder = $crate::core::validation::RuleBuilder::new();
                                $(
                                    builder = builder.$rule($($($arg),*)*);
                                )*
                                builder.build()
                            }
                        ),
                    )*
                ]
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::validation::RuleBuilder;
    use crate::core::http::{Request, Method};
    
    struct CreateUserRequest;
    
    impl FormRequest for CreateUserRequest {
        fn rules(&self) -> Vec<(String, Vec<crate::core::validation::ValidationRule>)> {
            vec![
                ("name".to_string(), RuleBuilder::new().required().min(2).max(50).build()),
                ("email".to_string(), RuleBuilder::new().required().email().build()),
                ("password".to_string(), RuleBuilder::new().required().min(8).build()),
            ]
        }
    }
    
    #[test]
    fn test_form_request_valid() {
        let request = Request::new(Method::Post, "/users".to_string());
        // In a real test, we'd set the JSON body
        
        let form_request = CreateUserRequest;
        let result = form_request.validate(&request);
        
        // Since we don't have a body, validation should fail
        assert!(result.fails());
    }
    
    #[test]
    fn test_form_request_authorization() {
        struct AuthorizedRequest;
        
        impl FormRequest for AuthorizedRequest {
            fn rules(&self) -> Vec<(String, Vec<crate::core::validation::ValidationRule>)> {
                vec![]
            }
            
            fn authorize(&self, _request: &Request) -> bool {
                false // Always unauthorized
            }
        }
        
        let request = Request::new(Method::Get, "/".to_string());
        let form_request = AuthorizedRequest;
        let result = form_request.validate(&request);
        
        assert!(!result.authorized);
        assert!(result.fails());
    }
}
