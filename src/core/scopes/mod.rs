// ============================================================
// Model Scopes - نطاقات النماذج
// ============================================================
// Reusable query filters that can be applied to query builders.
// e.g., User::active(), User::admins(), Post::published()
//
// فلاتر استعلام قابلة لإعادة الاستخدام.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// Scope function type
type ScopeFn = Arc<dyn Fn(crate::core::advanced_query::AdvancedQueryBuilder) -> crate::core::advanced_query::AdvancedQueryBuilder + Send + Sync>;

/// Scope registry
pub struct ScopeRegistry {
    scopes: Arc<RwLock<HashMap<String, ScopeFn>>>,
}

impl Default for ScopeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ScopeRegistry {
    pub fn new() -> Self {
        Self {
            scopes: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register a scope
    pub fn register<F>(&self, name: &str, scope: F) -> &Self
    where
        F: Fn(crate::core::advanced_query::AdvancedQueryBuilder) -> crate::core::advanced_query::AdvancedQueryBuilder + Send + Sync + 'static,
    {
        self.scopes.write().insert(name.to_string(), Arc::new(scope));
        self
    }
    
    /// Apply a scope to a query
    pub fn apply(&self, name: &str, query: crate::core::advanced_query::AdvancedQueryBuilder) -> Option<crate::core::advanced_query::AdvancedQueryBuilder> {
        self.scopes
            .read()
            .get(name)
            .map(|scope| scope(query))
    }
    
    /// Check if a scope exists
    pub fn has(&self, name: &str) -> bool {
        self.scopes.read().contains_key(name)
    }
    
    /// List all registered scopes
    pub fn list(&self) -> Vec<String> {
        self.scopes.read().keys().cloned().collect()
    }
    
    /// Get the count of registered scopes
    pub fn count(&self) -> usize {
        self.scopes.read().len()
    }
    
    /// Remove a scope
    pub fn remove(&self, name: &str) -> bool {
        self.scopes.write().remove(name).is_some()
    }
}

/// Common scopes that can be registered
pub fn register_common_scopes(registry: &ScopeRegistry) {
    // Active scope
    registry.register("active", |q| {
        q.where_("status", "=", "active")
    });
    
    // Inactive scope
    registry.register("inactive", |q| {
        q.where_("status", "=", "inactive")
    });
    
    // Pending scope
    registry.register("pending", |q| {
        q.where_("status", "=", "pending")
    });
    
    // Published scope
    registry.register("published", |q| {
        q.where_("status", "=", "published")
            .where_("published_at", "<=", chrono::Utc::now().timestamp())
    });
    
    // Draft scope
    registry.register("draft", |q| {
        q.where_("status", "=", "draft")
    });
    
    // Recent scope (last 7 days)
    registry.register("recent", |q| {
        let seven_days_ago = chrono::Utc::now().timestamp() - (7 * 24 * 3600);
        q.where_("created_at", ">=", seven_days_ago)
    });
    
    // Popular scope (most viewed)
    registry.register("popular", |q| {
        q.where_("views", ">", 100)
            .order_by("views", "desc")
    });
    
    // Featured scope
    registry.register("featured", |q| {
        q.where_("is_featured", "=", true)
    });
    
    // Verified scope
    registry.register("verified", |q| {
        q.where_not_null("email_verified_at")
    });
    
    // Admin scope
    registry.register("admins", |q| {
        q.where_("role", "=", "admin")
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_scope_registry() {
        let registry = ScopeRegistry::new();
        
        registry.register("active", |q| q.where_("status", "=", "active"));
        
        assert!(registry.has("active"));
        assert!(!registry.has("inactive"));
        assert_eq!(registry.count(), 1);
    }
    
    #[test]
    fn test_apply_scope() {
        let registry = ScopeRegistry::new();
        
        registry.register("active", |q| q.where_("status", "=", "active"));
        
        let query = crate::core::advanced_query::AdvancedQueryBuilder::table("users");
        let scoped = registry.apply("active", query);
        
        assert!(scoped.is_some());
        
        let (sql, _params) = scoped.unwrap().to_sql();
        assert!(sql.contains("status"));
    }
    
    #[test]
    fn test_common_scopes() {
        let registry = ScopeRegistry::new();
        register_common_scopes(&registry);
        
        assert!(registry.has("active"));
        assert!(registry.has("published"));
        assert!(registry.has("recent"));
        assert!(registry.has("popular"));
        assert!(registry.has("admins"));
    }
}
