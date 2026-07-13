// ============================================================
// Soft Deletes - الحذف الناعم
// ============================================================
// Mark records as deleted instead of actually deleting them.
// Allows restoration of deleted records.
//
// تعليم السجلات كمحذوفة بدلاً من حذفها فعلياً.
// ============================================================

use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// Soft delete trait
pub trait SoftDeletes: Send + Sync {
    /// Get the soft delete column name
    fn deleted_at_column(&self) -> &str {
        "deleted_at"
    }
    
    /// Check if the record is soft deleted
    fn is_deleted(&self) -> bool;
    
    /// Soft delete the record
    fn soft_delete(&mut self) -> crate::NoorResult<()>;
    
    /// Restore a soft deleted record
    fn restore(&mut self) -> crate::NoorResult<()>;
    
    /// Force delete (permanent)
    fn force_delete(&self) -> crate::NoorResult<()>;
}

/// Soft delete model mixin
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SoftDeletesMixin {
    pub deleted_at: Option<i64>,
}

impl SoftDeletesMixin {
    pub fn new() -> Self {
        Self { deleted_at: None }
    }
    
    /// Check if soft deleted
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
    
    /// Soft delete
    pub fn soft_delete(&mut self) {
        self.deleted_at = Some(chrono::Utc::now().timestamp());
    }
    
    /// Restore
    pub fn restore(&mut self) {
        self.deleted_at = None;
    }
}

/// Query scope for soft deletes
pub struct SoftDeleteScope;

impl SoftDeleteScope {
    /// Add WHERE deleted_at IS NULL to query
    pub fn apply_only_not_deleted(query: crate::core::advanced_query::AdvancedQueryBuilder) -> crate::core::advanced_query::AdvancedQueryBuilder {
        query.where_null("deleted_at")
    }
    
    /// Include soft deleted records (no filter)
    pub fn apply_with_deleted(query: crate::core::advanced_query::AdvancedQueryBuilder) -> crate::core::advanced_query::AdvancedQueryBuilder {
        query
    }
    
    /// Only soft deleted records
    pub fn apply_only_deleted(query: crate::core::advanced_query::AdvancedQueryBuilder) -> crate::core::advanced_query::AdvancedQueryBuilder {
        query.where_not_null("deleted_at")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_soft_delete_mixin() {
        let mut mixin = SoftDeletesMixin::new();
        
        assert!(!mixin.is_deleted());
        
        mixin.soft_delete();
        assert!(mixin.is_deleted());
        assert!(mixin.deleted_at.is_some());
        
        mixin.restore();
        assert!(!mixin.is_deleted());
        assert!(mixin.deleted_at.is_none());
    }
}
