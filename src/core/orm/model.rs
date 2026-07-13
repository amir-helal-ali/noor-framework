// ============================================================
// Model Definition - تعريف النموذج
// ============================================================

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Metadata about a model
/// معلومات عن النموذج
#[derive(Debug, Clone)]
pub struct ModelMeta {
    pub table_name: String,
    pub primary_key: String,
    pub fillable: Vec<String>,
    pub hidden: Vec<String>,
    pub casts: HashMap<String, CastType>,
    pub timestamps: bool,
}

/// Types for casting model attributes
/// أنواع لتحويل سمات النموذج
#[derive(Debug, Clone)]
pub enum CastType {
    Integer,
    Float,
    Boolean,
    String,
    Json,
    DateTime,
    Array,
}

/// Model trait that all models must implement
/// Trait يجب على جميع النماذج تطبيقه
pub trait Model: Send + Sync {
    /// Get the model metadata
    fn meta(&self) -> &ModelMeta;

    /// Convert the model to a JSON value
    fn to_json(&self) -> serde_json::Value;

    /// Get the primary key value
    fn get_id(&self) -> Option<serde_json::Value>;
}

/// Database query helpers for models.
///
/// Provides static-like methods (`find`, `all`, `where_`, `insert`, etc.)
/// that operate on a `Database` using the model's `ModelMeta`. Models
/// implement `Model` and can then use `ModelQueries::find(db, id)` etc.
///
/// This is a separate trait from `Model` so that callers can opt into the
/// async DB layer without being forced to pull in `Database` / `sqlx`.
#[async_trait::async_trait]
pub trait ModelQueries: Model {
    /// Find a single row by primary key.
    async fn find(db: &crate::core::orm::Database, id: serde_json::Value) -> crate::NoorResult<Option<serde_json::Value>> {
        let meta = Self::meta_static();
        let sql = format!(
            "SELECT * FROM {} WHERE {} = ? LIMIT 1",
            meta.table_name, meta.primary_key
        );
        db.query_first(&sql, &[id]).await
    }

    /// Return all rows (use with caution on large tables).
    async fn all(db: &crate::core::orm::Database) -> crate::NoorResult<Vec<serde_json::Value>> {
        let meta = Self::meta_static();
        let sql = format!("SELECT * FROM {}", meta.table_name);
        db.query(&sql, &[]).await
    }

    /// Return rows matching a simple `column = value` filter.
    async fn where_eq(
        db: &crate::core::orm::Database,
        column: &str,
        value: serde_json::Value,
    ) -> crate::NoorResult<Vec<serde_json::Value>> {
        let meta = Self::meta_static();
        let sql = format!("SELECT * FROM {} WHERE {} = ?", meta.table_name, column);
        db.query(&sql, &[value]).await
    }

    /// Count all rows.
    async fn count(db: &crate::core::orm::Database) -> crate::NoorResult<i64> {
        let meta = Self::meta_static();
        let sql = format!("SELECT COUNT(*) AS c FROM {}", meta.table_name);
        let row = db.query_first(&sql, &[]).await?;
        Ok(row.and_then(|r| r["c"].as_i64()).unwrap_or(0))
    }

    /// Insert a row from a JSON object (keys must match fillable columns).
    async fn insert(
        db: &crate::core::orm::Database,
        data: &serde_json::Map<String, serde_json::Value>,
    ) -> crate::NoorResult<u64> {
        let meta = Self::meta_static();
        let fillable: std::collections::HashSet<&str> =
            meta.fillable.iter().map(|s| s.as_str()).collect();

        let mut columns = Vec::new();
        let mut values = Vec::new();
        for (k, v) in data {
            if fillable.contains(k.as_str()) {
                columns.push(k.clone());
                values.push(v.clone());
            }
        }
        if columns.is_empty() {
            return Err(crate::NoorError::Database(
                "insert: no fillable columns in data".to_string(),
            ));
        }
        let placeholders: Vec<String> = (0..columns.len()).map(|_| "?".to_string()).collect();
        let sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            meta.table_name,
            columns.join(", "),
            placeholders.join(", ")
        );
        db.execute(&sql, &values).await
    }

    /// Update a row by primary key.
    async fn update_by_id(
        db: &crate::core::orm::Database,
        id: serde_json::Value,
        data: &serde_json::Map<String, serde_json::Value>,
    ) -> crate::NoorResult<u64> {
        let meta = Self::meta_static();
        let fillable: std::collections::HashSet<&str> =
            meta.fillable.iter().map(|s| s.as_str()).collect();

        let mut sets: Vec<String> = Vec::new();
        let mut params: Vec<serde_json::Value> = Vec::new();
        for (k, v) in data {
            if fillable.contains(k.as_str()) {
                sets.push(format!("{} = ?", k));
                params.push(v.clone());
            }
        }
        if sets.is_empty() {
            return Err(crate::NoorError::Database(
                "update: no fillable columns in data".to_string(),
            ));
        }
        params.push(id);
        let sql = format!(
            "UPDATE {} SET {} WHERE {} = ?",
            meta.table_name,
            sets.join(", "),
            meta.primary_key
        );
        db.execute(&sql, &params).await
    }

    /// Delete a row by primary key.
    async fn delete_by_id(
        db: &crate::core::orm::Database,
        id: serde_json::Value,
    ) -> crate::NoorResult<u64> {
        let meta = Self::meta_static();
        let sql = format!("DELETE FROM {} WHERE {} = ?", meta.table_name, meta.primary_key);
        db.execute(&sql, &[id]).await
    }

    /// Return the `ModelMeta` without an instance (used by the async helpers).
    fn meta_static() -> &'static ModelMeta;
}

/// Base model with common fields
/// نموذج أساسي بحقول مشتركة
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseModel {
    pub id: Option<i64>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl BaseModel {
    pub fn new() -> Self {
        Self {
            id: None,
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
        }
    }
}

impl Default for BaseModel {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to build model metadata
/// مساعد لبناء معلومات النموذج
pub struct ModelMetaBuilder {
    table_name: String,
    primary_key: String,
    fillable: Vec<String>,
    hidden: Vec<String>,
    casts: HashMap<String, CastType>,
    timestamps: bool,
}

impl ModelMetaBuilder {
    pub fn new(table_name: &str) -> Self {
        Self {
            table_name: table_name.to_string(),
            primary_key: "id".to_string(),
            fillable: Vec::new(),
            hidden: Vec::new(),
            casts: HashMap::new(),
            timestamps: true,
        }
    }
    
    pub fn primary_key(mut self, key: &str) -> Self {
        self.primary_key = key.to_string();
        self
    }
    
    pub fn fillable(mut self, columns: &[&str]) -> Self {
        self.fillable = columns.iter().map(|s| s.to_string()).collect();
        self
    }
    
    pub fn hidden(mut self, columns: &[&str]) -> Self {
        self.hidden = columns.iter().map(|s| s.to_string()).collect();
        self
    }
    
    pub fn cast(mut self, column: &str, cast_type: CastType) -> Self {
        self.casts.insert(column.to_string(), cast_type);
        self
    }
    
    pub fn timestamps(mut self, enabled: bool) -> Self {
        self.timestamps = enabled;
        self
    }
    
    pub fn build(self) -> ModelMeta {
        ModelMeta {
            table_name: self.table_name,
            primary_key: self.primary_key,
            fillable: self.fillable,
            hidden: self.hidden,
            casts: self.casts,
            timestamps: self.timestamps,
        }
    }
}
