// ============================================================
// Multi-tenancy - تعدد المستأجرين
// ============================================================
// Support for multi-tenant applications where multiple
// organizations share the same application instance.
//
// دعم التطبيقات متعددة المستأجرين.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// Tenant model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: String,
    pub name: String,
    pub domain: Option<String>,
    pub subdomain: Option<String>,
    pub plan: TenantPlan,
    pub status: TenantStatus,
    pub settings: HashMap<String, serde_json::Value>,
    pub database: Option<String>,  // Separate database name
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TenantPlan {
    Free,
    Starter,
    Pro,
    Enterprise,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TenantStatus {
    Active,
    Suspended,
    Trial,
    Canceled,
}

impl Tenant {
    pub fn new(name: &str) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            domain: None,
            subdomain: None,
            plan: TenantPlan::Free,
            status: TenantStatus::Trial,
            settings: HashMap::new(),
            database: None,
            created_at: now,
            updated_at: now,
        }
    }
    
    pub fn with_domain(mut self, domain: &str) -> Self {
        self.domain = Some(domain.to_string());
        self
    }
    
    pub fn with_subdomain(mut self, subdomain: &str) -> Self {
        self.subdomain = Some(subdomain.to_string());
        self
    }
    
    pub fn with_plan(mut self, plan: TenantPlan) -> Self {
        self.plan = plan;
        self
    }
    
    pub fn is_active(&self) -> bool {
        self.status == TenantStatus::Active
    }
    
    pub fn is_trial(&self) -> bool {
        self.status == TenantStatus::Trial
    }
}

/// Tenant resolution strategy
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResolutionStrategy {
    /// Extract tenant from subdomain (tenant.example.com)
    Subdomain,
    /// Extract tenant from custom domain
    Domain,
    /// Extract tenant from header (X-Tenant-ID)
    Header,
    /// Extract tenant from path (/tenants/{id}/...)
    Path,
    /// Extract tenant from query parameter (?tenant=...)
    QueryParam,
}

/// Tenant manager
pub struct TenantManager {
    tenants: Arc<RwLock<HashMap<String, Tenant>>>,
    domain_map: Arc<RwLock<HashMap<String, String>>>,  // domain -> tenant_id
    subdomain_map: Arc<RwLock<HashMap<String, String>>>,  // subdomain -> tenant_id
    strategy: ResolutionStrategy,
    header_name: String,
    query_param: String,
    current_tenant: Arc<RwLock<Option<Tenant>>>,
}

impl Default for TenantManager {
    fn default() -> Self {
        Self::new(ResolutionStrategy::Subdomain)
    }
}

impl TenantManager {
    pub fn new(strategy: ResolutionStrategy) -> Self {
        Self {
            tenants: Arc::new(RwLock::new(HashMap::new())),
            domain_map: Arc::new(RwLock::new(HashMap::new())),
            subdomain_map: Arc::new(RwLock::new(HashMap::new())),
            strategy,
            header_name: "X-Tenant-ID".to_string(),
            query_param: "tenant".to_string(),
            current_tenant: Arc::new(RwLock::new(None)),
        }
    }
    
    /// Register a tenant
    pub fn register(&self, tenant: Tenant) -> String {
        let id = tenant.id.clone();
        
        // Register domain mapping
        if let Some(ref domain) = tenant.domain {
            self.domain_map.write().insert(domain.clone(), id.clone());
        }
        
        // Register subdomain mapping
        if let Some(ref subdomain) = tenant.subdomain {
            self.subdomain_map.write().insert(subdomain.clone(), id.clone());
        }
        
        self.tenants.write().insert(id.clone(), tenant);
        id
    }
    
    /// Create a new tenant
    pub fn create(&self, name: &str) -> String {
        let tenant = Tenant::new(name);
        self.register(tenant)
    }
    
    /// Get a tenant by ID
    pub fn get(&self, id: &str) -> Option<Tenant> {
        self.tenants.read().get(id).cloned()
    }
    
    /// Get tenant by domain
    pub fn get_by_domain(&self, domain: &str) -> Option<Tenant> {
        let tenant_id = self.domain_map.read().get(domain).cloned()?;
        self.get(&tenant_id)
    }
    
    /// Get tenant by subdomain
    pub fn get_by_subdomain(&self, subdomain: &str) -> Option<Tenant> {
        let tenant_id = self.subdomain_map.read().get(subdomain).cloned()?;
        self.get(&tenant_id)
    }
    
    /// Resolve tenant from request
    pub fn resolve_from_request(&self, request: &crate::core::http::Request) -> Option<Tenant> {
        match self.strategy {
            ResolutionStrategy::Subdomain => {
                // Extract subdomain from Host header
                let host = request.header("host")?;
                let parts: Vec<&str> = host.split('.').collect();
                
                if parts.len() > 2 {
                    let subdomain = parts[0];
                    return self.get_by_subdomain(subdomain);
                }
                
                None
            }
            ResolutionStrategy::Domain => {
                let host = request.header("host")?;
                self.get_by_domain(host)
            }
            ResolutionStrategy::Header => {
                let tenant_id = request.header(&self.header_name)?;
                self.get(tenant_id)
            }
            ResolutionStrategy::Path => {
                // Extract from path: /tenants/{id}/...
                let parts: Vec<&str> = request.path.split('/').filter(|s| !s.is_empty()).collect();
                
                if parts.len() >= 2 && parts[0] == "tenants" {
                    return self.get(parts[1]);
                }
                
                None
            }
            ResolutionStrategy::QueryParam => {
                let tenant_id = request.query(&self.query_param)?;
                self.get(tenant_id)
            }
        }
    }
    
    /// Set the current tenant for the request
    pub fn set_current(&self, tenant: Tenant) {
        *self.current_tenant.write() = Some(tenant);
    }
    
    /// Get the current tenant
    pub fn current(&self) -> Option<Tenant> {
        self.current_tenant.read().clone()
    }
    
    /// Clear the current tenant
    pub fn clear_current(&self) {
        *self.current_tenant.write() = None;
    }
    
    /// Update a tenant
    pub fn update(&self, id: &str, updates: impl FnOnce(&mut Tenant)) -> Option<Tenant> {
        let mut tenants = self.tenants.write();
        
        if let Some(tenant) = tenants.get_mut(id) {
            updates(tenant);
            tenant.updated_at = chrono::Utc::now().timestamp();
            return Some(tenant.clone());
        }
        
        None
    }
    
    /// Delete a tenant
    pub fn delete(&self, id: &str) -> bool {
        let removed = self.tenants.write().remove(id).is_some();
        
        if removed {
            self.domain_map.write().retain(|_, v| v != id);
            self.subdomain_map.write().retain(|_, v| v != id);
        }
        
        removed
    }
    
    /// List all tenants
    pub fn list(&self) -> Vec<Tenant> {
        self.tenants.read().values().cloned().collect()
    }
    
    /// Count tenants
    pub fn count(&self) -> usize {
        self.tenants.read().len()
    }
    
    /// Check if tenant has feature access based on plan
    pub fn has_feature(&self, tenant_id: &str, feature: &str) -> bool {
        let tenant = match self.get(tenant_id) {
            Some(t) => t,
            None => return false,
        };
        
        if !tenant.is_active() && !tenant.is_trial() {
            return false;
        }
        
        match (&tenant.plan, feature) {
            (TenantPlan::Enterprise, _) => true,
            (TenantPlan::Pro, f) if !["white_label", "custom_domain"].contains(&f) => true,
            (TenantPlan::Starter, f) if !["white_label", "custom_domain", "api_access", "advanced_analytics"].contains(&f) => true,
            (TenantPlan::Free, f) if ["basic_features", "community_support"].contains(&f) => true,
            (TenantPlan::Custom(_), _) => true,
            _ => false,
        }
    }
}

/// Tenant scope for database queries
pub struct TenantScope {
    pub tenant_id: String,
}

impl TenantScope {
    pub fn new(tenant_id: &str) -> Self {
        Self {
            tenant_id: tenant_id.to_string(),
        }
    }
    
    /// Apply scope to a query builder
    pub fn apply(&self, query: crate::core::orm::QueryBuilder) -> crate::core::orm::QueryBuilder {
        query.where_("tenant_id", "=", self.tenant_id.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_tenant_creation() {
        let tenant = Tenant::new("Acme Corp")
            .with_domain("acme.example.com")
            .with_subdomain("acme")
            .with_plan(TenantPlan::Pro);
        
        assert_eq!(tenant.name, "Acme Corp");
        assert_eq!(tenant.domain, Some("acme.example.com".to_string()));
        assert_eq!(tenant.plan, TenantPlan::Pro);
    }
    
    #[test]
    fn test_tenant_manager() {
        let manager = TenantManager::new(ResolutionStrategy::Subdomain);
        
        let tenant = Tenant::new("Test Corp")
            .with_subdomain("test");
        let id = manager.register(tenant);
        
        assert_eq!(manager.count(), 1);
        assert!(manager.get(&id).is_some());
        assert!(manager.get_by_subdomain("test").is_some());
    }
    
    #[test]
    fn test_tenant_features() {
        let manager = TenantManager::new(ResolutionStrategy::Header);
        
        let free_tenant = manager.create("Free Corp");
        manager.update(&free_tenant, |t| {
            t.status = TenantStatus::Active;
        });
        
        assert!(manager.has_feature(&free_tenant, "basic_features"));
        assert!(!manager.has_feature(&free_tenant, "api_access"));
        
        let enterprise_tenant = manager.create("Enterprise Corp");
        manager.update(&enterprise_tenant, |t| {
            t.plan = TenantPlan::Enterprise;
            t.status = TenantStatus::Active;
        });
        
        assert!(manager.has_feature(&enterprise_tenant, "api_access"));
        assert!(manager.has_feature(&enterprise_tenant, "white_label"));
    }
    
    #[test]
    fn test_tenant_resolution_by_header() {
        let manager = TenantManager::new(ResolutionStrategy::Header);
        
        let tenant = Tenant::new("Test");
        let id = tenant.id.clone();
        manager.register(tenant);
        
        let mut request = crate::core::http::Request::new(
            crate::core::http::Method::Get,
            "/api/data".to_string(),
        );
        request.headers.insert("X-Tenant-ID".to_string(), id.clone());
        
        let resolved = manager.resolve_from_request(&request);
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().id, id);
    }
    
    #[test]
    fn test_tenant_scope() {
        let scope = TenantScope::new("tenant-123");
        
        let query = crate::core::orm::QueryBuilder::table("posts");
        let scoped = scope.apply(query);
        
        let (sql, params) = scoped.to_sql();
        assert!(sql.contains("tenant_id"));
        assert!(params.contains(&serde_json::json!("tenant-123")));
    }
}
