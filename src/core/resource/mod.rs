// ============================================================
// API Resources - موارد API
// ============================================================
// Transform models into API responses with consistent formatting.
// Separates internal data structure from API output.
//
// تحويل النماذج إلى استجابات API بتنسيق متسق.
// ============================================================

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// API Resource trait
pub trait ApiResource: Send + Sync {
    type Model;
    
    /// Transform a model into an array
    fn toArray(&self, model: &Self::Model) -> serde_json::Value;
    
    /// Transform with additional metadata
    fn toArray_with(&self, model: &Self::Model, extra: HashMap<String, serde_json::Value>) -> serde_json::Value {
        let mut data = self.toArray(model);
        if let serde_json::Value::Object(ref mut map) = data {
            for (k, v) in extra {
                map.insert(k, v);
            }
        }
        data
    }
}

/// Resource collection for paginated results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceCollection {
    pub data: Vec<serde_json::Value>,
    pub meta: ResourceMeta,
    pub links: ResourceLinks,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMeta {
    pub current_page: u32,
    pub per_page: u32,
    pub total: u64,
    pub last_page: u32,
    pub from: Option<u64>,
    pub to: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLinks {
    pub first: Option<String>,
    pub last: Option<String>,
    pub prev: Option<String>,
    pub next: Option<String>,
}

impl ResourceCollection {
    /// Create a resource collection from items
    pub fn new<T, R: ApiResource<Model = T>>(
        resource: &R,
        items: &[T],
        total: u64,
        page: u32,
        per_page: u32,
        path: &str,
    ) -> Self {
        let data: Vec<serde_json::Value> = items
            .iter()
            .map(|item| resource.toArray(item))
            .collect();
        
        let last_page = if per_page == 0 { 1 } else { ((total as f64 / per_page as f64).ceil() as u32).max(1) };
        
        let from = if total == 0 { None } else { Some(((page - 1) * per_page) as u64 + 1) };
        let to = if total == 0 { None } else { Some(std::cmp::min((page * per_page) as u64, total)) };
        
        let prev = if page > 1 { Some(format!("{}?page={}", path, page - 1)) } else { None };
        let next = if page < last_page { Some(format!("{}?page={}", path, page + 1)) } else { None };
        
        Self {
            data,
            meta: ResourceMeta {
                current_page: page,
                per_page,
                total,
                last_page,
                from,
                to,
            },
            links: ResourceLinks {
                first: Some(format!("{}?page=1", path)),
                last: Some(format!("{}?page={}", path, last_page)),
                prev,
                next,
            },
        }
    }
    
    /// Convert to JSON response
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::json!({"error": "Serialization failed"}))
    }
}

/// JSON API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonResponse {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub message: Option<String>,
    pub errors: Option<Vec<ErrorDetail>>,
    pub meta: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub field: Option<String>,
    pub message: String,
    pub code: String,
}

impl JsonResponse {
    /// Success response
    pub fn success(data: serde_json::Value) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
            errors: None,
            meta: None,
        }
    }
    
    /// Success with message
    pub fn success_with_message(data: serde_json::Value, message: &str) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: Some(message.to_string()),
            errors: None,
            meta: None,
        }
    }
    
    /// Error response
    pub fn error(message: &str) -> Self {
        Self {
            success: false,
            data: None,
            message: Some(message.to_string()),
            errors: None,
            meta: None,
        }
    }
    
    /// Validation error response
    pub fn validation_errors(errors: Vec<ErrorDetail>) -> Self {
        Self {
            success: false,
            data: None,
            message: Some("Validation failed".to_string()),
            errors: Some(errors),
            meta: None,
        }
    }
    
    /// Add metadata
    pub fn with_meta(mut self, meta: serde_json::Value) -> Self {
        self.meta = Some(meta);
        self
    }
}

/// Macro for creating resources
#[macro_export]
macro_rules! resource {
    ($name:ident, $model:ty, $body:expr) => {
        pub struct $name;
        
        impl $crate::core::resource::ApiResource for $name {
            type Model = $model;
            
            fn toArray(&self, model: &Self::Model) -> serde_json::Value {
                $body(model)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[derive(Debug, Clone)]
    struct User {
        id: i64,
        name: String,
        email: String,
        password: String,  // Should not be exposed
    }
    
    struct UserResource;
    
    impl ApiResource for UserResource {
        type Model = User;
        
        fn toArray(&self, user: &User) -> serde_json::Value {
            serde_json::json!({
                "id": user.id,
                "name": user.name,
                "email": user.email,
                // password is intentionally omitted
            })
        }
    }
    
    #[test]
    fn test_resource_transform() {
        let user = User {
            id: 1,
            name: "John".to_string(),
            email: "john@example.com".to_string(),
            password: "secret".to_string(),
        };
        
        let resource = UserResource;
        let json = resource.toArray(&user);
        
        assert_eq!(json["id"], 1);
        assert_eq!(json["name"], "John");
        assert_eq!(json["email"], "john@example.com");
        assert!(json.get("password").is_none()); // Password should not be exposed
    }
    
    #[test]
    fn test_resource_collection() {
        let users = vec![
            User { id: 1, name: "John".to_string(), email: "john@example.com".to_string(), password: "secret".to_string() },
            User { id: 2, name: "Jane".to_string(), email: "jane@example.com".to_string(), password: "secret".to_string() },
        ];
        
        let resource = UserResource;
        let collection = ResourceCollection::new(&resource, &users, 100, 1, 10, "/api/users");
        
        assert_eq!(collection.data.len(), 2);
        assert_eq!(collection.meta.total, 100);
        assert!(collection.links.next.is_some());
    }
    
    #[test]
    fn test_json_response() {
        let success = JsonResponse::success(serde_json::json!({"id": 1}));
        assert!(success.success);
        
        let error = JsonResponse::error("Not found");
        assert!(!error.success);
        
        let validation = JsonResponse::validation_errors(vec![
            ErrorDetail {
                field: Some("email".to_string()),
                message: "Invalid email".to_string(),
                code: "INVALID_EMAIL".to_string(),
            }
        ]);
        assert!(validation.errors.is_some());
    }
}
