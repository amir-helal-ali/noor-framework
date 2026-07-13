// ============================================================
// State Machine - آلة الحالة
// ============================================================
// Workflow management with states, transitions, and guards.
// Useful for order processing, content moderation, etc.
//
// إدارة سير العمل بالحالات والانتقالات.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// State machine definition
#[derive(Clone)]
pub struct StateMachine<S: Clone + PartialEq, E: Clone + PartialEq> {
    pub initial_state: S,
    pub states: Vec<S>,
    pub transitions: Vec<Transition<S, E>>,
}

/// A transition from one state to another
#[derive(Clone)]
pub struct Transition<S: Clone + PartialEq, E: Clone + PartialEq> {
    pub from: S,
    pub to: S,
    pub event: E,
    pub guard: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
}

impl<S: Clone + PartialEq, E: Clone + PartialEq> StateMachine<S, E> {
    pub fn new(initial_state: S) -> Self {
        Self {
            initial_state: initial_state.clone(),
            states: vec![initial_state],
            transitions: vec![],
        }
    }
    
    /// Add a state
    pub fn state(mut self, state: S) -> Self {
        if !self.states.contains(&state) {
            self.states.push(state);
        }
        self
    }
    
    /// Add a transition
    pub fn transition(mut self, from: S, event: E, to: S) -> Self {
        // Add states if they don't exist
        if !self.states.contains(&from) {
            self.states.push(from.clone());
        }
        if !self.states.contains(&to) {
            self.states.push(to.clone());
        }
        
        self.transitions.push(Transition {
            from,
            to,
            event,
            guard: None,
        });
        
        self
    }
    
    /// Add a transition with a guard
    pub fn transition_guarded<F>(mut self, from: S, event: E, to: S, guard: F) -> Self
    where
        F: Fn() -> bool + Send + Sync + 'static,
    {
        if !self.states.contains(&from) {
            self.states.push(from.clone());
        }
        if !self.states.contains(&to) {
            self.states.push(to.clone());
        }
        
        self.transitions.push(Transition {
            from,
            to,
            event,
            guard: Some(Arc::new(guard)),
        });
        
        self
    }
    
    /// Get all possible transitions from a state
    pub fn transitions_from(&self, state: &S) -> Vec<&Transition<S, E>> {
        self.transitions
            .iter()
            .filter(|t| t.from == *state)
            .collect()
    }
    
    /// Check if a transition is allowed
    pub fn can_transition(&self, from: &S, event: &E) -> bool {
        self.transitions
            .iter()
            .any(|t| t.from == *from && t.event == *event && t.guard.as_ref().map(|g| g()).unwrap_or(true))
    }
    
    /// Get the target state for an event
    pub fn target_state(&self, from: &S, event: &E) -> Option<S> {
        self.transitions
            .iter()
            .find(|t| t.from == *from && t.event == *event && t.guard.as_ref().map(|g| g()).unwrap_or(true))
            .map(|t| t.to.clone())
    }
    
    /// Get all valid events for a state
    pub fn valid_events(&self, state: &S) -> Vec<E> {
        self.transitions
            .iter()
            .filter(|t| t.from == *state)
            .map(|t| t.event.clone())
            .collect()
    }
}

/// State machine instance (runtime)
#[derive(Debug, Clone)]
pub struct StateMachineInstance<S> {
    pub id: String,
    pub current_state: S,
    pub history: Vec<StateHistoryEntry<S>>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct StateHistoryEntry<S> {
    pub from: S,
    pub to: S,
    pub event: String,
    pub timestamp: i64,
    pub metadata: Option<serde_json::Value>,
}

impl<S: Clone + PartialEq + std::fmt::Debug> StateMachineInstance<S> {
    pub fn new(id: &str, initial_state: S) -> Self {
        Self {
            id: id.to_string(),
            current_state: initial_state,
            history: vec![],
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
        }
    }
    
    /// Transition to a new state
    pub fn transition<E: Clone + PartialEq + std::fmt::Debug>(&mut self, machine: &StateMachine<S, E>, event: E) -> Result<(), String>
    where
        E: ToString,
    {
        if let Some(new_state) = machine.target_state(&self.current_state, &event) {
            let event_name = event.to_string();
            
            self.history.push(StateHistoryEntry {
                from: self.current_state.clone(),
                to: new_state.clone(),
                event: event_name,
                timestamp: chrono::Utc::now().timestamp(),
                metadata: None,
            });
            
            self.current_state = new_state;
            self.updated_at = chrono::Utc::now().timestamp();
            
            Ok(())
        } else {
            Err(format!("Cannot transition from {:?} with event {:?}", self.current_state, event))
        }
    }
    
    /// Get the current state
    pub fn state(&self) -> &S {
        &self.current_state
    }
    
    /// Check if in a specific state
    pub fn is_in(&self, state: &S) -> bool {
        self.current_state == *state
    }
    
    /// Get transition history
    pub fn history(&self) -> &[StateHistoryEntry<S>] {
        &self.history
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[derive(Debug, Clone, Copy, PartialEq)]
    enum OrderState {
        Pending,
        Paid,
        Shipped,
        Delivered,
        Canceled,
    }
    
    impl std::fmt::Display for OrderState {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self)
        }
    }
    
    #[derive(Debug, Clone, Copy, PartialEq)]
    enum OrderEvent {
        Pay,
        Ship,
        Deliver,
        Cancel,
    }
    
    impl std::fmt::Display for OrderEvent {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self)
        }
    }
    
    fn create_order_machine() -> StateMachine<OrderState, OrderEvent> {
        StateMachine::new(OrderState::Pending)
            .transition(OrderState::Pending, OrderEvent::Pay, OrderState::Paid)
            .transition(OrderState::Pending, OrderEvent::Cancel, OrderState::Canceled)
            .transition(OrderState::Paid, OrderEvent::Ship, OrderState::Shipped)
            .transition(OrderState::Paid, OrderEvent::Cancel, OrderState::Canceled)
            .transition(OrderState::Shipped, OrderEvent::Deliver, OrderState::Delivered)
    }
    
    #[test]
    fn test_state_machine_transitions() {
        let machine = create_order_machine();
        
        assert!(machine.can_transition(&OrderState::Pending, &OrderEvent::Pay));
        assert!(!machine.can_transition(&OrderState::Pending, &OrderEvent::Ship));
        
        let target = machine.target_state(&OrderState::Pending, &OrderEvent::Pay);
        assert_eq!(target, Some(OrderState::Paid));
    }
    
    #[test]
    fn test_state_machine_instance() {
        let machine = create_order_machine();
        let mut order = StateMachineInstance::new("order-1", OrderState::Pending);
        
        assert!(order.is_in(&OrderState::Pending));
        
        // Pay
        order.transition(&machine, OrderEvent::Pay).unwrap();
        assert!(order.is_in(&OrderState::Paid));
        
        // Ship
        order.transition(&machine, OrderEvent::Ship).unwrap();
        assert!(order.is_in(&OrderState::Shipped));
        
        // Try invalid transition
        assert!(order.transition(&machine, OrderEvent::Pay).is_err());
        
        // Check history
        assert_eq!(order.history().len(), 2);
    }
    
    #[test]
    fn test_valid_events() {
        let machine = create_order_machine();
        
        let events = machine.valid_events(&OrderState::Pending);
        assert!(events.contains(&OrderEvent::Pay));
        assert!(events.contains(&OrderEvent::Cancel));
        assert!(!events.contains(&OrderEvent::Ship));
    }
    
    #[test]
    fn test_guarded_transition() {
        let machine = StateMachine::new(OrderState::Pending)
            .transition_guarded(OrderState::Pending, OrderEvent::Pay, OrderState::Paid, || false);
        
        // Guard returns false, so transition should not be allowed
        assert!(!machine.can_transition(&OrderState::Pending, &OrderEvent::Pay));
    }
}
