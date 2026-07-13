// ============================================================
// Transaction Manager - مدير المعاملات
// ============================================================
// Database transaction support with nested transactions
// (savepoints), automatic rollback on error, and isolation levels.
//
// دعم معاملات قاعدة البيانات مع المعاملات المتداخلة.
// ============================================================

use std::sync::Arc;
use parking_lot::{RwLock, Mutex};
use serde::{Serialize, Deserialize};

/// Transaction isolation level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IsolationLevel {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Serializable,
}

impl IsolationLevel {
    pub fn as_sql(&self) -> &'static str {
        match self {
            Self::ReadUncommitted => "READ UNCOMMITTED",
            Self::ReadCommitted => "READ COMMITTED",
            Self::RepeatableRead => "REPEATABLE READ",
            Self::Serializable => "SERIALIZABLE",
        }
    }
}

/// Transaction state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransactionState {
    Active,
    Committed,
    RolledBack,
}

/// Transaction information
#[derive(Debug, Clone)]
struct TransactionInfo {
    id: String,
    savepoint_name: Option<String>,
    state: TransactionState,
    started_at: i64,
    queries: Vec<String>,
}

/// Transaction manager
pub struct TransactionManager {
    /// Stack of active transactions (for nested support)
    transactions: Arc<Mutex<Vec<TransactionInfo>>>,
    /// Default isolation level
    default_isolation: IsolationLevel,
    /// Transaction counter for generating IDs
    counter: Arc<Mutex<u64>>,
    /// Whether transactions are enabled
    enabled: Arc<RwLock<bool>>,
    /// Running counters of finished transactions (the stack only holds active
    /// ones, so we need separate counters for stats).
    committed_count: Arc<Mutex<usize>>,
    rolled_back_count: Arc<Mutex<usize>>,
}

impl Default for TransactionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TransactionManager {
    pub fn new() -> Self {
        Self {
            transactions: Arc::new(Mutex::new(Vec::new())),
            default_isolation: IsolationLevel::ReadCommitted,
            counter: Arc::new(Mutex::new(0)),
            enabled: Arc::new(RwLock::new(true)),
            committed_count: Arc::new(Mutex::new(0)),
            rolled_back_count: Arc::new(Mutex::new(0)),
        }
    }
    
    /// Set the default isolation level
    pub fn set_isolation_level(&mut self, level: IsolationLevel) {
        self.default_isolation = level;
    }
    
    /// Enable or disable transactions
    pub fn set_enabled(&self, enabled: bool) {
        *self.enabled.write() = enabled;
    }
    
    /// Check if transactions are enabled
    pub fn is_enabled(&self) -> bool {
        *self.enabled.read()
    }
    
    /// Begin a new transaction
    pub fn begin(&self) -> crate::NoorResult<String> {
        if !self.is_enabled() {
            return Ok("disabled".to_string());
        }
        
        let mut counter = self.counter.lock();
        *counter += 1;
        let id = format!("txn_{}", *counter);
        
        let mut transactions = self.transactions.lock();
        
        let savepoint_name = if transactions.is_empty() {
            // First transaction - use BEGIN
            None
        } else {
            // Nested transaction - use SAVEPOINT
            Some(format!("sp_{}", *counter))
        };
        
        let info = TransactionInfo {
            id: id.clone(),
            savepoint_name: savepoint_name.clone(),
            state: TransactionState::Active,
            started_at: chrono::Utc::now().timestamp(),
            queries: Vec::new(),
        };
        
        if let Some(ref sp) = savepoint_name {
            // Nested: create savepoint
            tracing::debug!("SAVEPOINT {}", sp);
        } else {
            // Top-level: begin transaction
            tracing::debug!("BEGIN TRANSACTION ISOLATION LEVEL {}", self.default_isolation.as_sql());
        }
        
        transactions.push(info);
        
        Ok(id)
    }
    
    /// Commit the current transaction
    pub fn commit(&self) -> crate::NoorResult<()> {
        if !self.is_enabled() {
            return Ok(());
        }
        
        let mut transactions = self.transactions.lock();
        
        if transactions.is_empty() {
            return Err(crate::NoorError::Database("No active transaction".to_string()));
        }
        
        let info = transactions.last_mut().unwrap();
        
        if info.state != TransactionState::Active {
            return Err(crate::NoorError::Database(
                format!("Transaction {} is not active", info.id)
            ));
        }
        
        if let Some(ref sp) = info.savepoint_name {
            // Release savepoint
            tracing::debug!("RELEASE SAVEPOINT {}", sp);
        } else {
            // Commit top-level transaction
            tracing::debug!("COMMIT");
        }
        
        info.state = TransactionState::Committed;
        transactions.pop();
        *self.committed_count.lock() += 1;

        Ok(())
    }
    
    /// Rollback the current transaction
    pub fn rollback(&self) -> crate::NoorResult<()> {
        if !self.is_enabled() {
            return Ok(());
        }
        
        let mut transactions = self.transactions.lock();
        
        if transactions.is_empty() {
            return Err(crate::NoorError::Database("No active transaction".to_string()));
        }
        
        let info = transactions.last_mut().unwrap();
        
        if info.state != TransactionState::Active {
            return Err(crate::NoorError::Database(
                format!("Transaction {} is not active", info.id)
            ));
        }
        
        if let Some(ref sp) = info.savepoint_name {
            // Rollback to savepoint
            tracing::debug!("ROLLBACK TO SAVEPOINT {}", sp);
        } else {
            // Rollback top-level transaction
            tracing::debug!("ROLLBACK");
        }
        
        info.state = TransactionState::RolledBack;
        transactions.pop();
        *self.rolled_back_count.lock() += 1;

        Ok(())
    }
    
    /// Execute a closure within a transaction
    pub fn transaction<F, T>(&self, f: F) -> crate::NoorResult<T>
    where
        F: FnOnce() -> crate::NoorResult<T>,
    {
        self.begin()?;
        
        match f() {
            Ok(result) => {
                self.commit()?;
                Ok(result)
            }
            Err(e) => {
                self.rollback()?;
                Err(e)
            }
        }
    }
    
    /// Execute a closure within a transaction with custom error handling
    pub fn transaction_or<F, T, E>(&self, f: F) -> Result<T, E>
    where
        F: FnOnce() -> Result<T, E>,
        E: From<crate::NoorError>,
    {
        self.begin().map_err(E::from)?;
        
        match f() {
            Ok(result) => {
                self.commit().map_err(E::from)?;
                Ok(result)
            }
            Err(e) => {
                self.rollback().ok();
                Err(e)
            }
        }
    }
    
    /// Record a query in the current transaction
    pub fn record_query(&self, sql: &str) {
        if let Some(txn) = self.transactions.lock().last_mut() {
            if txn.state == TransactionState::Active {
                txn.queries.push(sql.to_string());
            }
        }
    }
    
    /// Get the current transaction depth (0 = no transaction)
    pub fn depth(&self) -> usize {
        self.transactions.lock().len()
    }
    
    /// Check if inside a transaction
    pub fn in_transaction(&self) -> bool {
        self.depth() > 0
    }
    
    /// Get the current transaction ID
    pub fn current_id(&self) -> Option<String> {
        self.transactions.lock().last().map(|t| t.id.clone())
    }
    
    /// Get queries executed in the current transaction
    pub fn current_queries(&self) -> Vec<String> {
        self.transactions
            .lock()
            .last()
            .map(|t| t.queries.clone())
            .unwrap_or_default()
    }
    
    /// Get active transaction count
    pub fn active_count(&self) -> usize {
        self.transactions
            .lock()
            .iter()
            .filter(|t| t.state == TransactionState::Active)
            .count()
    }
    
    /// Get transaction statistics
    pub fn stats(&self) -> TransactionStats {
        let transactions = self.transactions.lock();

        TransactionStats {
            active: transactions.iter().filter(|t| t.state == TransactionState::Active).count(),
            committed: *self.committed_count.lock(),
            rolled_back: *self.rolled_back_count.lock(),
            total_queries: transactions.iter().map(|t| t.queries.len()).sum(),
        }
    }
}

/// Transaction statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionStats {
    pub active: usize,
    pub committed: usize,
    pub rolled_back: usize,
    pub total_queries: usize,
}

/// Transaction builder for configuring isolation level
pub struct TransactionBuilder<'a> {
    manager: &'a TransactionManager,
    isolation: Option<IsolationLevel>,
    read_only: bool,
}

impl<'a> TransactionBuilder<'a> {
    pub fn new(manager: &'a TransactionManager) -> Self {
        Self {
            manager,
            isolation: None,
            read_only: false,
        }
    }
    
    /// Set isolation level
    pub fn isolation(mut self, level: IsolationLevel) -> Self {
        self.isolation = Some(level);
        self
    }
    
    /// Set as read-only
    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }
    
    /// Begin the transaction
    pub fn begin(self) -> crate::NoorResult<String> {
        if let Some(level) = self.isolation {
            tracing::debug!("SET TRANSACTION ISOLATION LEVEL {}", level.as_sql());
        }
        
        if self.read_only {
            tracing::debug!("SET TRANSACTION READ ONLY");
        }
        
        self.manager.begin()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_transaction() {
        let manager = TransactionManager::new();
        
        assert!(!manager.in_transaction());
        
        let id = manager.begin().unwrap();
        assert!(manager.in_transaction());
        assert_eq!(manager.depth(), 1);
        
        manager.commit().unwrap();
        assert!(!manager.in_transaction());
    }
    
    #[test]
    fn test_rollback() {
        let manager = TransactionManager::new();
        
        manager.begin().unwrap();
        assert!(manager.in_transaction());
        
        manager.rollback().unwrap();
        assert!(!manager.in_transaction());
    }
    
    #[test]
    fn test_transaction_closure_success() {
        let manager = TransactionManager::new();
        
        let result: crate::NoorResult<i32> = manager.transaction(|| {
            Ok(42)
        });
        
        assert_eq!(result.unwrap(), 42);
        assert!(!manager.in_transaction());
    }
    
    #[test]
    fn test_transaction_closure_failure() {
        let manager = TransactionManager::new();
        
        let result: crate::NoorResult<i32> = manager.transaction(|| {
            Err(crate::NoorError::Database("Something went wrong".to_string()))
        });
        
        assert!(result.is_err());
        assert!(!manager.in_transaction());
    }
    
    #[test]
    fn test_nested_transactions() {
        let manager = TransactionManager::new();
        
        manager.begin().unwrap();
        assert_eq!(manager.depth(), 1);
        
        manager.begin().unwrap();
        assert_eq!(manager.depth(), 2);
        
        manager.commit().unwrap(); // Commit inner
        assert_eq!(manager.depth(), 1);
        
        manager.commit().unwrap(); // Commit outer
        assert_eq!(manager.depth(), 0);
    }
    
    #[test]
    fn test_nested_rollback() {
        let manager = TransactionManager::new();
        
        manager.begin().unwrap(); // Outer
        manager.begin().unwrap(); // Inner
        
        manager.rollback().unwrap(); // Rollback inner
        assert_eq!(manager.depth(), 1);
        
        manager.commit().unwrap(); // Commit outer
        assert_eq!(manager.depth(), 0);
    }
    
    #[test]
    fn test_query_recording() {
        let manager = TransactionManager::new();
        
        manager.begin().unwrap();
        
        manager.record_query("SELECT * FROM users");
        manager.record_query("INSERT INTO logs VALUES (...)"); 
        
        let queries = manager.current_queries();
        assert_eq!(queries.len(), 2);
        assert!(queries[0].contains("SELECT"));
        
        manager.rollback().unwrap();
    }
    
    #[test]
    fn test_transaction_stats() {
        let manager = TransactionManager::new();
        
        // Successful transaction
        manager.transaction(|| Ok(())).unwrap();
        
        // Failed transaction
        let fail_result: crate::NoorResult<()> =
            manager.transaction(|| Err(crate::NoorError::Database("error".to_string())));
        fail_result.ok();
        
        let stats = manager.stats();
        assert_eq!(stats.committed, 1);
        assert_eq!(stats.rolled_back, 1);
    }
    
    #[test]
    fn test_transaction_builder() {
        let manager = TransactionManager::new();
        
        let id = TransactionBuilder::new(&manager)
            .isolation(IsolationLevel::Serializable)
            .read_only()
            .begin()
            .unwrap();
        
        assert!(manager.in_transaction());
        
        manager.commit().unwrap();
    }
}
