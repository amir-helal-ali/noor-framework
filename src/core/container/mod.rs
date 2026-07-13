// ============================================================
// Service Container - حاوية الخدمات
// ============================================================
// Dependency injection container for managing application
// services and their lifecycle (singleton, transient, scoped).
//
// حاوية حقن التبعيات لإدارة خدمات التطبيق.
// ============================================================

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// Service lifetime
/// دورة حياة الخدمة
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceLifetime {
    /// Single instance for the entire application
    Singleton,
    /// New instance every time
    Transient,
    /// One instance per scope/request
    Scoped,
}

/// Service factory function
type ServiceFactory = Arc<dyn Fn(&Container) -> Arc<dyn Any + Send + Sync> + Send + Sync>;

/// Service registration
struct ServiceRegistration {
    factory: ServiceFactory,
    lifetime: ServiceLifetime,
    /// Cached singleton instance
    instance: Option<Arc<dyn Any + Send + Sync>>,
}

/// Dependency injection container
/// حاوية حقن التبعيات
pub struct Container {
    services: Arc<RwLock<HashMap<TypeId, ServiceRegistration>>>,
    /// Aliases for interface -> implementation mapping
    aliases: Arc<RwLock<HashMap<String, TypeId>>>,
}

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}

impl Container {
    pub fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            aliases: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register a singleton service
    pub fn singleton<T: 'static + Send + Sync, F>(&self, factory: F) -> &Self
    where
        F: Fn(&Container) -> T + Send + Sync + 'static,
    {
        self.register::<T, F>(factory, ServiceLifetime::Singleton)
    }
    
    /// Register a transient service (new instance each time)
    pub fn transient<T: 'static + Send + Sync, F>(&self, factory: F) -> &Self
    where
        F: Fn(&Container) -> T + Send + Sync + 'static,
    {
        self.register::<T, F>(factory, ServiceLifetime::Transient)
    }
    
    /// Register a scoped service
    pub fn scoped<T: 'static + Send + Sync, F>(&self, factory: F) -> &Self
    where
        F: Fn(&Container) -> T + Send + Sync + 'static,
    {
        self.register::<T, F>(factory, ServiceLifetime::Scoped)
    }
    
    /// Register a service with a specific lifetime
    fn register<T: 'static + Send + Sync, F>(&self, factory: F, lifetime: ServiceLifetime) -> &Self
    where
        F: Fn(&Container) -> T + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        
        let factory_arc: ServiceFactory = Arc::new(move |c| {
            Arc::new(factory(c)) as Arc<dyn Any + Send + Sync>
        });
        
        self.services.write().insert(type_id, ServiceRegistration {
            factory: factory_arc,
            lifetime,
            instance: None,
        });
        
        self
    }
    
    /// Register an instance directly (singleton)
    pub fn instance<T: 'static + Send + Sync>(&self, instance: T) -> &Self {
        let type_id = TypeId::of::<T>();
        let instance_arc = Arc::new(instance) as Arc<dyn Any + Send + Sync>;
        
        self.services.write().insert(type_id, ServiceRegistration {
            factory: Arc::new(move |_| {
                // This won't be called for singletons with instance
                Arc::new(()) as Arc<dyn Any + Send + Sync>
            }),
            lifetime: ServiceLifetime::Singleton,
            instance: Some(instance_arc),
        });
        
        self
    }
    
    /// Register an alias for a service
    pub fn alias<T: 'static>(&self, name: &str) -> &Self {
        self.aliases.write().insert(name.to_string(), TypeId::of::<T>());
        self
    }
    
    /// Resolve a service
    ///
    /// Note: we deliberately do NOT hold the `services` write lock while
    /// invoking the factory, because the factory may itself call
    /// `resolve()` (dependency injection). parking_lot::RwLock is not
    /// reentrant, so holding the lock would deadlock.
    pub fn resolve<T: 'static + Send + Sync>(&self) -> Option<Arc<T>> {
        let type_id = TypeId::of::<T>();

        // Fast path: singleton already instantiated.
        {
            let services = self.services.read();
            if let Some(registration) = services.get(&type_id) {
                if registration.lifetime == ServiceLifetime::Singleton {
                    if let Some(ref instance) = registration.instance {
                        return Arc::downcast::<T>(instance.clone()).ok();
                    }
                }
            }
        }

        // We need to create an instance. Clone the factory out so we can
        // release the lock before invoking it (prevents reentrant deadlock).
        let factory = {
            let services = self.services.read();
            services.get(&type_id).map(|r| r.factory.clone())?
        };

        let instance = factory(self);

        // Cache singleton instance.
        if {
            let services = self.services.read();
            services
                .get(&type_id)
                .map(|r| r.lifetime)
                .unwrap_or(ServiceLifetime::Transient)
        } == ServiceLifetime::Singleton
        {
            let mut services = self.services.write();
            if let Some(registration) = services.get_mut(&type_id) {
                if registration.instance.is_none() {
                    registration.instance = Some(instance.clone());
                } else if let Some(ref existing) = registration.instance {
                    // Another thread beat us; prefer the cached instance.
                    return Arc::downcast::<T>(existing.clone()).ok();
                }
            }
        }

        Arc::downcast::<T>(instance).ok()
    }
    
    /// Resolve a service or panic
    pub fn expect<T: 'static + Send + Sync>(&self) -> Arc<T> {
        self.resolve::<T>().expect(&format!(
            "Service {} not registered",
            std::any::type_name::<T>()
        ))
    }
    
    /// Check if a service is registered
    pub fn is_registered<T: 'static>(&self) -> bool {
        self.services.read().contains_key(&TypeId::of::<T>())
    }
    
    /// Get all registered service type names
    pub fn registered_services(&self) -> Vec<String> {
        self.services
            .read()
            .keys()
            .map(|_| "service".to_string())
            .collect()
    }
    
    /// Get the number of registered services
    pub fn count(&self) -> usize {
        self.services.read().len()
    }
    
    /// Create a scoped container (for request-scoped services)
    pub fn create_scope(&self) -> ScopedContainer {
        ScopedContainer {
            parent: self,
            scoped_instances: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

/// Scoped container for request-scoped services
pub struct ScopedContainer<'a> {
    parent: &'a Container,
    scoped_instances: Arc<RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>>,
}

impl<'a> ScopedContainer<'a> {
    /// Resolve a service in this scope
    pub fn resolve<T: 'static + Send + Sync>(&self) -> Option<Arc<T>> {
        let type_id = TypeId::of::<T>();

        // Check scoped instances first.
        {
            let scoped = self.scoped_instances.read();
            if let Some(instance) = scoped.get(&type_id) {
                return Arc::downcast::<T>(instance.clone()).ok();
            }
        }

        // Peek at the registration to decide lifetime, without holding the
        // lock during factory invocation (would deadlock on reentry).
        let (factory, lifetime) = {
            let services = self.parent.services.read();
            services
                .get(&type_id)
                .map(|r| (r.factory.clone(), r.lifetime))?
        };

        match lifetime {
            ServiceLifetime::Scoped => {
                let instance = (factory)(self.parent);
                self.scoped_instances.write().insert(type_id, instance.clone());
                Arc::downcast::<T>(instance).ok()
            }
            _ => self.parent.resolve::<T>(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    struct Database {
        name: String,
    }
    
    struct UserRepository {
        db: Arc<Database>,
    }
    
    struct EmailService;
    
    #[test]
    fn test_singleton() {
        let container = Container::new();
        
        container.singleton(|_| Database { name: "noor_db".to_string() });
        
        let db1 = container.resolve::<Database>().unwrap();
        let db2 = container.resolve::<Database>().unwrap();
        
        // Should be the same instance
        assert!(Arc::ptr_eq(&db1, &db2));
    }
    
    #[test]
    fn test_transient() {
        let container = Container::new();
        
        container.transient(|_| EmailService);
        
        let svc1 = container.resolve::<EmailService>().unwrap();
        let svc2 = container.resolve::<EmailService>().unwrap();
        
        // Should be different instances
        assert!(!Arc::ptr_eq(&svc1, &svc2));
    }
    
    #[test]
    fn test_dependency_injection() {
        let container = Container::new();
        
        container.singleton(|_| Database { name: "noor_db".to_string() });
        
        container.singleton(|c| UserRepository {
            db: c.expect::<Database>(),
        });
        
        let repo = container.resolve::<UserRepository>().unwrap();
        assert_eq!(repo.db.name, "noor_db");
    }
    
    #[test]
    fn test_instance_registration() {
        let container = Container::new();
        
        let db = Database { name: "custom_db".to_string() };
        container.instance(db);
        
        let resolved = container.resolve::<Database>().unwrap();
        assert_eq!(resolved.name, "custom_db");
    }
    
    #[test]
    fn test_is_registered() {
        let container = Container::new();
        
        assert!(!container.is_registered::<Database>());
        
        container.singleton(|_| Database { name: "test".to_string() });
        
        assert!(container.is_registered::<Database>());
    }
    
    #[test]
    fn test_scoped_container() {
        let container = Container::new();
        
        container.scoped(|_| EmailService);
        
        {
            let scope1 = container.create_scope();
            let svc1 = scope1.resolve::<EmailService>().unwrap();
            let svc1_again = scope1.resolve::<EmailService>().unwrap();
            
            // Same scope should return same instance
            assert!(Arc::ptr_eq(&svc1, &svc1_again));
        }
    }
}
