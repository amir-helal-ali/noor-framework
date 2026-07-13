// ============================================================
// Advanced Query Builder - منشئ الاستعلامات المتقدم
// ============================================================
// Extended query builder with JOINs, subqueries, unions,
// aggregations, and raw expressions.
//
// منشئ استعلامات متقدم مع JOIN و subqueries و aggregations.
// ============================================================

use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// Advanced query builder extending the base QueryBuilder
#[derive(Debug, Clone)]
pub struct AdvancedQueryBuilder {
    table: String,
    columns: Vec<String>,
    where_clauses: Vec<AdvancedWhereClause>,
    joins: Vec<JoinClause>,
    order_by: Vec<(String, String)>,
    group_by: Vec<String>,
    having: Vec<AdvancedWhereClause>,
    limit: Option<u32>,
    offset: Option<u32>,
    unions: Vec<UnionQuery>,
    params: Vec<serde_json::Value>,
    /// Whether this is a DISTINCT query
    distinct: bool,
    /// Lock type (FOR UPDATE, FOR SHARE)
    lock: Option<LockType>,
}

#[derive(Debug, Clone)]
struct AdvancedWhereClause {
    column: String,
    operator: String,
    value: serde_json::Value,
    boolean: String,
    /// For nested WHERE groups
    nested: Option<Vec<AdvancedWhereClause>>,
}

#[derive(Debug, Clone)]
struct JoinClause {
    table: String,
    on: String,
    join_type: JoinType,
    /// Additional WHERE conditions for the JOIN
    where_clauses: Vec<AdvancedWhereClause>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

#[derive(Debug, Clone)]
struct UnionQuery {
    sql: String,
    params: Vec<serde_json::Value>,
    all: bool,  // UNION ALL vs UNION
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum LockType {
    ForUpdate,
    ForShare,
}

impl AdvancedQueryBuilder {
    pub fn table(table: &str) -> Self {
        Self {
            table: table.to_string(),
            columns: vec!["*".to_string()],
            where_clauses: Vec::new(),
            joins: Vec::new(),
            order_by: Vec::new(),
            group_by: Vec::new(),
            having: Vec::new(),
            limit: None,
            offset: None,
            unions: Vec::new(),
            params: Vec::new(),
            distinct: false,
            lock: None,
        }
    }
    
    /// SELECT DISTINCT
    pub fn distinct(mut self) -> Self {
        self.distinct = true;
        self
    }
    
    /// Select specific columns
    pub fn select(mut self, columns: &[&str]) -> Self {
        self.columns = columns.iter().map(|s| s.to_string()).collect();
        self
    }
    
    /// Select with alias
    pub fn select_as(mut self, column: &str, alias: &str) -> Self {
        self.columns.push(format!("{} AS {}", column, alias));
        self
    }
    
    /// Select raw expression
    pub fn select_raw(mut self, expression: &str) -> Self {
        self.columns.push(expression.to_string());
        self
    }
    
    /// INNER JOIN
    pub fn join(self, table: &str, on: &str) -> Self {
        self.add_join(table, on, JoinType::Inner)
    }
    
    /// LEFT JOIN
    pub fn left_join(self, table: &str, on: &str) -> Self {
        self.add_join(table, on, JoinType::Left)
    }
    
    /// RIGHT JOIN
    pub fn right_join(self, table: &str, on: &str) -> Self {
        self.add_join(table, on, JoinType::Right)
    }
    
    /// FULL OUTER JOIN
    pub fn full_join(self, table: &str, on: &str) -> Self {
        self.add_join(table, on, JoinType::Full)
    }
    
    /// CROSS JOIN
    pub fn cross_join(self, table: &str) -> Self {
        self.add_join(table, "1=1", JoinType::Cross)
    }
    
    fn add_join(mut self, table: &str, on: &str, join_type: JoinType) -> Self {
        self.joins.push(JoinClause {
            table: table.to_string(),
            on: on.to_string(),
            join_type,
            where_clauses: Vec::new(),
        });
        self
    }
    
    /// WHERE clause
    pub fn where_(mut self, column: &str, operator: &str, value: impl Into<serde_json::Value>) -> Self {
        let value = value.into();
        self.where_clauses.push(AdvancedWhereClause {
            column: column.to_string(),
            operator: operator.to_string(),
            value: value.clone(),
            boolean: "AND".to_string(),
            nested: None,
        });
        self.params.push(value);
        self
    }
    
    /// OR WHERE
    pub fn or_where(mut self, column: &str, operator: &str, value: impl Into<serde_json::Value>) -> Self {
        let value = value.into();
        self.where_clauses.push(AdvancedWhereClause {
            column: column.to_string(),
            operator: operator.to_string(),
            value: value.clone(),
            boolean: "OR".to_string(),
            nested: None,
        });
        self.params.push(value);
        self
    }
    
    /// WHERE IN
    pub fn where_in(mut self, column: &str, values: &[serde_json::Value]) -> Self {
        self.where_clauses.push(AdvancedWhereClause {
            column: column.to_string(),
            operator: format!("IN ({})", values.iter().enumerate().map(|(i, _)| format!("${}", self.params.len() + i + 1)).collect::<Vec<_>>().join(", ")),
            value: serde_json::Value::Null,
            boolean: "AND".to_string(),
            nested: None,
        });
        
        for v in values {
            self.params.push(v.clone());
        }
        
        self
    }
    
    /// WHERE NOT IN
    pub fn where_not_in(mut self, column: &str, values: &[serde_json::Value]) -> Self {
        self.where_clauses.push(AdvancedWhereClause {
            column: column.to_string(),
            operator: format!("NOT IN ({})", values.iter().enumerate().map(|(i, _)| format!("${}", self.params.len() + i + 1)).collect::<Vec<_>>().join(", ")),
            value: serde_json::Value::Null,
            boolean: "AND".to_string(),
            nested: None,
        });
        
        for v in values {
            self.params.push(v.clone());
        }
        
        self
    }
    
    /// WHERE NULL
    pub fn where_null(mut self, column: &str) -> Self {
        self.where_clauses.push(AdvancedWhereClause {
            column: column.to_string(),
            operator: "IS NULL".to_string(),
            value: serde_json::Value::Null,
            boolean: "AND".to_string(),
            nested: None,
        });
        self
    }
    
    /// WHERE NOT NULL
    pub fn where_not_null(mut self, column: &str) -> Self {
        self.where_clauses.push(AdvancedWhereClause {
            column: column.to_string(),
            operator: "IS NOT NULL".to_string(),
            value: serde_json::Value::Null,
            boolean: "AND".to_string(),
            nested: None,
        });
        self
    }
    
    /// WHERE BETWEEN
    pub fn where_between(mut self, column: &str, low: impl Into<serde_json::Value>, high: impl Into<serde_json::Value>) -> Self {
        let low = low.into();
        let high = high.into();
        
        self.where_clauses.push(AdvancedWhereClause {
            column: column.to_string(),
            operator: format!("BETWEEN ${} AND ${}", self.params.len() + 1, self.params.len() + 2),
            value: serde_json::Value::Null,
            boolean: "AND".to_string(),
            nested: None,
        });
        
        self.params.push(low);
        self.params.push(high);
        self
    }
    
    /// WHERE LIKE
    pub fn where_like(mut self, column: &str, pattern: &str) -> Self {
        self.where_clauses.push(AdvancedWhereClause {
            column: column.to_string(),
            operator: "LIKE".to_string(),
            value: serde_json::Value::String(pattern.to_string()),
            boolean: "AND".to_string(),
            nested: None,
        });
        self.params.push(serde_json::Value::String(pattern.to_string()));
        self
    }
    
    /// WHERE EXISTS (subquery)
    pub fn where_exists(mut self, subquery: &str) -> Self {
        self.where_clauses.push(AdvancedWhereClause {
            column: "EXISTS".to_string(),
            operator: format!("({})", subquery),
            value: serde_json::Value::Null,
            boolean: "AND".to_string(),
            nested: None,
        });
        self
    }
    
    /// WHERE column = (subquery)
    pub fn where_subquery(mut self, column: &str, operator: &str, subquery: &str) -> Self {
        self.where_clauses.push(AdvancedWhereClause {
            column: column.to_string(),
            operator: format!("{} ({})", operator, subquery),
            value: serde_json::Value::Null,
            boolean: "AND".to_string(),
            nested: None,
        });
        self
    }
    
    /// Raw WHERE clause
    pub fn where_raw(mut self, sql: &str, params: &[serde_json::Value]) -> Self {
        self.where_clauses.push(AdvancedWhereClause {
            column: sql.to_string(),
            operator: String::new(),
            value: serde_json::Value::Null,
            boolean: "AND".to_string(),
            nested: None,
        });
        
        for p in params {
            self.params.push(p.clone());
        }
        
        self
    }
    
    /// GROUP BY
    pub fn group_by(mut self, columns: &[&str]) -> Self {
        self.group_by = columns.iter().map(|s| s.to_string()).collect();
        self
    }
    
    /// HAVING
    pub fn having(mut self, column: &str, operator: &str, value: impl Into<serde_json::Value>) -> Self {
        let value = value.into();
        self.having.push(AdvancedWhereClause {
            column: column.to_string(),
            operator: operator.to_string(),
            value: value.clone(),
            boolean: "AND".to_string(),
            nested: None,
        });
        self.params.push(value);
        self
    }
    
    /// ORDER BY
    pub fn order_by(mut self, column: &str, direction: &str) -> Self {
        self.order_by.push((column.to_string(), direction.to_uppercase()));
        self
    }
    
    /// LIMIT
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }
    
    /// OFFSET
    pub fn offset(mut self, offset: u32) -> Self {
        self.offset = Some(offset);
        self
    }
    
    /// FOR UPDATE (pessimistic locking)
    pub fn for_update(mut self) -> Self {
        self.lock = Some(LockType::ForUpdate);
        self
    }
    
    /// FOR SHARE (shared locking)
    pub fn for_share(mut self) -> Self {
        self.lock = Some(LockType::ForShare);
        self
    }
    
    /// UNION
    pub fn union(mut self, sql: &str, params: Vec<serde_json::Value>) -> Self {
        self.unions.push(UnionQuery {
            sql: sql.to_string(),
            params,
            all: false,
        });
        self
    }
    
    /// UNION ALL
    pub fn union_all(mut self, sql: &str, params: Vec<serde_json::Value>) -> Self {
        self.unions.push(UnionQuery {
            sql: sql.to_string(),
            params,
            all: true,
        });
        self
    }
    
    /// Build the SQL query
    pub fn to_sql(&self) -> (String, Vec<serde_json::Value>) {
        let mut sql = String::new();
        
        // SELECT
        sql.push_str("SELECT ");
        
        if self.distinct {
            sql.push_str("DISTINCT ");
        }
        
        sql.push_str(&self.columns.join(", "));
        sql.push_str(&format!(" FROM {}", self.table));
        
        // JOINs
        for join in &self.joins {
            let join_type = match join.join_type {
                JoinType::Inner => "INNER JOIN",
                JoinType::Left => "LEFT JOIN",
                JoinType::Right => "RIGHT JOIN",
                JoinType::Full => "FULL OUTER JOIN",
                JoinType::Cross => "CROSS JOIN",
            };
            sql.push_str(&format!(" {} {} ON {}", join_type, join.table, join.on));
        }
        
        // WHERE
        if !self.where_clauses.is_empty() {
            sql.push_str(" WHERE ");
            let mut param_idx = 1;
            
            for (i, clause) in self.where_clauses.iter().enumerate() {
                if i > 0 {
                    sql.push_str(&format!(" {} ", clause.boolean));
                }
                
                if clause.operator.is_empty() {
                    // Raw SQL
                    sql.push_str(&clause.column);
                } else if clause.operator.starts_with("IN") || clause.operator.starts_with("NOT IN") {
                    sql.push_str(&format!("{} {}", clause.column, clause.operator));
                } else if clause.operator == "IS NULL" || clause.operator == "IS NOT NULL" {
                    sql.push_str(&format!("{} {}", clause.column, clause.operator));
                } else if clause.operator.starts_with("BETWEEN") {
                    sql.push_str(&format!("{} {}", clause.column, clause.operator));
                    param_idx += 2;
                } else if clause.column == "EXISTS" {
                    sql.push_str(&format!("EXISTS {}", clause.operator));
                } else if clause.operator.contains('(') {
                    // Subquery
                    sql.push_str(&format!("{} {}", clause.column, clause.operator));
                } else {
                    sql.push_str(&format!("{} {} ${}", clause.column, clause.operator, param_idx));
                    param_idx += 1;
                }
            }
        }
        
        // GROUP BY
        if !self.group_by.is_empty() {
            sql.push_str(&format!(" GROUP BY {}", self.group_by.join(", ")));
        }
        
        // HAVING
        if !self.having.is_empty() {
            sql.push_str(" HAVING ");
            let mut param_idx = self.params.len() - self.having.len() + 1;
            
            for (i, clause) in self.having.iter().enumerate() {
                if i > 0 {
                    sql.push_str(&format!(" {} ", clause.boolean));
                }
                sql.push_str(&format!("{} {} ${}", clause.column, clause.operator, param_idx));
                param_idx += 1;
            }
        }
        
        // ORDER BY
        if !self.order_by.is_empty() {
            let order_str: Vec<String> = self.order_by
                .iter()
                .map(|(col, dir)| format!("{} {}", col, dir))
                .collect();
            sql.push_str(&format!(" ORDER BY {}", order_str.join(", ")));
        }
        
        // LIMIT
        if let Some(limit) = self.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }
        
        // OFFSET
        if let Some(offset) = self.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }
        
        // LOCK
        if let Some(lock) = self.lock {
            match lock {
                LockType::ForUpdate => sql.push_str(" FOR UPDATE"),
                LockType::ForShare => sql.push_str(" FOR SHARE"),
            }
        }
        
        // UNION
        for union in &self.unions {
            let union_type = if union.all { "UNION ALL" } else { "UNION" };
            sql.push_str(&format!(" {} {}", union_type, union.sql));
        }
        
        (sql, self.params.clone())
    }
    
    /// Build a COUNT query
    pub fn count_sql(&self) -> (String, Vec<serde_json::Value>) {
        let mut clone = self.clone();
        clone.columns = vec!["COUNT(*) as count".to_string()];
        clone.order_by.clear();
        clone.limit = None;
        clone.offset = None;
        clone.to_sql()
    }
    
    /// Build an aggregation query
    pub fn aggregate(&self, function: &str, column: &str) -> (String, Vec<serde_json::Value>) {
        let mut clone = self.clone();
        clone.columns = vec![format!("{}({}) as aggregate", function, column)];
        clone.order_by.clear();
        clone.limit = None;
        clone.offset = None;
        clone.to_sql()
    }
    
    /// SUM
    pub fn sum(&self, column: &str) -> (String, Vec<serde_json::Value>) {
        self.aggregate("SUM", column)
    }
    
    /// AVG
    pub fn avg(&self, column: &str) -> (String, Vec<serde_json::Value>) {
        self.aggregate("AVG", column)
    }
    
    /// MIN
    pub fn min(&self, column: &str) -> (String, Vec<serde_json::Value>) {
        self.aggregate("MIN", column)
    }
    
    /// MAX
    pub fn max(&self, column: &str) -> (String, Vec<serde_json::Value>) {
        self.aggregate("MAX", column)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_select_with_join() {
        let (sql, _params) = AdvancedQueryBuilder::table("posts")
            .select(&["posts.id", "posts.title", "users.name"])
            .join("users", "users.id = posts.author_id")
            .where_("posts.status", "=", "published")
            .order_by("posts.created_at", "desc")
            .limit(10)
            .to_sql();
        
        assert!(sql.contains("SELECT posts.id, posts.title, users.name"));
        assert!(sql.contains("FROM posts"));
        assert!(sql.contains("INNER JOIN users"));
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("ORDER BY"));
        assert!(sql.contains("LIMIT 10"));
    }
    
    #[test]
    fn test_where_in() {
        let (sql, params) = AdvancedQueryBuilder::table("users")
            .where_in("role", &[
                serde_json::json!("admin"),
                serde_json::json!("editor"),
            ])
            .to_sql();
        
        assert!(sql.contains("IN"));
        assert_eq!(params.len(), 2);
    }
    
    #[test]
    fn test_where_between() {
        let (sql, _params) = AdvancedQueryBuilder::table("products")
            .where_between("price", 10, 100)
            .to_sql();
        
        assert!(sql.contains("BETWEEN"));
    }
    
    #[test]
    fn test_where_like() {
        let (sql, _params) = AdvancedQueryBuilder::table("users")
            .where_like("name", "%john%")
            .to_sql();
        
        assert!(sql.contains("LIKE"));
    }
    
    #[test]
    fn test_group_by_having() {
        let (sql, _params) = AdvancedQueryBuilder::table("orders")
            .select(&["user_id", "COUNT(*) as order_count"])
            .group_by(&["user_id"])
            .having("order_count", ">", 5)
            .to_sql();
        
        assert!(sql.contains("GROUP BY"));
        assert!(sql.contains("HAVING"));
    }
    
    #[test]
    fn test_distinct() {
        let (sql, _params) = AdvancedQueryBuilder::table("orders")
            .distinct()
            .select(&["user_id"])
            .to_sql();
        
        assert!(sql.contains("DISTINCT"));
    }
    
    #[test]
    fn test_for_update() {
        let (sql, _params) = AdvancedQueryBuilder::table("accounts")
            .where_("id", "=", 1)
            .for_update()
            .to_sql();
        
        assert!(sql.contains("FOR UPDATE"));
    }
    
    #[test]
    fn test_union() {
        let (sql, _params) = AdvancedQueryBuilder::table("active_users")
            .select(&["id", "name"])
            .union("SELECT id, name FROM pending_users", vec![])
            .to_sql();
        
        assert!(sql.contains("UNION"));
        assert!(sql.contains("pending_users"));
    }
    
    #[test]
    fn test_aggregate() {
        let (sql, _params) = AdvancedQueryBuilder::table("orders")
            .where_("status", "=", "completed")
            .sum("total");
        
        assert!(sql.contains("SUM(total)"));
    }
    
    #[test]
    fn test_count() {
        let (sql, _params) = AdvancedQueryBuilder::table("users")
            .where_("active", "=", true)
            .count_sql();
        
        assert!(sql.contains("COUNT(*)"));
    }
    
    #[test]
    fn test_subquery() {
        let (sql, _params) = AdvancedQueryBuilder::table("users")
            .where_subquery("id", "IN", "SELECT user_id FROM orders WHERE total > 1000")
            .to_sql();
        
        assert!(sql.contains("SELECT user_id FROM orders"));
    }
}
