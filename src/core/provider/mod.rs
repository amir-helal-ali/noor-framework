// ============================================================
// Service Provider Pattern - نمط مزود الخدمة
// ============================================================
// Laravel-style service providers for bootstrapping services.
// Providers register dependencies and boot application services.
//
// مزودو الخدمات لإقلاع خدمات التطبيق.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// Service provider trait
pub trait ServiceProvider: Send + Sync {
    /// Get the provider name
    fn name(&self) -> &str;
    
    /// Register services in the container
    fn register(&self, container: &crate::core::container::Container) -> crate::NoorResult<()> {
        let _ = container;
        Ok(())
    }
    
    /// Boot the service (after all providers are registered)
    fn boot(&self, container: &crate::core::container::Container) -> crate::NoorResult<()> {
        let _ = container;
        Ok(())
    }
    
    /// Get provider dependencies (must be registered before this provider)
    fn dependencies(&self) -> Vec<String> {
        vec![]
    }
}

/// Provider manager
pub struct ProviderManager {
    providers: Arc<RwLock<Vec<Arc<dyn ServiceProvider>>>>,
    booted: Arc<RwLock<bool>>,
}

impl Default for ProviderManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderManager {
    pub fn new() -> Self {
        Self {
            providers: Arc::new(RwLock::new(Vec::new())),
            booted: Arc::new(RwLock::new(false)),
        }
    }
    
    /// Register a service provider
    pub fn register(&self, provider: Arc<dyn ServiceProvider>) -> &Self {
        tracing::info!("Registering service provider: {}", provider.name());
        self.providers.write().push(provider);
        self
    }
    
    /// Call register() on all providers
    pub fn register_all(&self, container: &crate::core::container::Container) -> crate::NoorResult<()> {
        let providers = self.providers.read().clone();
        for provider in &providers {
            provider.register(container)?;
        }
        Ok(())
    }
    
    /// Call boot() on all providers (after all are registered)
    pub fn boot_all(&self, container: &crate::core::container::Container) -> crate::NoorResult<()> {
        if *self.booted.read() {
            return Ok(());
        }
        
        let providers = self.providers.read().clone();
        for provider in &providers {
            tracing::info!("Booting service provider: {}", provider.name());
            provider.boot(container)?;
        }
        
        *self.booted.write() = true;
        Ok(())
    }
    
    /// Get all registered providers
    pub fn list(&self) -> Vec<String> {
        self.providers.read().iter().map(|p| p.name().to_string()).collect()
    }
    
    /// Check if a provider is registered
    pub fn has(&self, name: &str) -> bool {
        self.providers.read().iter().any(|p| p.name() == name)
    }
    
    /// Get the count of registered providers
    pub fn count(&self) -> usize {
        self.providers.read().len()
    }
}

/// Built-in providers
pub mod providers {
    use super::*;
    use crate::core::container::Container;
    
    /// Database service provider.
    ///
    /// NOTE: The synchronous `Container` cannot hold an async-constructed
    /// `Database` directly (sqlx pools must be `await`-ed into existence).
    /// Production code should construct the `Database` in an async context
    /// (e.g. `Application::boot`) and register the resulting pool with
    /// `Container::instance(...)`. This provider is kept as a placeholder
    /// so existing module wiring keeps compiling.
    pub struct DatabaseProvider;

    impl ServiceProvider for DatabaseProvider {
        fn name(&self) -> &str { "database" }

        fn register(&self, _container: &Container) -> crate::NoorResult<()> {
            // No-op: real Database registration must happen in async context.
            tracing::debug!("DatabaseProvider::register (deferred to async boot)");
            Ok(())
        }

        fn boot(&self, _container: &Container) -> crate::NoorResult<()> {
            tracing::info!("Database provider booted");
            Ok(())
        }
    }
    
    /// Cache service provider
    pub struct CacheProvider;
    
    impl ServiceProvider for CacheProvider {
        fn name(&self) -> &str { "cache" }
        
        fn register(&self, container: &Container) -> crate::NoorResult<()> {
            container.singleton(|_| {
                crate::core::cache::CacheManager::for_weak_server("storage/cache")
                    .unwrap_or_else(|_| crate::core::cache::CacheManager::memory_only(1000))
            });
            Ok(())
        }
    }
    
    /// Auth service provider
    pub struct AuthProvider;
    
    impl ServiceProvider for AuthProvider {
        fn name(&self) -> &str { "auth" }
        
        fn register(&self, container: &Container) -> crate::NoorResult<()> {
            container.singleton(|_| {
                crate::core::auth::Jwt::new("secret", "noor", "noor_app")
            });
            container.singleton(|_| {
                crate::core::auth::Rbac::new()
            });
            Ok(())
        }
    }
    
    /// View service provider
    pub struct ViewProvider;
    
    impl ServiceProvider for ViewProvider {
        fn name(&self) -> &str { "view" }
        
        fn register(&self, container: &Container) -> crate::NoorResult<()> {
            container.singleton(|_| {
                crate::core::view::ViewEngine::new("resources/views", true, true)
                    .unwrap_or_else(|_| {
                        // Return a stub if views dir doesn't exist
                        crate::core::view::ViewEngine::new("resources/views", false, false)
                            .unwrap()
                    })
            });
            Ok(())
        }
    }
    
    /// Mail service provider
    pub struct MailProvider;
    
    impl ServiceProvider for MailProvider {
        fn name(&self) -> &str { "mail" }
        
        fn register(&self, container: &Container) -> crate::NoorResult<()> {
            container.singleton(|_| {
                crate::core::mail::Mailer::new(crate::core::mail::MailConfig::default())
            });
            Ok(())
        }
    }
    
    /// Queue service provider
    pub struct QueueProvider;
    
    impl ServiceProvider for QueueProvider {
        fn name(&self) -> &str { "queue" }
        
        fn register(&self, container: &Container) -> crate::NoorResult<()> {
            container.singleton(|_| {
                crate::core::queue::Queue::new()
            });
            Ok(())
        }
    }
    
    /// Events service provider
    pub struct EventProvider;
    
    impl ServiceProvider for EventProvider {
        fn name(&self) -> &str { "events" }
        
        fn register(&self, container: &Container) -> crate::NoorResult<()> {
            container.singleton(|_| {
                crate::core::events::EventEmitter::new()
            });
            Ok(())
        }
    }
    
    /// Notification service provider
    pub struct NotificationProvider;
    
    impl ServiceProvider for NotificationProvider {
        fn name(&self) -> &str { "notification" }
        
        fn register(&self, container: &Container) -> crate::NoorResult<()> {
            container.singleton(|_| {
                crate::core::notification::NotificationManager::new()
            });
            Ok(())
        }
    }
    
    /// Register all built-in providers
    pub fn register_all(manager: &ProviderManager) {
        manager.register(Arc::new(DatabaseProvider));
        manager.register(Arc::new(CacheProvider));
        manager.register(Arc::new(AuthProvider));
        manager.register(Arc::new(ViewProvider));
        manager.register(Arc::new(MailProvider));
        manager.register(Arc::new(QueueProvider));
        manager.register(Arc::new(EventProvider));
        manager.register(Arc::new(NotificationProvider));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    struct TestProvider;
    
    impl ServiceProvider for TestProvider {
        fn name(&self) -> &str { "test" }
    }
    
    #[test]
    fn test_provider_manager() {
        let manager = ProviderManager::new();
        manager.register(Arc::new(TestProvider));
        
        assert_eq!(manager.count(), 1);
        assert!(manager.has("test"));
        assert!(!manager.has("other"));
    }
    
    #[test]
    fn test_builtin_providers() {
        let manager = ProviderManager::new();
        providers::register_all(&manager);
        
        assert!(manager.count() >= 8);
        assert!(manager.has("database"));
        assert!(manager.has("cache"));
        assert!(manager.has("auth"));
    }
}
