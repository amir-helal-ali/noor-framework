// ============================================================
// Service Discovery - اكتشاف الخدمات
// ============================================================
// Service registry and discovery for microservices.
// Allows services to find and communicate with each other.
//
// تسجيل واكتشاف الخدمات للـ microservices.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// Service instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInstance {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub protocol: String,
    pub metadata: HashMap<String, String>,
    pub healthy: bool,
    pub registered_at: i64,
    pub last_heartbeat: i64,
}

impl ServiceInstance {
    pub fn new(name: &str, host: &str, port: u16) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            host: host.to_string(),
            port,
            protocol: "http".to_string(),
            metadata: HashMap::new(),
            healthy: true,
            registered_at: chrono::Utc::now().timestamp(),
            last_heartbeat: chrono::Utc::now().timestamp(),
        }
    }
    
    /// Get the full URL
    pub fn url(&self) -> String {
        format!("{}://{}:{}", self.protocol, self.host, self.port)
    }
    
    /// Get a specific URL path
    pub fn url_path(&self, path: &str) -> String {
        format!("{}{}", self.url(), path)
    }
    
    /// Check if heartbeat is stale
    pub fn is_stale(&self, timeout_secs: i64) -> bool {
        chrono::Utc::now().timestamp() - self.last_heartbeat > timeout_secs
    }
    
    /// Update heartbeat
    pub fn heartbeat(&mut self) {
        self.last_heartbeat = chrono::Utc::now().timestamp();
        self.healthy = true;
    }
}

/// Service registry
pub struct ServiceRegistry {
    services: Arc<RwLock<HashMap<String, Vec<ServiceInstance>>>>,
    heartbeat_timeout: i64,
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new(30) // 30 seconds heartbeat timeout
    }
}

impl ServiceRegistry {
    pub fn new(heartbeat_timeout_secs: i64) -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            heartbeat_timeout: heartbeat_timeout_secs,
        }
    }
    
    /// Register a service
    pub fn register(&self, instance: ServiceInstance) -> String {
        let id = instance.id.clone();
        let name = instance.name.clone();
        
        let mut services = self.services.write();
        services
            .entry(name.clone())
            .or_insert_with(Vec::new)
            .push(instance);
        
        tracing::info!("Service registered: {} ({})", name, id);
        
        id
    }
    
    /// Deregister a service
    pub fn deregister(&self, service_id: &str) -> bool {
        let mut services = self.services.write();
        
        for instances in services.values_mut() {
            if let Some(pos) = instances.iter().position(|i| i.id == service_id) {
                let instance = instances.remove(pos);
                tracing::info!("Service deregistered: {} ({})", instance.name, service_id);
                return true;
            }
        }
        
        false
    }
    
    /// Send heartbeat for a service
    pub fn heartbeat(&self, service_id: &str) -> bool {
        let mut services = self.services.write();
        
        for instances in services.values_mut() {
            if let Some(instance) = instances.iter_mut().find(|i| i.id == service_id) {
                instance.heartbeat();
                return true;
            }
        }
        
        false
    }
    
    /// Discover all instances of a service
    pub fn discover(&self, service_name: &str) -> Vec<ServiceInstance> {
        self.cleanup_stale();
        
        let services = self.services.read();
        
        services
            .get(service_name)
            .cloned()
            .unwrap_or_default()
    }
    
    /// Discover a single instance (round-robin would be better)
    pub fn discover_one(&self, service_name: &str) -> Option<ServiceInstance> {
        let instances = self.discover(service_name);
        
        if instances.is_empty() {
            return None;
        }
        
        // Simple: return first healthy instance
        instances.into_iter().find(|i| i.healthy)
    }
    
    /// Discover with load balancing (random)
    pub fn discover_random(&self, service_name: &str) -> Option<ServiceInstance> {
        use rand::seq::SliceRandom;
        
        let mut instances = self.discover(service_name);
        instances.shuffle(&mut rand::thread_rng());
        
        instances.into_iter().next()
    }
    
    /// List all registered services
    pub fn list_services(&self) -> Vec<String> {
        self.services.read().keys().cloned().collect()
    }
    
    /// Get all instances
    pub fn all_instances(&self) -> Vec<ServiceInstance> {
        let services = self.services.read();
        services.values().flat_map(|v| v.iter().cloned()).collect()
    }
    
    /// Get service count
    pub fn service_count(&self) -> usize {
        self.services.read().len()
    }
    
    /// Get instance count for a service
    pub fn instance_count(&self, service_name: &str) -> usize {
        self.services
            .read()
            .get(service_name)
            .map(|v| v.len())
            .unwrap_or(0)
    }
    
    /// Clean up stale services (no heartbeat)
    pub fn cleanup_stale(&self) -> usize {
        let mut cleaned = 0;
        let mut services = self.services.write();
        
        for instances in services.values_mut() {
            let initial = instances.len();
            instances.retain(|i| !i.is_stale(self.heartbeat_timeout));
            cleaned += initial - instances.len();
        }
        
        // Remove empty services
        services.retain(|_, v| !v.is_empty());
        
        cleaned
    }
    
    /// Mark a service as unhealthy
    pub fn mark_unhealthy(&self, service_id: &str) -> bool {
        let mut services = self.services.write();
        
        for instances in services.values_mut() {
            if let Some(instance) = instances.iter_mut().find(|i| i.id == service_id) {
                instance.healthy = false;
                return true;
            }
        }
        
        false
    }
    
    /// Get registry statistics
    pub fn stats(&self) -> ServiceRegistryStats {
        let services = self.services.read();
        
        let total_instances: usize = services.values().map(|v| v.len()).sum();
        let healthy_instances: usize = services
            .values()
            .flat_map(|v| v.iter())
            .filter(|i| i.healthy)
            .count();
        
        ServiceRegistryStats {
            total_services: services.len(),
            total_instances,
            healthy_instances,
            unhealthy_instances: total_instances - healthy_instances,
        }
    }
}

/// Service registry statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRegistryStats {
    pub total_services: usize,
    pub total_instances: usize,
    pub healthy_instances: usize,
    pub unhealthy_instances: usize,
}

/// Load balancer strategies
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoadBalancerStrategy {
    RoundRobin,
    Random,
    LeastConnections,
    IPHash,
}

/// Load balancer for service instances
pub struct LoadBalancer {
    registry: Arc<ServiceRegistry>,
    strategy: LoadBalancerStrategy,
    /// Counter for round-robin
    counter: Arc<std::sync::atomic::AtomicUsize>,
}

impl LoadBalancer {
    pub fn new(registry: Arc<ServiceRegistry>, strategy: LoadBalancerStrategy) -> Self {
        Self {
            registry,
            strategy,
            counter: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }
    
    /// Get the next instance based on the strategy
    pub fn next(&self, service_name: &str) -> Option<ServiceInstance> {
        let instances = self.registry.discover(service_name);
        
        if instances.is_empty() {
            return None;
        }
        
        match self.strategy {
            LoadBalancerStrategy::RoundRobin => {
                let count = self.counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Some(instances[count % instances.len()].clone())
            }
            LoadBalancerStrategy::Random => {
                use rand::seq::SliceRandom;
                instances.choose(&mut rand::thread_rng()).cloned()
            }
            LoadBalancerStrategy::LeastConnections => {
                // Simple: just return first (real impl would track connections)
                instances.first().cloned()
            }
            LoadBalancerStrategy::IPHash => {
                // Simple: just return first (real impl would hash client IP)
                instances.first().cloned()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_register_and_discover() {
        let registry = ServiceRegistry::default();
        
        let instance = ServiceInstance::new("user-service", "localhost", 8080);
        registry.register(instance);
        
        let services = registry.list_services();
        assert!(services.contains(&"user-service".to_string()));
        
        let discovered = registry.discover("user-service");
        assert_eq!(discovered.len(), 1);
    }
    
    #[test]
    fn test_deregister() {
        let registry = ServiceRegistry::default();
        
        let instance = ServiceInstance::new("user-service", "localhost", 8080);
        let id = registry.register(instance);
        
        assert_eq!(registry.instance_count("user-service"), 1);
        
        registry.deregister(&id);
        assert_eq!(registry.instance_count("user-service"), 0);
    }
    
    #[test]
    fn test_heartbeat() {
        let registry = ServiceRegistry::default();
        
        let instance = ServiceInstance::new("api", "localhost", 8080);
        let id = registry.register(instance);
        
        assert!(registry.heartbeat(&id));
        
        let discovered = registry.discover("api");
        assert!(discovered[0].healthy);
    }
    
    #[test]
    fn test_service_url() {
        let mut instance = ServiceInstance::new("api", "localhost", 8080);
        instance.protocol = "https".to_string();
        
        assert_eq!(instance.url(), "https://localhost:8080");
        assert_eq!(instance.url_path("/users"), "https://localhost:8080/users");
    }
    
    #[test]
    fn test_load_balancer_round_robin() {
        let registry = Arc::new(ServiceRegistry::default());
        
        registry.register(ServiceInstance::new("api", "host1", 8080));
        registry.register(ServiceInstance::new("api", "host2", 8080));
        registry.register(ServiceInstance::new("api", "host3", 8080));
        
        let lb = LoadBalancer::new(registry, LoadBalancerStrategy::RoundRobin);
        
        let instance1 = lb.next("api").unwrap();
        let instance2 = lb.next("api").unwrap();
        let instance3 = lb.next("api").unwrap();
        
        // Should get different instances (round-robin)
        assert_ne!(instance1.host, instance2.host);
        assert_ne!(instance2.host, instance3.host);
    }
    
    #[test]
    fn test_registry_stats() {
        let registry = ServiceRegistry::default();
        
        registry.register(ServiceInstance::new("api", "host1", 8080));
        registry.register(ServiceInstance::new("api", "host2", 8080));
        registry.register(ServiceInstance::new("web", "host3", 3000));
        
        let stats = registry.stats();
        
        assert_eq!(stats.total_services, 2);
        assert_eq!(stats.total_instances, 3);
        assert_eq!(stats.healthy_instances, 3);
    }
}
