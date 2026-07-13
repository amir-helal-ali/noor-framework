// ============================================================
// Plugin System - نظام الإضافات
// ============================================================
// Extensible plugin architecture that allows third-party
// extensions to hook into the framework's lifecycle.
//
// بنية إضافات قابلة للتوسعة تسمح بتوصيلات الطرف الثالث.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// Plugin metadata
/// معلومات الإضافة
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub homepage: Option<String>,
    pub license: String,
}

/// Plugin trait that all plugins must implement
/// Trait يجب على جميع الإضافات تطبيقه
pub trait Plugin: Send + Sync {
    /// Get plugin information
    fn info(&self) -> &PluginInfo;
    
    /// Called when the plugin is registered
    fn on_register(&self) -> crate::NoorResult<()> {
        Ok(())
    }
    
    /// Called when the plugin is booted (before serving requests)
    fn on_boot(&self) -> crate::NoorResult<()> {
        Ok(())
    }
    
    /// Called when the application is shutting down
    fn on_shutdown(&self) -> crate::NoorResult<()> {
        Ok(())
    }
    
    /// Called before a request is processed
    fn on_request(&self, _request: &crate::core::http::Request) -> crate::NoorResult<()> {
        Ok(())
    }
    
    /// Called after a response is generated
    fn on_response(&self, _request: &crate::core::http::Request, _response: &mut crate::core::http::Response) -> crate::NoorResult<()> {
        Ok(())
    }
}

/// Hook types for plugin lifecycle
/// أنواع الـ hooks لدورة حياة الإضافة
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Hook {
    Register,
    Boot,
    Shutdown,
    BeforeRequest,
    AfterRequest,
    BeforeResponse,
    AfterResponse,
    OnError,
}

/// Hook handler function type
type HookHandler = Arc<dyn Fn() -> crate::NoorResult<()> + Send + Sync>;

/// Plugin manager
/// مدير الإضافات
pub struct PluginManager {
    plugins: Arc<RwLock<Vec<Arc<dyn Plugin>>>>,
    hooks: Arc<RwLock<HashMap<Hook, Vec<HookHandler>>>>,
    enabled: Arc<RwLock<HashMap<String, bool>>>,
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(Vec::new())),
            hooks: Arc::new(RwLock::new(HashMap::new())),
            enabled: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register a plugin
    /// تسجيل إضافة
    pub fn register(&self, plugin: Arc<dyn Plugin>) -> crate::NoorResult<()> {
        let info = plugin.info().clone();
        tracing::info!("Registering plugin: {} v{}", info.name, info.version);
        
        plugin.on_register()?;
        
        self.plugins.write().push(plugin);
        self.enabled.write().insert(info.name, true);
        
        Ok(())
    }
    
    /// Enable a plugin by name
    pub fn enable(&self, name: &str) {
        self.enabled.write().insert(name.to_string(), true);
    }
    
    /// Disable a plugin by name
    pub fn disable(&self, name: &str) {
        self.enabled.write().insert(name.to_string(), false);
    }
    
    /// Check if a plugin is enabled
    pub fn is_enabled(&self, name: &str) -> bool {
        *self.enabled.read().get(name).unwrap_or(&false)
    }
    
    /// Boot all registered plugins
    pub fn boot_all(&self) -> crate::NoorResult<()> {
        let plugins = self.plugins.read().clone();
        for plugin in &plugins {
            let info = plugin.info();
            if self.is_enabled(&info.name) {
                plugin.on_boot()?;
                tracing::info!("Plugin '{}' booted", info.name);
            }
        }
        Ok(())
    }
    
    /// Shutdown all registered plugins
    pub fn shutdown_all(&self) -> crate::NoorResult<()> {
        let plugins = self.plugins.read().clone();
        for plugin in plugins.iter().rev() {
            let info = plugin.info();
            if self.is_enabled(&info.name) {
                plugin.on_shutdown()?;
                tracing::info!("Plugin '{}' shut down", info.name);
            }
        }
        Ok(())
    }
    
    /// Run before-request hooks for all plugins
    pub fn before_request(&self, request: &crate::core::http::Request) -> crate::NoorResult<()> {
        let plugins = self.plugins.read().clone();
        for plugin in &plugins {
            let info = plugin.info();
            if self.is_enabled(&info.name) {
                plugin.on_request(request)?;
            }
        }
        Ok(())
    }
    
    /// Run after-response hooks for all plugins
    pub fn after_response(
        &self,
        request: &crate::core::http::Request,
        response: &mut crate::core::http::Response,
    ) -> crate::NoorResult<()> {
        let plugins = self.plugins.read().clone();
        for plugin in &plugins {
            let info = plugin.info();
            if self.is_enabled(&info.name) {
                plugin.on_response(request, response)?;
            }
        }
        Ok(())
    }
    
    /// Register a custom hook handler
    pub fn on(&self, hook: Hook, handler: HookHandler) {
        self.hooks
            .write()
            .entry(hook)
            .or_insert_with(Vec::new)
            .push(handler);
    }
    
    /// Execute all handlers for a hook
    pub fn trigger(&self, hook: Hook) -> crate::NoorResult<()> {
        let hooks = self.hooks.read();
        if let Some(handlers) = hooks.get(&hook) {
            for handler in handlers {
                handler()?;
            }
        }
        Ok(())
    }
    
    /// List all registered plugins
    pub fn list(&self) -> Vec<PluginInfo> {
        self.plugins
            .read()
            .iter()
            .map(|p| p.info().clone())
            .collect()
    }
    
    /// Get the number of registered plugins
    pub fn count(&self) -> usize {
        self.plugins.read().len()
    }
}

/// Built-in debug plugin (logs all requests)
/// إضافة التصحيح المدمجة (تسجل جميع الطلبات)
pub struct DebugPlugin {
    info: PluginInfo,
}

impl DebugPlugin {
    pub fn new() -> Self {
        Self {
            info: PluginInfo {
                name: "debug".to_string(),
                version: "1.0.0".to_string(),
                description: "Debug plugin that logs all requests".to_string(),
                author: "Noor Framework".to_string(),
                homepage: None,
                license: "MIT".to_string(),
            },
        }
    }
}

impl Default for DebugPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for DebugPlugin {
    fn info(&self) -> &PluginInfo {
        &self.info
    }
    
    fn on_request(&self, request: &crate::core::http::Request) -> crate::NoorResult<()> {
        tracing::debug!(
            plugin = "debug",
            method = %request.method,
            path = %request.path,
            "Debug plugin: request received"
        );
        Ok(())
    }
}

/// Built-in stats plugin (tracks request counts)
pub struct StatsPlugin {
    info: PluginInfo,
    request_count: Arc<std::sync::atomic::AtomicU64>,
}

impl StatsPlugin {
    pub fn new() -> Self {
        Self {
            info: PluginInfo {
                name: "stats".to_string(),
                version: "1.0.0".to_string(),
                description: "Tracks request statistics".to_string(),
                author: "Noor Framework".to_string(),
                homepage: None,
                license: "MIT".to_string(),
            },
            request_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }
    
    pub fn request_count(&self) -> u64 {
        self.request_count.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl Default for StatsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for StatsPlugin {
    fn info(&self) -> &PluginInfo {
        &self.info
    }
    
    fn on_request(&self, _request: &crate::core::http::Request) -> crate::NoorResult<()> {
        self.request_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_plugin_manager() {
        let manager = PluginManager::new();
        let plugin = Arc::new(DebugPlugin::new());
        
        manager.register(plugin).unwrap();
        manager.boot_all().unwrap();
        
        assert_eq!(manager.count(), 1);
        
        let plugins = manager.list();
        assert_eq!(plugins[0].name, "debug");
    }
    
    #[test]
    fn test_stats_plugin() {
        let manager = PluginManager::new();
        let stats = Arc::new(StatsPlugin::new());
        
        manager.register(stats.clone()).unwrap();
        manager.boot_all().unwrap();
        
        let request = crate::core::http::Request::new(
            crate::core::http::Method::Get,
            "/".to_string(),
        );
        
        manager.before_request(&request).unwrap();
        manager.before_request(&request).unwrap();
        
        assert_eq!(stats.request_count(), 2);
    }
}
