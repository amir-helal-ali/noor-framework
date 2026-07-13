// ============================================================
// Observer Pattern - نمط المراقب للنماذج
// ============================================================
// Listen to model lifecycle events (creating, created, updating,
// updated, deleting, deleted, etc.)
//
// الاستماع لأحداث دورة حياة النموذج.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::Serialize;

/// Model events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModelEvent {
    Creating,
    Created,
    Updating,
    Updated,
    Saving,
    Saved,
    Deleting,
    Deleted,
    Restoring,
    Restored,
    ForceDeleting,
    ForceDeleted,
}

/// Observer trait
pub trait Observer: Send + Sync {
    /// Called before a model is created
    fn on_creating(&self, _model: &mut serde_json::Value) -> bool {
        true // Return false to cancel
    }
    
    /// Called after a model is created
    fn on_created(&self, _model: &serde_json::Value) {}
    
    /// Called before a model is updated
    fn on_updating(&self, _model: &mut serde_json::Value) -> bool {
        true
    }
    
    /// Called after a model is updated
    fn on_updated(&self, _model: &serde_json::Value) {}
    
    /// Called before a model is saved (create or update)
    fn on_saving(&self, _model: &mut serde_json::Value) -> bool {
        true
    }
    
    /// Called after a model is saved
    fn on_saved(&self, _model: &serde_json::Value) {}
    
    /// Called before a model is deleted
    fn on_deleting(&self, _model: &serde_json::Value) -> bool {
        true
    }
    
    /// Called after a model is deleted
    fn on_deleted(&self, _model: &serde_json::Value) {}
}

/// Observer manager for a model type
pub struct ObserverManager {
    observers: Arc<RwLock<Vec<Arc<dyn Observer>>>>,
}

impl Default for ObserverManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ObserverManager {
    pub fn new() -> Self {
        Self {
            observers: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Register an observer
    pub fn register(&self, observer: Arc<dyn Observer>) -> &Self {
        self.observers.write().push(observer);
        self
    }
    
    /// Fire the creating event
    pub fn fire_creating(&self, model: &mut serde_json::Value) -> bool {
        let observers = self.observers.read().clone();
        for observer in &observers {
            if !observer.on_creating(model) {
                return false; // Cancel
            }
        }
        true
    }
    
    /// Fire the created event
    pub fn fire_created(&self, model: &serde_json::Value) {
        let observers = self.observers.read().clone();
        for observer in &observers {
            observer.on_created(model);
        }
    }
    
    /// Fire the updating event
    pub fn fire_updating(&self, model: &mut serde_json::Value) -> bool {
        let observers = self.observers.read().clone();
        for observer in &observers {
            if !observer.on_updating(model) {
                return false;
            }
        }
        true
    }
    
    /// Fire the updated event
    pub fn fire_updated(&self, model: &serde_json::Value) {
        let observers = self.observers.read().clone();
        for observer in &observers {
            observer.on_updated(model);
        }
    }
    
    /// Fire the saving event
    pub fn fire_saving(&self, model: &mut serde_json::Value) -> bool {
        let observers = self.observers.read().clone();
        for observer in &observers {
            if !observer.on_saving(model) {
                return false;
            }
        }
        true
    }
    
    /// Fire the saved event
    pub fn fire_saved(&self, model: &serde_json::Value) {
        let observers = self.observers.read().clone();
        for observer in &observers {
            observer.on_saved(model);
        }
    }
    
    /// Fire the deleting event
    pub fn fire_deleting(&self, model: &serde_json::Value) -> bool {
        let observers = self.observers.read().clone();
        for observer in &observers {
            if !observer.on_deleting(model) {
                return false;
            }
        }
        true
    }
    
    /// Fire the deleted event
    pub fn fire_deleted(&self, model: &serde_json::Value) {
        let observers = self.observers.read().clone();
        for observer in &observers {
            observer.on_deleted(model);
        }
    }
    
    /// Get the count of registered observers
    pub fn count(&self) -> usize {
        self.observers.read().len()
    }
    
    /// Clear all observers
    pub fn clear(&self) {
        self.observers.write().clear();
    }
}

/// Global observer registry (one manager per model type)
pub struct ObserverRegistry {
    managers: Arc<RwLock<HashMap<String, Arc<ObserverManager>>>>,
}

impl Default for ObserverRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ObserverRegistry {
    pub fn new() -> Self {
        Self {
            managers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Get or create an observer manager for a model type
    pub fn for_model(&self, model_name: &str) -> Arc<ObserverManager> {
        let mut managers = self.managers.write();
        
        managers
            .entry(model_name.to_string())
            .or_insert_with(|| Arc::new(ObserverManager::new()))
            .clone()
    }
    
    /// Register an observer for a model type
    pub fn observe(&self, model_name: &str, observer: Arc<dyn Observer>) {
        self.for_model(model_name).register(observer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    struct LoggingObserver {
        call_count: Arc<AtomicUsize>,
    }
    
    impl Observer for LoggingObserver {
        fn on_creating(&self, _model: &mut serde_json::Value) -> bool {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            true
        }
        
        fn on_created(&self, _model: &serde_json::Value) {
            self.call_count.fetch_add(1, Ordering::SeqCst);
        }
        
        fn on_deleting(&self, _model: &serde_json::Value) -> bool {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            true
        }
    }
    
    struct CancelingObserver;
    
    impl Observer for CancelingObserver {
        fn on_creating(&self, _model: &mut serde_json::Value) -> bool {
            false // Cancel the operation
        }
    }
    
    #[test]
    fn test_observer_events() {
        let manager = ObserverManager::new();
        let counter = Arc::new(AtomicUsize::new(0));
        
        manager.register(Arc::new(LoggingObserver { call_count: counter.clone() }));
        
        let mut model = serde_json::json!({"name": "Test"});
        
        // Fire creating -> should increment counter
        assert!(manager.fire_creating(&mut model));
        
        // Fire created -> should increment counter
        manager.fire_created(&model);
        
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }
    
    #[test]
    fn test_observer_cancellation() {
        let manager = ObserverManager::new();
        manager.register(Arc::new(CancelingObserver));
        
        let mut model = serde_json::json!({"name": "Test"});
        
        // Creating should be cancelled
        assert!(!manager.fire_creating(&mut model));
    }
    
    #[test]
    fn test_observer_registry() {
        let registry = ObserverRegistry::new();
        
        let counter = Arc::new(AtomicUsize::new(0));
        let observer = Arc::new(LoggingObserver { call_count: counter.clone() });
        
        registry.observe("User", observer);
        
        let user_manager = registry.for_model("User");
        assert_eq!(user_manager.count(), 1);
        
        // Different model should have its own manager
        let post_manager = registry.for_model("Post");
        assert_eq!(post_manager.count(), 0);
    }
}
