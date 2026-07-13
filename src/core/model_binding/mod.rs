// ============================================================
// Route Model Binding - ربط النماذج بالمسارات
// ============================================================
// Automatically resolve route parameters to model instances.
// e.g., /users/{user} -> User model
//
// ربط معاملات المسار بالنماذج تلقائياً.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// Model resolver trait
pub trait ModelResolver: Send + Sync {
    type Model: Clone + Send + Sync;
    
    /// Resolve a model by its ID
    fn resolve(&self, id: &str) -> Option<Self::Model>;
}

/// Model binding registry
pub struct ModelBinder {
    resolvers: Arc<RwLock<HashMap<String, Arc<dyn ModelResolver<Model = serde_json::Value> + Send + Sync>>>>,
    /// Custom parameter name -> model mapping
    bindings: Arc<RwLock<HashMap<String, String>>>,
}

impl Default for ModelBinder {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelBinder {
    pub fn new() -> Self {
        Self {
            resolvers: Arc::new(RwLock::new(HashMap::new())),
            bindings: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register a model resolver
    pub fn bind<R>(&self, param_name: &str, resolver: Arc<R>)
    where
        R: ModelResolver<Model = serde_json::Value> + Send + Sync + 'static,
    {
        self.resolvers.write().insert(param_name.to_string(), resolver);
    }
    
    /// Register a binding pattern (e.g., "user" -> "User")
    pub fn pattern(&self, param: &str, model: &str) {
        self.bindings.write().insert(param.to_string(), model.to_string());
    }
    
    /// Resolve a parameter to a model
    pub fn resolve(&self, param_name: &str, param_value: &str) -> Option<serde_json::Value> {
        let resolvers = self.resolvers.read();
        
        if let Some(resolver) = resolvers.get(param_name) {
            return resolver.resolve(param_value);
        }
        
        None
    }
    
    /// Check if a parameter has a binding
    pub fn has_binding(&self, param_name: &str) -> bool {
        self.resolvers.read().contains_key(param_name)
    }
    
    /// List all registered bindings
    pub fn list_bindings(&self) -> Vec<String> {
        self.resolvers.read().keys().cloned().collect()
    }
}

/// Implicit model binding (auto-resolve from route parameter)
pub struct ImplicitBinding {
    /// Map of parameter names to resolver functions
    resolvers: HashMap<String, Box<dyn Fn(&str) -> Option<serde_json::Value> + Send + Sync>>,
}

impl Default for ImplicitBinding {
    fn default() -> Self {
        Self::new()
    }
}

impl ImplicitBinding {
    pub fn new() -> Self {
        Self {
            resolvers: HashMap::new(),
        }
    }
    
    /// Register a resolver for a parameter
    pub fn resolver<F>(&mut self, param: &str, resolver: F)
    where
        F: Fn(&str) -> Option<serde_json::Value> + Send + Sync + 'static,
    {
        self.resolvers.insert(param.to_string(), Box::new(resolver));
    }
    
    /// Resolve a parameter
    pub fn resolve(&self, param: &str, value: &str) -> Option<serde_json::Value> {
        self.resolvers.get(param).and_then(|f| f(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    struct UserResolver;
    
    impl ModelResolver for UserResolver {
        type Model = serde_json::Value;
        
        fn resolve(&self, id: &str) -> Option<serde_json::Value> {
            if id == "1" {
                Some(serde_json::json!({
                    "id": 1,
                    "name": "John Doe",
                    "email": "john@example.com"
                }))
            } else {
                None
            }
        }
    }
    
    #[test]
    fn test_model_binder() {
        let binder = ModelBinder::new();
        binder.bind("user", Arc::new(UserResolver));
        
        assert!(binder.has_binding("user"));
        
        let model = binder.resolve("user", "1").unwrap();
        assert_eq!(model["name"], "John Doe");
        
        let not_found = binder.resolve("user", "999");
        assert!(not_found.is_none());
    }
    
    #[test]
    fn test_implicit_binding() {
        let mut binding = ImplicitBinding::new();
        
        binding.resolver("post", |id| {
            if id == "1" {
                Some(serde_json::json!({"id": 1, "title": "Hello"}))
            } else {
                None
            }
        });
        
        let post = binding.resolve("post", "1").unwrap();
        assert_eq!(post["title"], "Hello");
    }
}
