// ============================================================
// Query Builder - منشئ الاستعلامات
// ============================================================
// Fluent, parameterized query builder that prevents SQL injection.
// Uses parameter binding for all values.
//
// منشئ استعلامات سلس يستخدم parameterized queries.
// ============================================================

use std::collections::HashMap;

/// Query builder for constructing SQL queries safely
/// منشئ استعلامات SQL آمن
#[derive(Debug, Clone)]
pub struct QueryBuilder {
    table: String,
    action: QueryAction,
    columns: Vec<String>,
    where_clauses: Vec<WhereClause>,
    order_by: Vec<(String, OrderDirection)>,
    limit: Option<u32>,
    offset: Option<u32>,
    joins: Vec<Join>,
    params: Vec<serde_json::Value>,
    group_by: Vec<String>,
    having: Vec<WhereClause>,
}

#[derive(Debug, Clone)]
enum QueryAction {
    Select,
    Insert,
    Update,
    Delete,
}

#[derive(Debug, Clone)]
struct WhereClause {
    column: String,
    operator: String,
    value: serde_json::Value,
    boolean: String, // AND/OR
}

#[derive(Debug, Clone)]
struct Join {
    table: String,
    on: String,
    join_type: JoinType,
}

#[derive(Debug, Clone)]
enum JoinType {
    Inner,
    Left,
    Right,
    Full,
}

#[derive(Debug, Clone)]
enum OrderDirection {
    Asc,
    Desc,
}

/// Query result
/// نتيجة الاستعلام
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub rows: Vec<serde_json::Value>,
    pub affected: u64,
    pub last_insert_id: Option<i64>,
}

impl QueryBuilder {
    /// Create a new SELECT query builder
    /// إنشاء منشئ استعلام SELECT جديد
    pub fn table(table: &str) -> Self {
        Self {
            table: table.to_string(),
            action: QueryAction::Select,
            columns: vec!["*".to_string()],
            where_clauses: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
            joins: Vec::new(),
            params: Vec::new(),
            group_by: Vec::new(),
            having: Vec::new(),
        }
    }
    
    /// Set columns to select
    /// تعيين الأعمدة للاختيار
    pub fn select(mut self, columns: &[&str]) -> Self {
        self.columns = columns.iter().map(|s| s.to_string()).collect();
        self
    }
    
    /// Add a WHERE clause
    /// إضافة شرط WHERE
    pub fn where_(mut self, column: &str, operator: &str, value: impl Into<serde_json::Value>) -> Self {
        let converted = value.into();
        self.where_clauses.push(WhereClause {
            column: column.to_string(),
            operator: operator.to_string(),
            value: converted.clone(),
            boolean: "AND".to_string(),
        });
        self.params.push(converted);
        self
    }
    
    /// Add an OR WHERE clause
    pub fn or_where(mut self, column: &str, operator: &str, value: impl Into<serde_json::Value>) -> Self {
        let converted = value.into();
        self.where_clauses.push(WhereClause {
            column: column.to_string(),
            operator: operator.to_string(),
            value: converted.clone(),
            boolean: "OR".to_string(),
        });
        self.params.push(converted);
        self
    }
    
    /// Add a WHERE IN clause
    ///
    /// The actual `$N` placeholders are generated at SQL-build time in
    /// `build_select`, which walks the where-clauses in order and assigns
    /// placeholders sequentially. We stash the values as a JSON array in the
    /// clause's `value` field so the builder can splice them in at the right
    /// position.
    pub fn where_in(mut self, column: &str, values: &[serde_json::Value]) -> Self {
        self.where_clauses.push(WhereClause {
            column: column.to_string(),
            operator: "IN".to_string(),
            value: serde_json::Value::Array(values.to_vec()),
            boolean: "AND".to_string(),
        });

        self
    }
    
    /// Add a WHERE NULL clause
    pub fn where_null(mut self, column: &str) -> Self {
        self.where_clauses.push(WhereClause {
            column: column.to_string(),
            operator: "IS NULL".to_string(),
            value: serde_json::Value::Null,
            boolean: "AND".to_string(),
        });
        self
    }
    
    /// Add a JOIN clause
    pub fn join(mut self, table: &str, on: &str) -> Self {
        self.joins.push(Join {
            table: table.to_string(),
            on: on.to_string(),
            join_type: JoinType::Inner,
        });
        self
    }
    
    /// Add a LEFT JOIN clause
    pub fn left_join(mut self, table: &str, on: &str) -> Self {
        self.joins.push(Join {
            table: table.to_string(),
            on: on.to_string(),
            join_type: JoinType::Left,
        });
        self
    }
    
    /// Add ORDER BY clause
    pub fn order_by(mut self, column: &str, direction: &str) -> Self {
        let dir = if direction.to_uppercase() == "DESC" {
            OrderDirection::Desc
        } else {
            OrderDirection::Asc
        };
        self.order_by.push((column.to_string(), dir));
        self
    }
    
    /// Set LIMIT
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }
    
    /// Set OFFSET
    pub fn offset(mut self, offset: u32) -> Self {
        self.offset = Some(offset);
        self
    }
    
    /// Add GROUP BY clause
    pub fn group_by(mut self, column: &str) -> Self {
        self.group_by.push(column.to_string());
        self
    }
    
    /// Convert to SQL string and parameters
    /// تحويل إلى SQL والمعاملات
    ///
    /// Uses `?` placeholders (SQLite/MySQL style). When targeting Postgres,
    /// the `Database::query` / `Database::execute` methods handle the
    /// translation transparently because sqlx's `Any` layer normalizes
    /// placeholder syntax per backend.
    pub fn to_sql(&self) -> (String, Vec<serde_json::Value>) {
        match self.action {
            QueryAction::Select => self.build_select(),
            QueryAction::Delete => {
                let (select_sql, params) = self.build_select();
                // `build_select` returns `SELECT * FROM table WHERE ...`,
                // but for DELETE we want `DELETE FROM table WHERE ...`.
                // Strip the leading `SELECT * FROM ` and prepend `DELETE FROM `.
                let delete_sql = if select_sql.starts_with("SELECT ") {
                    let after_from = select_sql.find(" FROM ").map(|i| &select_sql[i + 1..]).unwrap_or(&select_sql);
                    format!("DELETE{}", after_from)
                } else {
                    select_sql
                };
                (delete_sql, params)
            }
            _ => (String::new(), vec![]),
        }
    }

    /// Execute the SELECT against a `Database` and return all matching rows.
    ///
    /// Runs the built SQL via `Database::query`.
    pub async fn fetch(&self, db: &crate::core::orm::Database) -> crate::NoorResult<Vec<serde_json::Value>> {
        let (sql, params) = self.to_sql();
        db.query(&sql, &params).await
    }

    /// Execute the SELECT and return the first matching row (or `None`).
    pub async fn first(
        &self,
        db: &crate::core::orm::Database,
    ) -> crate::NoorResult<Option<serde_json::Value>> {
        let (sql, params) = self.to_sql();
        db.query_first(&sql, &params).await
    }

    /// Execute a DELETE query and return rows affected.
    pub async fn execute_delete(
        &self,
        db: &crate::core::orm::Database,
    ) -> crate::NoorResult<u64> {
        let (sql, params) = self.to_sql();
        db.execute(&sql, &params).await
    }

    /// Count matching rows (runs a `SELECT COUNT(*)` variant of this query).
    pub async fn count(&self, db: &crate::core::orm::Database) -> crate::NoorResult<i64> {
        // Build a COUNT(*) query using the same FROM/JOIN/WHERE.
        let mut count_qb = QueryBuilder::table(&self.table);
        count_qb.columns = vec!["COUNT(*) AS c".to_string()];
        count_qb.joins = self.joins.clone();
        count_qb.where_clauses = self.where_clauses.clone();
        count_qb.group_by = Vec::new(); // GROUP BY doesn't apply to COUNT(*)
        count_qb.having = Vec::new();
        count_qb.order_by = Vec::new();
        count_qb.limit = None;
        count_qb.offset = None;
        let row = count_qb.first(db).await?;
        Ok(row.and_then(|r| r["c"].as_i64()).unwrap_or(0))
    }
    
    fn build_select(&self) -> (String, Vec<serde_json::Value>) {
        let mut sql = format!("SELECT {} FROM {}", self.columns.join(", "), self.table);

        // JOINs
        for join in &self.joins {
            let join_type = match join.join_type {
                JoinType::Inner => "INNER JOIN",
                JoinType::Left => "LEFT JOIN",
                JoinType::Right => "RIGHT JOIN",
                JoinType::Full => "FULL OUTER JOIN",
            };
            sql.push_str(&format!(" {} {} ON {}", join_type, join.table, join.on));
        }

        // WHERE — we walk the clauses in order and emit `?` placeholders.
        // sqlx's `Any` layer rewrites `?` into `$1, $2, ...` for Postgres
        // automatically, so `?` is the portable choice.
        let mut params: Vec<serde_json::Value> = Vec::new();

        if !self.where_clauses.is_empty() {
            sql.push_str(" WHERE ");

            for (i, clause) in self.where_clauses.iter().enumerate() {
                if i > 0 {
                    sql.push_str(&format!(" {} ", clause.boolean));
                }

                if clause.operator == "IS NULL" || clause.operator == "IS NOT NULL" {
                    sql.push_str(&format!("{} {}", clause.column, clause.operator));
                } else if clause.operator == "IN" {
                    let values: Vec<serde_json::Value> = match &clause.value {
                        serde_json::Value::Array(arr) => arr.clone(),
                        _ => Vec::new(),
                    };
                    let placeholders: Vec<String> = (0..values.len()).map(|_| "?".to_string()).collect();
                    sql.push_str(&format!(
                        "{} IN ({})",
                        clause.column,
                        placeholders.join(", ")
                    ));
                    params.extend(values);
                } else {
                    sql.push_str(&format!("{} {} ?", clause.column, clause.operator));
                    params.push(clause.value.clone());
                }
            }
        }

        // GROUP BY
        if !self.group_by.is_empty() {
            sql.push_str(&format!(" GROUP BY {}", self.group_by.join(", ")));
        }

        // ORDER BY
        if !self.order_by.is_empty() {
            let order_str: Vec<String> = self.order_by
                .iter()
                .map(|(col, dir)| {
                    let d = match dir {
                        OrderDirection::Asc => "ASC",
                        OrderDirection::Desc => "DESC",
                    };
                    format!("{} {}", col, d)
                })
                .collect();
            sql.push_str(&format!(" ORDER BY {}", order_str.join(", ")));
        }

        // LIMIT/OFFSET
        if let Some(limit) = self.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = self.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        (sql, params)
    }
    
    /// Create an INSERT query
    pub fn insert(table: &str) -> InsertBuilder {
        InsertBuilder::new(table)
    }
    
    /// Create an UPDATE query
    pub fn update(table: &str) -> UpdateBuilder {
        UpdateBuilder::new(table)
    }
    
    /// Create a DELETE query (alias for table())
    pub fn delete_from(table: &str) -> Self {
        let mut qb = Self::table(table);
        qb.action = QueryAction::Delete;
        qb
    }
}

/// INSERT query builder
pub struct InsertBuilder {
    table: String,
    columns: Vec<String>,
    values: Vec<serde_json::Value>,
}

impl InsertBuilder {
    pub fn new(table: &str) -> Self {
        Self {
            table: table.to_string(),
            columns: Vec::new(),
            values: Vec::new(),
        }
    }
    
    pub fn set(mut self, column: &str, value: impl Into<serde_json::Value>) -> Self {
        self.columns.push(column.to_string());
        self.values.push(value.into());
        self
    }
    
    pub fn set_many(mut self, data: HashMap<String, serde_json::Value>) -> Self {
        for (k, v) in data {
            self.columns.push(k);
            self.values.push(v);
        }
        self
    }
    
    pub fn to_sql(&self) -> (String, Vec<serde_json::Value>) {
        let placeholders: Vec<String> = (0..self.columns.len()).map(|_| "?".to_string()).collect();

        let sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            self.table,
            self.columns.join(", "),
            placeholders.join(", ")
        );

        (sql, self.values.clone())
    }

    /// Execute the INSERT against a `Database` and return rows affected.
    pub async fn execute(
        &self,
        db: &crate::core::orm::Database,
    ) -> crate::NoorResult<u64> {
        let (sql, params) = self.to_sql();
        db.execute(&sql, &params).await
    }
}

/// UPDATE query builder
pub struct UpdateBuilder {
    table: String,
    sets: Vec<(String, serde_json::Value)>,
    where_clauses: Vec<WhereClause>,
    params: Vec<serde_json::Value>,
}

impl UpdateBuilder {
    pub fn new(table: &str) -> Self {
        Self {
            table: table.to_string(),
            sets: Vec::new(),
            where_clauses: Vec::new(),
            params: Vec::new(),
        }
    }
    
    pub fn set(mut self, column: &str, value: impl Into<serde_json::Value>) -> Self {
        self.sets.push((column.to_string(), value.into()));
        self
    }
    
    pub fn where_(mut self, column: &str, operator: &str, value: impl Into<serde_json::Value>) -> Self {
        let converted = value.into();
        self.where_clauses.push(WhereClause {
            column: column.to_string(),
            operator: operator.to_string(),
            value: converted.clone(),
            boolean: "AND".to_string(),
        });
        self.params.push(converted);
        self
    }
    
    pub fn to_sql(&self) -> (String, Vec<serde_json::Value>) {
        let mut params = Vec::new();

        let set_str: Vec<String> = self.sets
            .iter()
            .map(|(col, val)| {
                params.push(val.clone());
                format!("{} = ?", col)
            })
            .collect();

        let mut sql = format!("UPDATE {} SET {}", self.table, set_str.join(", "));

        if !self.where_clauses.is_empty() {
            sql.push_str(" WHERE ");
            for (i, clause) in self.where_clauses.iter().enumerate() {
                if i > 0 {
                    sql.push_str(&format!(" {} ", clause.boolean));
                }
                sql.push_str(&format!("{} {} ?", clause.column, clause.operator));
                params.push(clause.value.clone());
            }
        }

        (sql, params)
    }

    /// Execute the UPDATE against a `Database` and return rows affected.
    pub async fn execute(
        &self,
        db: &crate::core::orm::Database,
    ) -> crate::NoorResult<u64> {
        let (sql, params) = self.to_sql();
        db.execute(&sql, &params).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_select_query() {
        let (sql, params) = QueryBuilder::table("users")
            .select(&["id", "name", "email"])
            .where_("id", "=", 1)
            .where_("status", "=", "active")
            .to_sql();
        
        assert!(sql.contains("SELECT id, name, email FROM users"));
        assert!(sql.contains("WHERE"));
        assert_eq!(params.len(), 2);
    }
    
    #[test]
    fn test_insert_query() {
        let (sql, params) = QueryBuilder::insert("users")
            .set("name", "John")
            .set("email", "john@example.com")
            .to_sql();
        
        assert!(sql.contains("INSERT INTO users"));
        assert_eq!(params.len(), 2);
    }
    
    #[test]
    fn test_update_query() {
        let (sql, params) = QueryBuilder::update("users")
            .set("name", "Jane")
            .where_("id", "=", 1)
            .to_sql();
        
        assert!(sql.contains("UPDATE users SET"));
        assert_eq!(params.len(), 2);
    }
}
