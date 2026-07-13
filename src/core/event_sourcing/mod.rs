// ============================================================
// Event Sourcing - مصدر الأحداث
// ============================================================
// Store all changes as a sequence of events.
// Rebuild state by replaying events. Enables time travel
// and audit trails.
//
// تخزين جميع التغييرات كتسلسل من الأحداث.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

/// Domain event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainEvent {
    pub id: String,
    pub aggregate_id: String,
    pub aggregate_type: String,
    pub event_type: String,
    pub data: serde_json::Value,
    pub metadata: EventMetadata,
    pub version: i64,
    pub timestamp: i64,
}

/// Event metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventMetadata {
    pub user_id: Option<String>,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

/// Event store trait
pub trait EventStore: Send + Sync {
    /// Append events to an aggregate
    fn append(&self, aggregate_id: &str, events: &[DomainEvent]) -> crate::NoorResult<()>;
    
    /// Get all events for an aggregate
    fn get_events(&self, aggregate_id: &str) -> crate::NoorResult<Vec<DomainEvent>>;
    
    /// Get events from a specific version
    fn get_events_from(&self, aggregate_id: &str, version: i64) -> crate::NoorResult<Vec<DomainEvent>>;
    
    /// Get all events of a specific type
    fn get_events_by_type(&self, event_type: &str) -> crate::NoorResult<Vec<DomainEvent>>;
    
    /// Get the current version of an aggregate
    fn get_version(&self, aggregate_id: &str) -> crate::NoorResult<i64>;
}

/// In-memory event store
pub struct InMemoryEventStore {
    events: Arc<RwLock<HashMap<String, Vec<DomainEvent>>>>,
}

impl Default for InMemoryEventStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryEventStore {
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl EventStore for InMemoryEventStore {
    fn append(&self, aggregate_id: &str, events: &[DomainEvent]) -> crate::NoorResult<()> {
        let mut store = self.events.write();
        let aggregate_events = store.entry(aggregate_id.to_string()).or_insert_with(Vec::new);
        
        for event in events {
            aggregate_events.push(event.clone());
        }
        
        Ok(())
    }
    
    fn get_events(&self, aggregate_id: &str) -> crate::NoorResult<Vec<DomainEvent>> {
        Ok(self.events.read().get(aggregate_id).cloned().unwrap_or_default())
    }
    
    fn get_events_from(&self, aggregate_id: &str, version: i64) -> crate::NoorResult<Vec<DomainEvent>> {
        let events = self.get_events(aggregate_id)?;
        Ok(events.into_iter().filter(|e| e.version > version).collect())
    }
    
    fn get_events_by_type(&self, event_type: &str) -> crate::NoorResult<Vec<DomainEvent>> {
        let store = self.events.read();
        let mut result = Vec::new();
        
        for events in store.values() {
            for event in events {
                if event.event_type == event_type {
                    result.push(event.clone());
                }
            }
        }
        
        Ok(result)
    }
    
    fn get_version(&self, aggregate_id: &str) -> crate::NoorResult<i64> {
        let events = self.get_events(aggregate_id)?;
        Ok(events.last().map(|e| e.version).unwrap_or(0))
    }
}

/// Aggregate root trait
pub trait AggregateRoot: Send + Sync {
    type Id: Send + Sync;
    
    /// Get the aggregate ID
    fn id(&self) -> &Self::Id;
    
    /// Get the aggregate type name
    fn aggregate_type(&self) -> &'static str;
    
    /// Apply an event to update state
    fn apply(&mut self, event: &DomainEvent);
    
    /// Get uncommitted events
    fn get_uncommitted_events(&self) -> &[DomainEvent];
    
    /// Clear uncommitted events
    fn clear_uncommitted_events(&mut self);
}

/// Event sourcing repository
pub struct EventSourcingRepository<A: AggregateRoot> {
    store: Arc<dyn EventStore>,
    _phantom: std::marker::PhantomData<A>,
}

impl<A: AggregateRoot> EventSourcingRepository<A> {
    pub fn new(store: Arc<dyn EventStore>) -> Self {
        Self {
            store,
            _phantom: std::marker::PhantomData,
        }
    }
    
    /// Save an aggregate by appending uncommitted events
    pub fn save(&self, aggregate: &mut A) -> crate::NoorResult<()>
    where
        A::Id: std::fmt::Display,
    {
        let events = aggregate.get_uncommitted_events();
        
        if !events.is_empty() {
            self.store.append(&aggregate.id().to_string(), events)?;
            aggregate.clear_uncommitted_events();
        }
        
        Ok(())
    }
    
    /// Load an aggregate by replaying events
    pub fn load<F>(&self, id: &str, factory: F) -> crate::NoorResult<A>
    where
        F: FnOnce() -> A,
        A::Id: std::fmt::Display,
    {
        let events = self.store.get_events(id)?;
        
        let mut aggregate = factory();
        
        for event in &events {
            aggregate.apply(event);
        }
        
        Ok(aggregate)
    }
}

/// Event factory for creating events
pub struct EventFactory;

impl EventFactory {
    /// Create a new domain event
    pub fn create(
        aggregate_id: &str,
        aggregate_type: &str,
        event_type: &str,
        data: serde_json::Value,
        version: i64,
    ) -> DomainEvent {
        DomainEvent {
            id: uuid::Uuid::new_v4().to_string(),
            aggregate_id: aggregate_id.to_string(),
            aggregate_type: aggregate_type.to_string(),
            event_type: event_type.to_string(),
            data,
            metadata: EventMetadata::default(),
            version,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
    
    /// Create with metadata
    pub fn create_with_metadata(
        aggregate_id: &str,
        aggregate_type: &str,
        event_type: &str,
        data: serde_json::Value,
        version: i64,
        metadata: EventMetadata,
    ) -> DomainEvent {
        DomainEvent {
            id: uuid::Uuid::new_v4().to_string(),
            aggregate_id: aggregate_id.to_string(),
            aggregate_type: aggregate_type.to_string(),
            event_type: event_type.to_string(),
            data,
            metadata,
            version,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

/// Snapshot for performance optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub aggregate_id: String,
    pub aggregate_type: String,
    pub version: i64,
    pub state: serde_json::Value,
    pub created_at: i64,
}

impl Snapshot {
    pub fn new(aggregate_id: &str, aggregate_type: &str, version: i64, state: serde_json::Value) -> Self {
        Self {
            aggregate_id: aggregate_id.to_string(),
            aggregate_type: aggregate_type.to_string(),
            version,
            state,
            created_at: chrono::Utc::now().timestamp(),
        }
    }
}

/// Snapshot store
pub struct SnapshotStore {
    snapshots: Arc<RwLock<HashMap<String, Snapshot>>>,
}

impl Default for SnapshotStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SnapshotStore {
    pub fn new() -> Self {
        Self {
            snapshots: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub fn save(&self, snapshot: Snapshot) {
        self.snapshots.write().insert(snapshot.aggregate_id.clone(), snapshot);
    }
    
    pub fn load(&self, aggregate_id: &str) -> Option<Snapshot> {
        self.snapshots.read().get(aggregate_id).cloned()
    }
    
    pub fn delete(&self, aggregate_id: &str) -> bool {
        self.snapshots.write().remove(aggregate_id).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Test aggregate
    #[derive(Debug, Clone)]
    struct UserAggregate {
        id: String,
        name: String,
        email: String,
        active: bool,
        version: i64,
        uncommitted: Vec<DomainEvent>,
    }
    
    impl UserAggregate {
        fn new(id: &str) -> Self {
            Self {
                id: id.to_string(),
                name: String::new(),
                email: String::new(),
                active: false,
                version: 0,
                uncommitted: Vec::new(),
            }
        }
        
        fn create(id: &str, name: &str, email: &str) -> Self {
            let mut user = Self::new(id);
            let event = EventFactory::create(
                id,
                "User",
                "UserCreated",
                serde_json::json!({"name": name, "email": email}),
                1,
            );
            user.apply(&event);
            user.uncommitted.push(event);
            user
        }
        
        fn activate(&mut self) {
            let event = EventFactory::create(
                &self.id,
                "User",
                "UserActivated",
                serde_json::json!({}),
                self.version + 1,
            );
            self.apply(&event);
            self.uncommitted.push(event);
        }
        
        fn deactivate(&mut self) {
            let event = EventFactory::create(
                &self.id,
                "User",
                "UserDeactivated",
                serde_json::json!({}),
                self.version + 1,
            );
            self.apply(&event);
            self.uncommitted.push(event);
        }
        
        fn change_name(&mut self, new_name: &str) {
            let event = EventFactory::create(
                &self.id,
                "User",
                "UserNameChanged",
                serde_json::json!({"name": new_name}),
                self.version + 1,
            );
            self.apply(&event);
            self.uncommitted.push(event);
        }
    }
    
    impl AggregateRoot for UserAggregate {
        type Id = String;
        
        fn id(&self) -> &String {
            &self.id
        }
        
        fn aggregate_type(&self) -> &'static str {
            "User"
        }
        
        fn apply(&mut self, event: &DomainEvent) {
            match event.event_type.as_str() {
                "UserCreated" => {
                    self.name = event.data["name"].as_str().unwrap_or("").to_string();
                    self.email = event.data["email"].as_str().unwrap_or("").to_string();
                    self.active = true;
                }
                "UserActivated" => {
                    self.active = true;
                }
                "UserDeactivated" => {
                    self.active = false;
                }
                "UserNameChanged" => {
                    self.name = event.data["name"].as_str().unwrap_or("").to_string();
                }
                _ => {}
            }
            self.version = event.version;
        }
        
        fn get_uncommitted_events(&self) -> &[DomainEvent] {
            &self.uncommitted
        }
        
        fn clear_uncommitted_events(&mut self) {
            self.uncommitted.clear();
        }
    }
    
    #[test]
    fn test_event_store_append_and_read() {
        let store = InMemoryEventStore::new();
        
        let event1 = EventFactory::create("user-1", "User", "UserCreated", serde_json::json!({}), 1);
        let event2 = EventFactory::create("user-1", "User", "UserActivated", serde_json::json!({}), 2);
        
        store.append("user-1", &[event1, event2]).unwrap();
        
        let events = store.get_events("user-1").unwrap();
        assert_eq!(events.len(), 2);
        
        assert_eq!(store.get_version("user-1").unwrap(), 2);
    }
    
    #[test]
    fn test_aggregate_apply_events() {
        let mut user = UserAggregate::create("user-1", "John", "john@example.com");
        
        assert_eq!(user.name, "John");
        assert!(user.active);
        assert_eq!(user.version, 1);
        
        user.deactivate();
        assert!(!user.active);
        assert_eq!(user.version, 2);
        
        user.activate();
        assert!(user.active);
        assert_eq!(user.version, 3);
    }
    
    #[test]
    fn test_event_sourcing_repository() {
        let store = Arc::new(InMemoryEventStore::new());
        let repo = EventSourcingRepository::<UserAggregate>::new(store.clone());
        
        // Create and save
        let mut user = UserAggregate::create("user-1", "John", "john@example.com");
        user.activate();
        
        repo.save(&mut user).unwrap();
        
        // Load and verify
        let loaded = repo.load("user-1", || UserAggregate::new("user-1")).unwrap();
        
        assert_eq!(loaded.name, "John");
        assert_eq!(loaded.email, "john@example.com");
        assert!(loaded.active);
        assert_eq!(loaded.version, 2);
    }
    
    #[test]
    fn test_time_travel() {
        let store = Arc::new(InMemoryEventStore::new());
        
        // Create events
        let events = vec![
            EventFactory::create("user-1", "User", "UserCreated", serde_json::json!({"name": "John"}), 1),
            EventFactory::create("user-1", "User", "UserNameChanged", serde_json::json!({"name": "Jane"}), 2),
            EventFactory::create("user-1", "User", "UserNameChanged", serde_json::json!({"name": "Bob"}), 3),
        ];
        
        store.append("user-1", &events).unwrap();
        
        // Get events from version 1 (should get events 2 and 3)
        let from_v1 = store.get_events_from("user-1", 1).unwrap();
        assert_eq!(from_v1.len(), 2);
        
        // Get events by type
        let name_changes = store.get_events_by_type("UserNameChanged").unwrap();
        assert_eq!(name_changes.len(), 2);
    }
    
    #[test]
    fn test_snapshot_store() {
        let store = SnapshotStore::new();
        
        let snapshot = Snapshot::new(
            "user-1",
            "User",
            10,
            serde_json::json!({"name": "John", "email": "john@example.com"}),
        );
        
        store.save(snapshot);
        
        let loaded = store.load("user-1").unwrap();
        assert_eq!(loaded.version, 10);
        assert_eq!(loaded.state["name"], "John");
        
        assert!(store.delete("user-1"));
        assert!(store.load("user-1").is_none());
    }
}
