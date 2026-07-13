// ============================================================
// Database Connection Manager
// مدير اتصال قاعدة البيانات
// ============================================================
// Real database implementation built on top of sqlx's `Any` driver, which
// lets the same code path talk to SQLite, PostgreSQL, or MySQL depending on
// the connection URL.
//
// Provides:
// - async `execute` / `query` / `query_first` with JSON-typed parameters
// - real transactions (BEGIN / COMMIT / ROLLBACK)
// - `table_exists` via parameterized information-schema queries
//
// The previous implementation returned mock values (Ok(1) / Ok(vec![]));
// this one actually talks to a database.
// ============================================================

use std::sync::Arc;

use sqlx::any::{
    install_default_drivers, Any, AnyArguments, AnyPoolOptions, AnyRow,
};
use sqlx::Column;
use sqlx::Pool;
use sqlx::Row as _;

use crate::core::orm::json_bind::JsonToSql;

/// Lazy-initialized driver installer. `install_default_drivers()` is idempotent
/// but must run exactly once before the first `AnyPool::connect` call.
static DRIVERS_INSTALLED: std::sync::Once = std::sync::Once::new();

fn ensure_drivers() {
    DRIVERS_INSTALLED.call_once(|| {
        install_default_drivers();
    });
}

/// Type alias for the Any-typed connection pool. `sqlx::any::AnyPool` lives
/// behind a `reexports` submodule in sqlx 0.7, so we reach it via `Pool<Any>`.
type AnyPool = Pool<Any>;

/// Run a closure on a freshly spawned OS thread and return its result.
///
/// This is used by `execute_blocking` / `query_blocking` so that sync code
/// paths (e.g. the framework's sync `Handler` trait) can call into the
/// async sqlx pool without deadlocking the surrounding tokio runtime:
/// `Handle::current().block_on(...)` panics inside a runtime, but a brand
/// new thread has no runtime context and is free to create its own.
fn run_in_blocking_thread<F, T>(f: F) -> crate::NoorResult<T>
where
    F: FnOnce() -> crate::NoorResult<T> + Send + 'static,
    T: Send + 'static,
{
    let (tx, rx) = std::sync::mpsc::channel::<crate::NoorResult<T>>();
    std::thread::spawn(move || {
        let result = f();
        let _ = tx.send(result);
    });
    rx.recv().map_err(|e| {
        crate::NoorError::Database(format!("blocking thread disappeared: {}", e))
    })?
}

/// Database connection (backed by an sqlx `AnyPool`).
///
/// اتصال قاعدة البيانات
pub struct Database {
    pub driver: String,
    pub url: String,
    pub pool_size: u32,
    pool: AnyPool,
    /// `AnyPool::begin()` borrows the pool for the lifetime of the returned
    /// transaction. To give callers an *owned* `Transaction` (so they can
    /// move it around freely), we wrap the pool in an `Arc` and let the
    /// transaction hold its own clone.
    _pool_arc: Arc<()>,
}

impl Database {
    /// Create a new database connection.
    ///
    /// `driver` must be one of: `"sqlite"`, `"postgres"`, `"mysql"`.
    /// `url` is the standard connection URL for that driver, e.g.
    ///   - `sqlite::memory:` or `sqlite:./storage/noor.db`
    ///   - `postgres://user:pass@host:5432/db`
    ///   - `mysql://user:pass@host:3306/db`
    ///
    /// If the driver is `"sqlite"`, the URL is normalized so callers may pass
    /// either `sqlite://path` (which sqlx does NOT accept) or
    /// `sqlite:path` / `sqlite::memory:`.
    pub async fn new(driver: &str, url: &str) -> crate::NoorResult<Self> {
        Self::with_pool_size(driver, url, 10).await
    }

    /// Create a connection with a custom maximum pool size.
    pub async fn with_pool_size(driver: &str, url: &str, pool_size: u32) -> crate::NoorResult<Self> {
        ensure_drivers();
        let normalized_url = normalize_url(driver, url);

        let pool = AnyPoolOptions::new()
            .max_connections(pool_size)
            .connect(&normalized_url)
            .await
            .map_err(|e| {
                crate::NoorError::Database(format!(
                    "Failed to connect to {} at {}: {}",
                    driver, normalized_url, e
                ))
            })?;

        Ok(Self {
            driver: driver.to_string(),
            url: url.to_string(),
            pool_size,
            pool,
            _pool_arc: Arc::new(()),
        })
    }

    /// Build a Database from the framework's `DatabaseConfig`.
    pub async fn from_config(
        cfg: &crate::core::config::DatabaseConfig,
    ) -> crate::NoorResult<Self> {
        let driver = match cfg.driver {
            crate::core::config::DatabaseDriver::Sqlite => "sqlite",
            crate::core::config::DatabaseDriver::Postgres => "postgres",
            crate::core::config::DatabaseDriver::Mysql => "mysql",
        };
        Self::with_pool_size(driver, &cfg.url, cfg.max_connections).await
    }

    /// Returns the kind of backend in use as a lowercase string
    /// (`"sqlite"`, `"postgres"`, `"mysql"`). Useful when you need to
    /// branch on driver-specific SQL at runtime.
    pub fn backend_kind(&self) -> &str {
        self.driver.as_str()
    }

    /// Execute a non-row-returning statement (INSERT/UPDATE/DELETE/DDL).
    ///
    /// Returns the number of rows affected.
    pub async fn execute(
        &self,
        sql: &str,
        params: &[serde_json::Value],
    ) -> crate::NoorResult<u64> {
        let mut query = sqlx::query::<Any>(sql);
        for p in params {
            query = JsonToSql::bind(query, p);
        }
        let result = query
            .execute(&self.pool)
            .await
            .map_err(|e| crate::NoorError::Database(format!("Execute error: {}", e)))?;
        Ok(result.rows_affected())
    }

    /// Run a SELECT (or any row-returning) statement and convert each row to
    /// a `serde_json::Value` object keyed by column name.
    pub async fn query(
        &self,
        sql: &str,
        params: &[serde_json::Value],
    ) -> crate::NoorResult<Vec<serde_json::Value>> {
        let mut query = sqlx::query::<Any>(sql);
        for p in params {
            query = JsonToSql::bind(query, p);
        }
        let rows = query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| crate::NoorError::Database(format!("Query error: {}", e)))?;

        Ok(rows.iter().map(row_to_json).collect())
    }

    /// Run a query and return the first row (if any).
    pub async fn query_first(
        &self,
        sql: &str,
        params: &[serde_json::Value],
    ) -> crate::NoorResult<Option<serde_json::Value>> {
        let rows = self.query(sql, params).await?;
        Ok(rows.into_iter().next())
    }

    /// Begin a real database transaction.
    pub async fn begin_transaction(&self) -> crate::NoorResult<Transaction> {
        let tx = self
            .pool
            .begin()
            .await
            .map_err(|e| crate::NoorError::Database(format!("BEGIN error: {}", e)))?;
        Ok(Transaction { tx })
    }

    /// Check whether a table exists, using a parameterized information-schema
    /// query (so the table name is never interpolated into SQL).
    pub async fn table_exists(&self, table: &str) -> crate::NoorResult<bool> {
        let sql = match self.driver.as_str() {
            "sqlite" => "SELECT name FROM sqlite_master WHERE type='table' AND name = ?",
            "postgres" => "SELECT table_name FROM information_schema.tables WHERE table_name = $1",
            "mysql" => "SELECT table_name FROM information_schema.tables WHERE table_schema = DATABASE() AND table_name = ?",
            _ => return Ok(false),
        };
        let result = self
            .query_first(sql, &[serde_json::Value::String(table.to_string())])
            .await?;
        Ok(result.is_some())
    }

    /// Synchronous wrapper around `execute()` for use from sync handler
    /// contexts. Spawns a dedicated thread that drives the async pool call
    /// to completion, so it is safe to call from inside a tokio runtime
    /// (unlike `Handle::current().block_on(...)` which would deadlock).
    pub fn execute_blocking(
        &self,
        sql: &str,
        params: &[serde_json::Value],
    ) -> crate::NoorResult<u64> {
        let pool = self.pool.clone();
        let sql = sql.to_string();
        let params = params.to_vec();
        run_in_blocking_thread(move || {
            let mut query = sqlx::query::<Any>(&sql);
            for p in &params {
                query = JsonToSql::bind(query, p);
            }
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| {
                    crate::NoorError::Database(format!("blocking runtime error: {}", e))
                })?;
            let result = rt
                .block_on(async move { query.execute(&pool).await })
                .map_err(|e| crate::NoorError::Database(format!("Execute error: {}", e)))?;
            Ok(result.rows_affected())
        })
    }

    /// Synchronous wrapper around `query()`. See `execute_blocking` for
    /// rationale.
    pub fn query_blocking(
        &self,
        sql: &str,
        params: &[serde_json::Value],
    ) -> crate::NoorResult<Vec<serde_json::Value>> {
        let pool = self.pool.clone();
        let sql = sql.to_string();
        let params = params.to_vec();
        run_in_blocking_thread(move || {
            let mut query = sqlx::query::<Any>(&sql);
            for p in &params {
                query = JsonToSql::bind(query, p);
            }
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| {
                    crate::NoorError::Database(format!("blocking runtime error: {}", e))
                })?;
            let rows = rt
                .block_on(async move { query.fetch_all(&pool).await })
                .map_err(|e| crate::NoorError::Database(format!("Query error: {}", e)))?;
            Ok(rows.iter().map(row_to_json).collect())
        })
    }
}

/// Convert an sqlx `AnyRow` into a JSON object keyed by column name.
///
/// Each column is decoded to a best-effort JSON value based on its declared
/// type. NULLs become `Value::Null`; integers, floats, booleans, strings,
/// bytes, and chrono datetimes are all handled.
fn row_to_json(row: &AnyRow) -> serde_json::Value {
    use serde_json::json;
    let mut obj = serde_json::Map::new();
    for col in row.columns() {
        let name = col.name();
        // Try a sequence of common types. The first one that decodes wins.
        let json_val = try_decode(row, name);
        obj.insert(name.to_string(), json_val);
    }
    json!(obj)
}

/// Try a series of types in order of likelihood for a generic SQL column.
///
/// sqlx's `Any` backend only supports a small set of concrete Rust types for
/// decoding (String, i64, f64, bool, Vec<u8>). Date/time types are NOT
/// implement `Decode<Any>` in sqlx 0.7 — they come back as strings, which
/// callers can then parse with chrono if needed.
fn try_decode(row: &AnyRow, name: &str) -> serde_json::Value {
    macro_rules! try_type {
        ($t:ty) => {
            if let Ok(v) = row.try_get::<$t, _>(name) {
                return serde_json::json!(v);
            }
        };
    }
    try_type!(String);
    try_type!(i64);
    try_type!(f64);
    try_type!(bool);
    try_type!(Vec<u8>);
    serde_json::Value::Null
}

/// Normalize a connection URL so that sqlx's `AnyPool` accepts it.
///
/// sqlx is strict about URL schemes:
///   - It accepts `sqlite:path`, `sqlite::memory:`, but NOT `sqlite://path`.
///   - It accepts `postgres://...` and `mysql://...`.
///
/// This function turns the framework's common (but sqlx-incompatible)
/// `sqlite://path` and `sqlite:./path` shapes into the canonical
/// `sqlite:path` / `sqlite::memory:` shapes.
fn normalize_url(driver: &str, url: &str) -> String {
    if driver == "sqlite" {
        // sqlx accepts:
        //   `sqlite::memory:`         — in-memory
        //   `sqlite:relative.db`      — relative path
        //   `sqlite:///absolute/path` — absolute path (triple slash)
        //
        // The framework also accepts `sqlite://path` (double slash) which
        // sqlx does NOT accept, so we normalize it.

        // Handle `sqlite://` prefix → convert to proper sqlx format.
        if let Some(rest) = url.strip_prefix("sqlite://") {
            if rest.is_empty() || rest == ":memory:" {
                return "sqlite::memory:".to_string();
            }
            // If rest starts with `/`, it's an absolute path → use triple slash.
            // Otherwise it's a relative path → use single `sqlite:` prefix.
            if rest.starts_with('/') {
                return format!("sqlite://{}", rest);
            }
            return format!("sqlite:{}", rest);
        }
        // `sqlite::memory:` and `sqlite:path` are passed through unchanged.
        return url.to_string();
    }
    url.to_string()
}

/// A real database transaction backed by sqlx.
///
/// Holds an owned sqlx transaction. The transaction is committed via
/// `commit()` or rolled back via `rollback()`; if neither is called and the
/// `Transaction` is dropped, sqlx rolls back automatically.
pub struct Transaction<'a> {
    tx: sqlx::Transaction<'a, Any>,
}

impl<'a> Transaction<'a> {
    /// Execute a statement inside this transaction.
    pub async fn execute(
        &mut self,
        sql: &str,
        params: &[serde_json::Value],
    ) -> crate::NoorResult<u64> {
        let mut query = sqlx::query::<Any>(sql);
        for p in params {
            query = JsonToSql::bind(query, p);
        }
        let result = query
            .execute(&mut *self.tx)
            .await
            .map_err(|e| crate::NoorError::Database(format!("Transaction execute error: {}", e)))?;
        Ok(result.rows_affected())
    }

    /// Run a SELECT inside this transaction.
    pub async fn query(
        &mut self,
        sql: &str,
        params: &[serde_json::Value],
    ) -> crate::NoorResult<Vec<serde_json::Value>> {
        let mut query = sqlx::query::<Any>(sql);
        for p in params {
            query = JsonToSql::bind(query, p);
        }
        let rows = query
            .fetch_all(&mut *self.tx)
            .await
            .map_err(|e| crate::NoorError::Database(format!("Transaction query error: {}", e)))?;
        Ok(rows.iter().map(row_to_json).collect())
    }

    /// Commit the transaction.
    pub async fn commit(self) -> crate::NoorResult<()> {
        self.tx
            .commit()
            .await
            .map_err(|e| crate::NoorError::Database(format!("COMMIT error: {}", e)))
    }

    /// Roll back the transaction.
    pub async fn rollback(self) -> crate::NoorResult<()> {
        self.tx
            .rollback()
            .await
            .map_err(|e| crate::NoorError::Database(format!("ROLLBACK error: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each test gets its own temp file database. We can't use
    /// `sqlite::memory:` because sqlx's `AnyPool` may hand out different
    /// connections for different queries, and each `:memory:` connection
    /// has its own private schema — so a table created via one connection
    /// wouldn't be visible to a query running on another connection.
    /// A file-backed DB shares schema across all connections in the pool.
    async fn test_db() -> (Database, tempfile::TempPath) {
        let file = tempfile::Builder::new()
            .suffix(".db")
            .tempfile()
            .expect("failed to create temp file");
        let path = file.into_temp_path();
        let url = format!("sqlite:{}", path.display());
        let db = Database::new("sqlite", &url)
            .await
            .expect("failed to open test sqlite db");
        (db, path)
    }

    #[tokio::test]
    async fn test_create_and_query() {
        let (db, _path) = test_db().await;

        db.execute(
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL, age INTEGER)",
            &[],
        )
        .await
        .unwrap();

        let affected = db
            .execute(
                "INSERT INTO users (name, age) VALUES (?, ?)",
                &[
                    serde_json::json!("Alice"),
                    serde_json::json!(30),
                ],
            )
            .await
            .unwrap();
        assert_eq!(affected, 1);

        let rows = db.query("SELECT id, name, age FROM users", &[]).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["name"], serde_json::json!("Alice"));
        assert_eq!(rows[0]["age"], serde_json::json!(30));
    }

    #[tokio::test]
    async fn test_query_first() {
        let (db, _path) = test_db().await;
        db.execute(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT)",
            &[],
        )
        .await
        .unwrap();
        db.execute(
            "INSERT INTO t (val) VALUES ('a'), ('b'), ('c')",
            &[],
        )
        .await
        .unwrap();

        let first = db
            .query_first("SELECT val FROM t ORDER BY id", &[])
            .await
            .unwrap();
        assert_eq!(first.unwrap()["val"], serde_json::json!("a"));

        let none = db
            .query_first("SELECT val FROM t WHERE val = 'zzz'", &[])
            .await
            .unwrap();
        assert!(none.is_none());
    }

    #[tokio::test]
    async fn test_table_exists() {
        let (db, _path) = test_db().await;
        db.execute("CREATE TABLE foo (id INTEGER)", &[]).await.unwrap();

        assert!(db.table_exists("foo").await.unwrap());
        assert!(!db.table_exists("bar").await.unwrap());
    }

    #[tokio::test]
    async fn test_transaction_commit() {
        let (db, _path) = test_db().await;
        db.execute("CREATE TABLE counter (n INTEGER)", &[]).await.unwrap();

        let mut tx = db.begin_transaction().await.unwrap();
        tx.execute("INSERT INTO counter (n) VALUES (1)", &[]).await.unwrap();
        tx.execute("INSERT INTO counter (n) VALUES (2)", &[]).await.unwrap();
        tx.commit().await.unwrap();

        let rows = db.query("SELECT n FROM counter", &[]).await.unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[tokio::test]
    async fn test_transaction_rollback() {
        let (db, _path) = test_db().await;
        db.execute("CREATE TABLE counter (n INTEGER)", &[]).await.unwrap();

        let mut tx = db.begin_transaction().await.unwrap();
        tx.execute("INSERT INTO counter (n) VALUES (1)", &[]).await.unwrap();
        tx.rollback().await.unwrap();

        let rows = db.query("SELECT n FROM counter", &[]).await.unwrap();
        assert_eq!(rows.len(), 0);
    }

    #[tokio::test]
    async fn test_parameterized_query_prevents_injection() {
        let (db, _path) = test_db().await;
        db.execute(
            "CREATE TABLE u (id INTEGER PRIMARY KEY, name TEXT)",
            &[],
        )
        .await
        .unwrap();
        db.execute(
            "INSERT INTO u (name) VALUES ('alice')",
            &[],
        )
        .await
        .unwrap();

        // A malicious value that should be treated as a literal string.
        let evil = "'; DROP TABLE u; --";
        let rows = db
            .query("SELECT name FROM u WHERE name = ?", &[serde_json::json!(evil)])
            .await
            .unwrap();
        assert_eq!(rows.len(), 0, "no rows should match the evil name");

        // Table must still exist.
        assert!(db.table_exists("u").await.unwrap());
    }
}
