// ============================================================
// Saga Pattern - نمط الساغا
// ============================================================
// Manages distributed transactions across multiple services.
// Coordinates a sequence of local transactions with compensating
// actions for rollback.
//
// يدير المعاملات الموزعة عبر خدمات متعددة.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// Saga step
#[derive(Clone)]
pub struct SagaStep {
    pub name: String,
    pub execute: Arc<dyn Fn() -> crate::NoorResult<()> + Send + Sync>,
    pub compensate: Arc<dyn Fn() -> crate::NoorResult<()> + Send + Sync>,
}

impl SagaStep {
    pub fn new<F, C>(name: &str, execute: F, compensate: C) -> Self
    where
        F: Fn() -> crate::NoorResult<()> + Send + Sync + 'static,
        C: Fn() -> crate::NoorResult<()> + Send + Sync + 'static,
    {
        Self {
            name: name.to_string(),
            execute: Arc::new(execute),
            compensate: Arc::new(compensate),
        }
    }
}

/// Saga state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SagaState {
    Pending,
    Running,
    Completed,
    Compensating,
    Compensated,
    Failed,
}

/// Saga execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaStatus {
    pub id: String,
    pub name: String,
    pub state: SagaState,
    pub completed_steps: Vec<String>,
    pub failed_step: Option<String>,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub error: Option<String>,
}

/// Saga definition
pub struct Saga {
    pub name: String,
    pub steps: Vec<SagaStep>,
}

impl Saga {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            steps: Vec::new(),
        }
    }
    
    pub fn step(mut self, step: SagaStep) -> Self {
        self.steps.push(step);
        self
    }
    
    /// Execute the saga
    pub fn execute(&self) -> SagaResult {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();
        
        let mut status = SagaStatus {
            id: id.clone(),
            name: self.name.clone(),
            state: SagaState::Running,
            completed_steps: Vec::new(),
            failed_step: None,
            started_at: now,
            completed_at: None,
            error: None,
        };
        
        // Execute steps
        for step in &self.steps {
            match (step.execute)() {
                Ok(()) => {
                    status.completed_steps.push(step.name.clone());
                }
                Err(e) => {
                    status.failed_step = Some(step.name.clone());
                    status.error = Some(e.to_string());
                    status.state = SagaState::Compensating;
                    
                    // Compensate in reverse order
                    for step_name in status.completed_steps.iter().rev() {
                        if let Some(compensate_step) = self.steps.iter().find(|s| &s.name == step_name) {
                            tracing::info!("Compensating step: {}", step_name);
                            if let Err(e) = (compensate_step.compensate)() {
                                tracing::error!("Compensation failed for {}: {}", step_name, e);
                            }
                        }
                    }
                    
                    status.state = SagaState::Failed;
                    status.completed_at = Some(chrono::Utc::now().timestamp());
                    return SagaResult::Failed(status);
                }
            }
        }
        
        status.state = SagaState::Completed;
        status.completed_at = Some(chrono::Utc::now().timestamp());
        SagaResult::Success(status)
    }
}

/// Saga execution result
pub enum SagaResult {
    Success(SagaStatus),
    Failed(SagaStatus),
}

impl SagaResult {
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success(_))
    }
    
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed(_))
    }
    
    pub fn status(&self) -> &SagaStatus {
        match self {
            Self::Success(s) => s,
            Self::Failed(s) => s,
        }
    }
}

/// Saga orchestrator for managing multiple sagas
pub struct SagaOrchestrator {
    sagas: Arc<RwLock<HashMap<String, SagaStatus>>>,
}

impl Default for SagaOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl SagaOrchestrator {
    pub fn new() -> Self {
        Self {
            sagas: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Execute a saga and track it
    pub fn execute(&self, saga: Saga) -> SagaResult {
        let result = saga.execute();
        
        let status = result.status().clone();
        self.sagas.write().insert(status.id.clone(), status);
        
        result
    }
    
    /// Get saga status by ID
    pub fn get_status(&self, saga_id: &str) -> Option<SagaStatus> {
        self.sagas.read().get(saga_id).cloned()
    }
    
    /// Get all saga statuses
    pub fn all_statuses(&self) -> Vec<SagaStatus> {
        self.sagas.read().values().cloned().collect()
    }
    
    /// Get count of sagas by state
    pub fn count_by_state(&self, state: SagaState) -> usize {
        self.sagas.read().values().filter(|s| s.state == state).count()
    }
    
    /// Get total saga count
    pub fn count(&self) -> usize {
        self.sagas.read().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    
    #[test]
    fn test_saga_success() {
        let counter = Arc::new(AtomicUsize::new(0));
        
        let saga = Saga::new("order_creation")
            .step(SagaStep::new(
                "create_order",
                { let c = counter.clone(); move || { c.fetch_add(1, Ordering::SeqCst); Ok(()) } },
                || Ok(()),
            ))
            .step(SagaStep::new(
                "charge_payment",
                { let c = counter.clone(); move || { c.fetch_add(1, Ordering::SeqCst); Ok(()) } },
                || Ok(()),
            ))
            .step(SagaStep::new(
                "ship_order",
                { let c = counter.clone(); move || { c.fetch_add(1, Ordering::SeqCst); Ok(()) } },
                || Ok(()),
            ));
        
        let result = saga.execute();
        
        assert!(result.is_success());
        assert_eq!(counter.load(Ordering::SeqCst), 3);
        assert_eq!(result.status().completed_steps.len(), 3);
    }
    
    #[test]
    fn test_saga_failure_with_compensation() {
        let executed = Arc::new(AtomicUsize::new(0));
        let compensated = Arc::new(AtomicUsize::new(0));
        
        let saga = Saga::new("failing_saga")
            .step(SagaStep::new(
                "step1",
                { let c = executed.clone(); move || { c.fetch_add(1, Ordering::SeqCst); Ok(()) } },
                { let c = compensated.clone(); move || { c.fetch_add(1, Ordering::SeqCst); Ok(()) } },
            ))
            .step(SagaStep::new(
                "step2",
                { let c = executed.clone(); move || { c.fetch_add(1, Ordering::SeqCst); Ok(()) } },
                { let c = compensated.clone(); move || { c.fetch_add(1, Ordering::SeqCst); Ok(()) } },
            ))
            .step(SagaStep::new(
                "step3_fails",
                move || { Err(crate::NoorError::Internal("Step 3 failed".to_string())) },
                move || Ok(()),
            ));
        
        let result = saga.execute();
        
        assert!(result.is_failed());
        assert_eq!(executed.load(Ordering::SeqCst), 2); // Steps 1 and 2
        assert_eq!(compensated.load(Ordering::SeqCst), 2); // Compensated 1 and 2
        assert_eq!(result.status().failed_step, Some("step3_fails".to_string()));
    }
    
    #[test]
    fn test_saga_orchestrator() {
        let orchestrator = SagaOrchestrator::new();
        
        let saga = Saga::new("test_saga")
            .step(SagaStep::new("step1", || Ok(()), || Ok(())));
        
        let result = orchestrator.execute(saga);
        
        assert!(result.is_success());
        assert_eq!(orchestrator.count(), 1);
        assert_eq!(orchestrator.count_by_state(SagaState::Completed), 1);
    }
    
    #[test]
    fn test_saga_status_tracking() {
        let orchestrator = SagaOrchestrator::new();
        
        let saga = Saga::new("tracked_saga")
            .step(SagaStep::new("step1", || Ok(()), || Ok(())))
            .step(SagaStep::new("step2", || Ok(()), || Ok(())));
        
        let result = orchestrator.execute(saga);
        let saga_id = result.status().id.clone();
        
        let status = orchestrator.get_status(&saga_id).unwrap();
        assert_eq!(status.state, SagaState::Completed);
        assert_eq!(status.completed_steps.len(), 2);
    }
}
