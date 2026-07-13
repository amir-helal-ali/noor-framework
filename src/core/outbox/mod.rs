// ============================================================
// Outbox Pattern - نمط صندوق الصادر
// ============================================================
// Ensures database updates and event publication happen
// atomically. Events are stored in an outbox table and
// published asynchronously.
//
// يضمن تحديث قاعدة البيانات ونشر الأحداث بشكل ذري.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// Outbox message status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutboxStatus {
    Pending,
    Processing,
    Published,
    Failed,
}

/// Outbox message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxMessage {
    pub id: String,
    pub aggregate_id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub headers: HashMap<String, String>,
    pub status: OutboxStatus,
    pub attempts: u32,
    pub max_attempts: u32,
    pub created_at: i64,
    pub processed_at: Option<i64>,
    pub last_error: Option<String>,
}

impl OutboxMessage {
    pub fn new(aggregate_id: &str, event_type: &str, payload: serde_json::Value) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            aggregate_id: aggregate_id.to_string(),
            event_type: event_type.to_string(),
            payload,
            headers: HashMap::new(),
            status: OutboxStatus::Pending,
            attempts: 0,
            max_attempts: 3,
            created_at: chrono::Utc::now().timestamp(),
            processed_at: None,
            last_error: None,
        }
    }
    
    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }
    
    pub fn with_max_attempts(mut self, max: u32) -> Self {
        self.max_attempts = max;
        self
    }
}

/// Outbox store
pub trait OutboxStore: Send + Sync {
    fn save(&self, message: OutboxMessage) -> crate::NoorResult<()>;
    fn get_pending(&self, limit: usize) -> Vec<OutboxMessage>;
    fn mark_processing(&self, id: &str) -> bool;
    fn mark_published(&self, id: &str) -> bool;
    fn mark_failed(&self, id: &str, error: &str) -> bool;
    fn get(&self, id: &str) -> Option<OutboxMessage>;
    fn count_by_status(&self, status: OutboxStatus) -> usize;
}

/// In-memory outbox store
pub struct InMemoryOutboxStore {
    messages: Arc<RwLock<HashMap<String, OutboxMessage>>>,
}

impl Default for InMemoryOutboxStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryOutboxStore {
    pub fn new() -> Self {
        Self {
            messages: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl OutboxStore for InMemoryOutboxStore {
    fn save(&self, message: OutboxMessage) -> crate::NoorResult<()> {
        self.messages.write().insert(message.id.clone(), message);
        Ok(())
    }
    
    fn get_pending(&self, limit: usize) -> Vec<OutboxMessage> {
        self.messages
            .read()
            .values()
            .filter(|m| m.status == OutboxStatus::Pending)
            .take(limit)
            .cloned()
            .collect()
    }
    
    fn mark_processing(&self, id: &str) -> bool {
        if let Some(msg) = self.messages.write().get_mut(id) {
            msg.status = OutboxStatus::Processing;
            msg.attempts += 1;
            return true;
        }
        false
    }
    
    fn mark_published(&self, id: &str) -> bool {
        if let Some(msg) = self.messages.write().get_mut(id) {
            msg.status = OutboxStatus::Published;
            msg.processed_at = Some(chrono::Utc::now().timestamp());
            return true;
        }
        false
    }
    
    fn mark_failed(&self, id: &str, error: &str) -> bool {
        if let Some(msg) = self.messages.write().get_mut(id) {
            msg.status = if msg.attempts >= msg.max_attempts {
                OutboxStatus::Failed
            } else {
                OutboxStatus::Pending
            };
            msg.last_error = Some(error.to_string());
            return true;
        }
        false
    }
    
    fn get(&self, id: &str) -> Option<OutboxMessage> {
        self.messages.read().get(id).cloned()
    }
    
    fn count_by_status(&self, status: OutboxStatus) -> usize {
        self.messages
            .read()
            .values()
            .filter(|m| m.status == status)
            .count()
    }
}

/// Outbox processor
pub struct OutboxProcessor {
    store: Arc<dyn OutboxStore>,
    batch_size: usize,
    poll_interval_ms: u64,
}

impl OutboxProcessor {
    pub fn new(store: Arc<dyn OutboxStore>) -> Self {
        Self {
            store,
            batch_size: 100,
            poll_interval_ms: 1000,
        }
    }
    
    pub fn batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }
    
    pub fn poll_interval(mut self, ms: u64) -> Self {
        self.poll_interval_ms = ms;
        self
    }
    
    /// Process pending messages
    pub fn process_batch<F>(&self, publisher: F) -> usize
    where
        F: Fn(&OutboxMessage) -> crate::NoorResult<()>,
    {
        let pending = self.store.get_pending(self.batch_size);
        let mut processed = 0;
        
        for message in pending {
            if self.store.mark_processing(&message.id) {
                match publisher(&message) {
                    Ok(()) => {
                        self.store.mark_published(&message.id);
                        processed += 1;
                    }
                    Err(e) => {
                        self.store.mark_failed(&message.id, &e.to_string());
                        tracing::warn!("Failed to publish outbox message {}: {}", message.id, e);
                    }
                }
            }
        }
        
        processed
    }
    
    /// Get statistics
    pub fn stats(&self) -> OutboxStats {
        OutboxStats {
            pending: self.store.count_by_status(OutboxStatus::Pending),
            processing: self.store.count_by_status(OutboxStatus::Processing),
            published: self.store.count_by_status(OutboxStatus::Published),
            failed: self.store.count_by_status(OutboxStatus::Failed),
        }
    }
}

/// Outbox statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboxStats {
    pub pending: usize,
    pub processing: usize,
    pub published: usize,
    pub failed: usize,
}

impl OutboxStats {
    pub fn total(&self) -> usize {
        self.pending + self.processing + self.published + self.failed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_save_and_get() {
        let store = InMemoryOutboxStore::new();
        
        let msg = OutboxMessage::new("agg-1", "UserCreated", serde_json::json!({"id": 1}));
        let id = msg.id.clone();
        
        store.save(msg).unwrap();
        
        let retrieved = store.get(&id).unwrap();
        assert_eq!(retrieved.event_type, "UserCreated");
        assert_eq!(retrieved.status, OutboxStatus::Pending);
    }
    
    #[test]
    fn test_get_pending() {
        let store = InMemoryOutboxStore::new();
        
        store.save(OutboxMessage::new("agg-1", "Event1", serde_json::json!({}))).unwrap();
        store.save(OutboxMessage::new("agg-2", "Event2", serde_json::json!({}))).unwrap();
        store.save(OutboxMessage::new("agg-3", "Event3", serde_json::json!({}))).unwrap();
        
        let pending = store.get_pending(10);
        assert_eq!(pending.len(), 3);
        
        let pending_limited = store.get_pending(2);
        assert_eq!(pending_limited.len(), 2);
    }
    
    #[test]
    fn test_mark_published() {
        let store = InMemoryOutboxStore::new();
        
        let msg = OutboxMessage::new("agg-1", "Event", serde_json::json!({}));
        let id = msg.id.clone();
        
        store.save(msg).unwrap();
        
        assert!(store.mark_published(&id));
        
        let retrieved = store.get(&id).unwrap();
        assert_eq!(retrieved.status, OutboxStatus::Published);
        assert!(retrieved.processed_at.is_some());
    }
    
    #[test]
    fn test_mark_failed_retry() {
        let store = InMemoryOutboxStore::new();
        
        let msg = OutboxMessage::new("agg-1", "Event", serde_json::json!({}))
            .with_max_attempts(3);
        let id = msg.id.clone();
        
        store.save(msg).unwrap();
        
        // First failure
        store.mark_processing(&id);
        store.mark_failed(&id, "Connection error");
        
        let retrieved = store.get(&id).unwrap();
        assert_eq!(retrieved.status, OutboxStatus::Pending); // Can retry
        assert_eq!(retrieved.attempts, 1);
        
        // Second failure
        store.mark_processing(&id);
        store.mark_failed(&id, "Timeout");
        
        let retrieved = store.get(&id).unwrap();
        assert_eq!(retrieved.attempts, 2);
        assert_eq!(retrieved.status, OutboxStatus::Pending);
        
        // Third failure (max attempts reached)
        store.mark_processing(&id);
        store.mark_failed(&id, "Final error");
        
        let retrieved = store.get(&id).unwrap();
        assert_eq!(retrieved.attempts, 3);
        assert_eq!(retrieved.status, OutboxStatus::Failed);
    }
    
    #[test]
    fn test_outbox_processor() {
        let store = Arc::new(InMemoryOutboxStore::new());
        
        store.save(OutboxMessage::new("agg-1", "Event1", serde_json::json!({}))).unwrap();
        store.save(OutboxMessage::new("agg-2", "Event2", serde_json::json!({}))).unwrap();
        
        let processor = OutboxProcessor::new(store.clone());
        
        let processed = processor.process_batch(|msg| {
            Ok(())
        });
        
        assert_eq!(processed, 2);
        assert_eq!(store.count_by_status(OutboxStatus::Published), 2);
    }
    
    #[test]
    fn test_outbox_stats() {
        let store = Arc::new(InMemoryOutboxStore::new());
        
        store.save(OutboxMessage::new("1", "E1", serde_json::json!({}))).unwrap();
        store.save(OutboxMessage::new("2", "E2", serde_json::json!({}))).unwrap();
        
        let processor = OutboxProcessor::new(store.clone());
        processor.process_batch(|_| Ok(()));
        
        let stats = processor.stats();
        assert_eq!(stats.pending, 0);
        assert_eq!(stats.published, 2);
        assert_eq!(stats.total(), 2);
    }
}
