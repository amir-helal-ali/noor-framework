// ============================================================
// Events Module - نظام الأحداث
// ============================================================
// Event-driven architecture with publishers and subscribers.
// Supports sync and async event handlers.
//
// بنية مدفوعة بالأحداث مع ناشرين ومشتركين.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// An event that can be dispatched
/// حدث يمكن توزيعه
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub name: String,
    pub payload: serde_json::Value,
    pub timestamp: i64,
}

impl Event {
    pub fn new(name: &str, payload: serde_json::Value) -> Self {
        Self {
            name: name.to_string(),
            payload,
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }
}

/// Event handler type
/// نوع معالج الحدث
pub type EventHandler = Arc<dyn Fn(&Event) -> crate::NoorResult<()> + Send + Sync>;

/// Event dispatcher (publish-subscribe pattern)
/// موزع الأحداث (نموذج publish-subscribe)
pub struct EventEmitter {
    handlers: Arc<RwLock<HashMap<String, Vec<EventHandler>>>>,
    /// Wildcard handlers (receive all events)
    wildcard_handlers: Arc<RwLock<Vec<EventHandler>>>,
}

impl Default for EventEmitter {
    fn default() -> Self {
        Self::new()
    }
}

impl EventEmitter {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
            wildcard_handlers: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Subscribe to an event
    /// الاشتراك في حدث
    pub fn on(&self, event_name: &str, handler: EventHandler) -> &Self {
        self.handlers
            .write()
            .entry(event_name.to_string())
            .or_insert_with(Vec::new)
            .push(handler);
        self
    }
    
    /// Subscribe to all events (wildcard)
    /// الاشتراك في جميع الأحداث
    pub fn on_any(&self, handler: EventHandler) -> &Self {
        self.wildcard_handlers.write().push(handler);
        self
    }
    
    /// Dispatch an event to all subscribers
    /// توزيع حدث لجميع المشتركين
    pub fn emit(&self, event: &Event) -> crate::NoorResult<()> {
        // Wildcard handlers
        let wildcard_handlers = self.wildcard_handlers.read();
        for handler in wildcard_handlers.iter() {
            if let Err(e) = handler(event) {
                tracing::error!("Wildcard event handler error: {}", e);
            }
        }
        drop(wildcard_handlers);
        
        // Specific handlers
        let handlers = self.handlers.read();
        if let Some(event_handlers) = handlers.get(&event.name) {
            for handler in event_handlers {
                if let Err(e) = handler(event) {
                    tracing::error!("Event handler error for '{}': {}", event.name, e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Create and dispatch an event in one call
    /// إنشاء وتوزيع حدث في استدعاء واحد
    pub fn fire(&self, name: &str, payload: serde_json::Value) -> crate::NoorResult<()> {
        let event = Event::new(name, payload);
        self.emit(&event)
    }
    
    /// Get the number of subscribers for an event
    /// عدد المشتركين في حدث
    pub fn subscriber_count(&self, event_name: &str) -> usize {
        self.handlers
            .read()
            .get(event_name)
            .map(|v| v.len())
            .unwrap_or(0)
    }
    
    /// Remove all subscribers for an event
    /// إزالة جميع المشتركين في حدث
    pub fn clear(&self, event_name: &str) {
        self.handlers.write().remove(event_name);
    }
    
    /// Remove all subscribers for all events
    /// إزالة جميع المشتركين
    pub fn clear_all(&self) {
        self.handlers.write().clear();
        self.wildcard_handlers.write().clear();
    }
}

/// Built-in event names
/// أسماء الأحداث المدمجة
pub mod events {
    pub const USER_REGISTERED: &str = "user.registered";
    pub const USER_LOGIN: &str = "user.login";
    pub const USER_LOGOUT: &str = "user.logout";
    pub const POST_CREATED: &str = "post.created";
    pub const POST_UPDATED: &str = "post.updated";
    pub const POST_DELETED: &str = "post.deleted";
    pub const EMAIL_SEND: &str = "email.send";
    pub const CACHE_CLEARED: &str = "cache.cleared";
    pub const DATABASE_MIGRATED: &str = "database.migrated";
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    #[test]
    fn test_event_emitter() {
        let emitter = EventEmitter::new();
        let counter = Arc::new(AtomicUsize::new(0));
        
        let counter_clone = counter.clone();
        emitter.on("test.event", Arc::new(move |_event| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }));
        
        emitter.fire("test.event", serde_json::json!({"key": "value"})).unwrap();
        emitter.fire("other.event", serde_json::json!({})).unwrap();
        
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
    
    #[test]
    fn test_wildcard_handler() {
        let emitter = EventEmitter::new();
        let counter = Arc::new(AtomicUsize::new(0));
        
        let counter_clone = counter.clone();
        emitter.on_any(Arc::new(move |_event| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }));
        
        emitter.fire("event1", serde_json::json!({})).unwrap();
        emitter.fire("event2", serde_json::json!({})).unwrap();
        emitter.fire("event3", serde_json::json!({})).unwrap();
        
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }
}
