// ============================================================
// JSON → sqlx parameter binding
// ============================================================
// sqlx's `Query::bind` accepts a single concrete `Encode` type per call, but
// the framework's `Database::execute` / `query` APIs accept `&[serde_json::Value]`.
// This module bridges that gap by inspecting each `serde_json::Value` and
// dispatching to the appropriate `bind::<T>()` call.
//
// Lifetime: every bind takes ownership of the query and returns the new
// query, mirroring sqlx's builder pattern.
// ============================================================

use sqlx::any::{Any, AnyArguments};
use sqlx::query::Query;

/// Helper trait that lets us write `JsonToSql::bind(query, value)` for any
/// `serde_json::Value` and have it dispatched to the right underlying type.
pub trait JsonToSql<'q> {
    fn bind(self, value: &serde_json::Value) -> Query<'q, Any, AnyArguments<'q>>;
}

impl<'q> JsonToSql<'q> for Query<'q, Any, AnyArguments<'q>> {
    fn bind(mut self, value: &serde_json::Value) -> Query<'q, Any, AnyArguments<'q>> {
        match value {
            serde_json::Value::Null => {
                self = self.bind(None::<String>);
            }
            serde_json::Value::Bool(b) => {
                self = self.bind(*b);
            }
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    self = self.bind(i);
                } else if let Some(u) = n.as_u64() {
                    self = self.bind(u as i64);
                } else if let Some(f) = n.as_f64() {
                    self = self.bind(f);
                }
            }
            serde_json::Value::String(s) => {
                self = self.bind(s.clone());
            }
            serde_json::Value::Array(arr) => {
                // Bind the array as a JSON string so the receiving column
                // can store it in a JSON/TEXT column. This is a reasonable
                // cross-database default; callers needing array-binding
                // semantics on Postgres can use a direct sqlx query.
                self = self.bind(serde_json::to_string(arr).unwrap_or_default());
            }
            serde_json::Value::Object(obj) => {
                self = self.bind(serde_json::to_string(obj).unwrap_or_default());
            }
        }
        self
    }
}
