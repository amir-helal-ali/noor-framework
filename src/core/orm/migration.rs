// ============================================================
// Database Migrations - ترحيلات قاعدة البيانات
// ============================================================
// Real migration runner backed by a `Database` connection.
//
// State persistence:
//   The migrator creates a `_migrations` table in the target database and
//   records each applied migration's version + name + timestamp there. On
//   subsequent runs, only pending migrations are executed.
//
//   The previous implementation kept "applied" state only in memory, so
//   restarting the application re-ran every migration (which is a
//   destructive operation for `CREATE TABLE` without `IF NOT EXISTS`).
//
// Transactions:
//   Each migration runs inside its own transaction. If the migration's SQL
//   fails, the transaction is rolled back and the migration is NOT marked
//   as applied. On success the transaction commits and the migration row
//   is inserted atomically.
// ============================================================

use std::sync::Arc;

use parking_lot::RwLock;

use crate::core::orm::database::Database;

/// A migration definition.
///
/// `version` is a free-form string (we recommend ISO-like timestamps such
/// as `20260710_000001` so they sort chronologically). `name` is a
/// human-readable description. `up_sql` runs on `Migrator::run`; `down_sql`
/// runs on `Migrator::rollback`.
#[derive(Clone)]
pub struct Migration {
    pub version: String,
    pub name: String,
    pub up_sql: String,
    pub down_sql: String,
}

impl Migration {
    /// Convenience constructor.
    pub fn new(version: &str, name: &str, up_sql: &str, down_sql: &str) -> Self {
        Self {
            version: version.to_string(),
            name: name.to_string(),
            up_sql: up_sql.to_string(),
            down_sql: down_sql.to_string(),
        }
    }
}

/// Migration manager.
///
/// Construct with `Migrator::new()`, register migrations with `add()`, then
/// call `run()` against a live `Database` to apply pending migrations or
/// `rollback()` to revert the most recently applied one.
pub struct Migrator {
    migrations: Vec<Migration>,
    /// In-memory mirror of the `_migrations` table, populated by
    /// `sync_applied()`. Kept as an `Arc<RwLock<…>>` so callers can share a
    /// migrator across tasks if they wish.
    applied: Arc<RwLock<Vec<AppliedMigration>>>,
}

/// A row in the `_migrations` table.
#[derive(Debug, Clone)]
pub struct AppliedMigration {
    pub version: String,
    pub name: String,
    pub applied_at: i64,
}

impl Migrator {
    pub fn new() -> Self {
        Self {
            migrations: Vec::new(),
            applied: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a migration to the registry. Migrations are applied in the order
    /// they are added, so register them in chronological order.
    pub fn add(&mut self, migration: Migration) {
        self.migrations.push(migration);
    }

    /// Ensure the `_migrations` table exists, then load the list of already-
    /// applied migrations into memory.
    pub async fn sync_applied(&self, db: &Database) -> crate::NoorResult<()> {
        db.execute(
            "CREATE TABLE IF NOT EXISTS _migrations (
                version TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at INTEGER NOT NULL
            )",
            &[],
        )
        .await?;

        let rows = db
            .query(
                "SELECT version, name, applied_at FROM _migrations ORDER BY applied_at",
                &[],
            )
            .await?;

        let applied: Vec<AppliedMigration> = rows
            .iter()
            .map(|r| AppliedMigration {
                version: r["version"].as_str().unwrap_or("").to_string(),
                name: r["name"].as_str().unwrap_or("").to_string(),
                applied_at: r["applied_at"].as_i64().unwrap_or(0),
            })
            .collect();

        *self.applied.write() = applied;
        Ok(())
    }

    /// Run all pending migrations in order.
    ///
    /// Each migration runs inside its own transaction. If the SQL succeeds,
    /// the transaction commits and a row is inserted into `_migrations`. If
    /// the SQL fails, the transaction rolls back and the error is returned
    /// immediately (no further migrations are attempted).
    pub async fn run(&self, db: &Database) -> crate::NoorResult<usize> {
        self.sync_applied(db).await?;

        // Snapshot of already-applied versions so we can skip them.
        let applied_versions: std::collections::HashSet<String> = self
            .applied
            .read()
            .iter()
            .map(|a| a.version.clone())
            .collect();

        let mut applied_count = 0usize;
        for migration in &self.migrations {
            if applied_versions.contains(&migration.version) {
                continue;
            }

            tracing::info!(
                "Running migration {} ({})",
                migration.version,
                migration.name
            );

            // Execute the migration's SQL inside a transaction. If it fails,
            // roll back and surface the error.
            let mut tx = db.begin_transaction().await?;

            // sqlx executes a single statement per `execute()` call. Split
            // the migration SQL on `;` so DDL like
            //   `CREATE TABLE a (...); CREATE TABLE b (...);`
            // works as expected. This is a simple splitter — it does NOT
            // understand string literals containing `;`, so migrations
            // using those should use a single `execute` statement.
            for stmt in split_statements(&migration.up_sql) {
                let trimmed = stmt.trim();
                if trimmed.is_empty() {
                    continue;
                }
                tx.execute(trimmed, &[]).await?;
            }

            // Record the migration as applied, inside the same transaction
            // so the schema change + the bookkeeping row commit atomically.
            tx.execute(
                "INSERT INTO _migrations (version, name, applied_at) VALUES (?, ?, ?)",
                &[
                    serde_json::json!(migration.version),
                    serde_json::json!(migration.name),
                    serde_json::json!(chrono::Utc::now().timestamp()),
                ],
            )
            .await?;

            tx.commit().await?;

            self.applied.write().push(AppliedMigration {
                version: migration.version.clone(),
                name: migration.name.clone(),
                applied_at: chrono::Utc::now().timestamp(),
            });
            applied_count += 1;
        }

        Ok(applied_count)
    }

    /// Roll back the most recently applied migration.
    ///
    /// Looks up the last entry in `_migrations` (by `applied_at`), finds the
    /// matching `Migration` in the registry, runs its `down_sql` inside a
    /// transaction, and removes the `_migrations` row on success.
    pub async fn rollback(&self, db: &Database) -> crate::NoorResult<Option<String>> {
        self.sync_applied(db).await?;

        let last = self.applied.read().last().cloned();
        let last = match last {
            Some(l) => l,
            None => return Ok(None),
        };

        let migration = self
            .migrations
            .iter()
            .find(|m| m.version == last.version)
            .cloned();

        let down_sql = match migration {
            Some(m) => m.down_sql,
            None => {
                return Err(crate::NoorError::Database(format!(
                    "Migration {} is applied but no longer in the registry; cannot roll back",
                    last.version
                )))
            }
        };

        tracing::info!("Rolling back migration {} ({})", last.version, last.name);

        let mut tx = db.begin_transaction().await?;
        for stmt in split_statements(&down_sql) {
            let trimmed = stmt.trim();
            if trimmed.is_empty() {
                continue;
            }
            tx.execute(trimmed, &[]).await?;
        }
        tx.execute(
            "DELETE FROM _migrations WHERE version = ?",
            &[serde_json::json!(last.version)],
        )
        .await?;
        tx.commit().await?;

        self.applied.write().retain(|a| a.version != last.version);

        Ok(Some(last.version))
    }

    /// Migrations registered but not yet applied.
    pub fn pending(&self) -> Vec<&Migration> {
        let applied: std::collections::HashSet<String> = self
            .applied
            .read()
            .iter()
            .map(|a| a.version.clone())
            .collect();
        self.migrations
            .iter()
            .filter(|m| !applied.contains(&m.version))
            .collect()
    }

    /// Migrations that have been applied (according to the in-memory mirror,
    /// which is populated by `sync_applied` / `run`).
    pub fn applied_migrations(&self) -> Vec<AppliedMigration> {
        self.applied.read().clone()
    }

    /// True if the given version has been applied.
    pub fn is_applied(&self, version: &str) -> bool {
        self.applied.read().iter().any(|a| a.version == version)
    }
}

impl Default for Migrator {
    fn default() -> Self {
        Self::new()
    }
}

/// Split a multi-statement SQL string on `;` while respecting:
///   - single-quoted string literals (`'...'`)
///   - double-quoted identifiers (`"..."`)
///   - line comments (`-- ...`)
///   - block comments (`/* ... */`)
///
/// This is good enough for migration files; it does NOT handle dollar-quoted
/// strings (Postgres `$$...$$`), so callers using those should run their
/// migration body as a single statement (which is the default for raw SQL
/// migration files anyway).
fn split_statements(sql: &str) -> Vec<String> {
    let mut stmts = Vec::new();
    let mut current = String::new();
    let mut chars = sql.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\'' => {
                current.push(c);
                // Consume until the closing quote (handle '' escapes).
                while let Some(&next) = chars.peek() {
                    current.push(chars.next().unwrap());
                    if next == '\'' {
                        // Look ahead: if next char is also a quote, it's an escape.
                        if chars.peek() == Some(&'\'') {
                            current.push(chars.next().unwrap());
                            continue;
                        }
                        break;
                    }
                }
            }
            '"' => {
                current.push(c);
                while let Some(&next) = chars.peek() {
                    current.push(chars.next().unwrap());
                    if next == '"' {
                        break;
                    }
                }
            }
            '-' if chars.peek() == Some(&'-') => {
                // Line comment: consume until newline.
                current.push(c);
                while let Some(&next) = chars.peek() {
                    if next == '\n' {
                        break;
                    }
                    current.push(chars.next().unwrap());
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                // Block comment: consume until `*/`.
                current.push(c);
                current.push(chars.next().unwrap()); // consume '*'
                let mut prev = '\0';
                while let Some(&next) = chars.peek() {
                    let n = chars.next().unwrap();
                    current.push(n);
                    if prev == '*' && n == '/' {
                        break;
                    }
                    prev = n;
                }
            }
            ';' => {
                stmts.push(std::mem::take(&mut current));
            }
            _ => {
                current.push(c);
            }
        }
    }
    if !current.trim().is_empty() {
        stmts.push(current);
    }
    stmts
}

/// Helper for creating table migrations (DDL builder).
pub struct TableBuilder {
    table_name: String,
    columns: Vec<ColumnDef>,
}

struct ColumnDef {
    name: String,
    sql_type: String,
    nullable: bool,
    default: Option<String>,
    primary_key: bool,
    unique: bool,
    indexed: bool,
}

impl TableBuilder {
    pub fn new(table_name: &str) -> Self {
        Self {
            table_name: table_name.to_string(),
            columns: Vec::new(),
        }
    }

    pub fn id(mut self) -> Self {
        self.columns.push(ColumnDef {
            name: "id".to_string(),
            sql_type: "BIGINT".to_string(),
            nullable: false,
            default: None,
            primary_key: true,
            unique: false,
            indexed: false,
        });
        self
    }

    pub fn string(mut self, name: &str) -> Self {
        self.columns.push(ColumnDef {
            name: name.to_string(),
            sql_type: "VARCHAR(255)".to_string(),
            nullable: true,
            default: None,
            primary_key: false,
            unique: false,
            indexed: false,
        });
        self
    }

    pub fn text(mut self, name: &str) -> Self {
        self.columns.push(ColumnDef {
            name: name.to_string(),
            sql_type: "TEXT".to_string(),
            nullable: true,
            default: None,
            primary_key: false,
            unique: false,
            indexed: false,
        });
        self
    }

    pub fn integer(mut self, name: &str) -> Self {
        self.columns.push(ColumnDef {
            name: name.to_string(),
            sql_type: "INTEGER".to_string(),
            nullable: true,
            default: None,
            primary_key: false,
            unique: false,
            indexed: false,
        });
        self
    }

    pub fn boolean(mut self, name: &str) -> Self {
        self.columns.push(ColumnDef {
            name: name.to_string(),
            sql_type: "BOOLEAN".to_string(),
            nullable: true,
            default: Some("false".to_string()),
            primary_key: false,
            unique: false,
            indexed: false,
        });
        self
    }

    pub fn timestamps(mut self) -> Self {
        self.columns.push(ColumnDef {
            name: "created_at".to_string(),
            sql_type: "TIMESTAMP".to_string(),
            nullable: true,
            default: Some("CURRENT_TIMESTAMP".to_string()),
            primary_key: false,
            unique: false,
            indexed: false,
        });
        self.columns.push(ColumnDef {
            name: "updated_at".to_string(),
            sql_type: "TIMESTAMP".to_string(),
            nullable: true,
            default: Some("CURRENT_TIMESTAMP".to_string()),
            primary_key: false,
            unique: false,
            indexed: false,
        });
        self
    }

    pub fn build(self) -> String {
        let mut sql = format!("CREATE TABLE IF NOT EXISTS {} (\n", self.table_name);

        let column_strs: Vec<String> = self
            .columns
            .iter()
            .map(|c| {
                let mut s = format!("  {} {}", c.name, c.sql_type);
                if !c.nullable {
                    s.push_str(" NOT NULL");
                }
                if let Some(ref default) = c.default {
                    s.push_str(&format!(" DEFAULT {}", default));
                }
                if c.primary_key {
                    s.push_str(" PRIMARY KEY");
                }
                if c.unique {
                    s.push_str(" UNIQUE");
                }
                s
            })
            .collect();

        sql.push_str(&column_strs.join(",\n"));
        sql.push_str("\n)");

        sql
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// File-backed temp DB (see orm::database::tests::test_db for why we
    /// can't use `sqlite::memory:` with a pool).
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
    async fn test_run_applies_pending_migrations() {
        let (db, _path) = test_db().await;
        let mut m = Migrator::new();
        m.add(Migration::new(
            "20260710_000001",
            "create_users",
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
            "DROP TABLE users",
        ));
        m.add(Migration::new(
            "20260710_000002",
            "create_posts",
            "CREATE TABLE posts (id INTEGER PRIMARY KEY, title TEXT NOT NULL)",
            "DROP TABLE posts",
        ));

        let applied = m.run(&db).await.unwrap();
        assert_eq!(applied, 2);

        assert!(db.table_exists("users").await.unwrap());
        assert!(db.table_exists("posts").await.unwrap());
        assert!(db.table_exists("_migrations").await.unwrap());

        // Re-running should be a no-op.
        let applied_again = m.run(&db).await.unwrap();
        assert_eq!(applied_again, 0);
    }

    #[tokio::test]
    async fn test_migration_persists_across_migrator_instances() {
        let (db, _path) = test_db().await;

        {
            let mut m = Migrator::new();
            m.add(Migration::new(
                "20260710_000001",
                "create_users",
                "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)",
                "DROP TABLE users",
            ));
            m.run(&db).await.unwrap();
        }

        // A fresh migrator instance should see the migration as applied.
        let mut m2 = Migrator::new();
        m2.add(Migration::new(
            "20260710_000001",
            "create_users",
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)",
            "DROP TABLE users",
        ));
        let applied = m2.run(&db).await.unwrap();
        assert_eq!(applied, 0, "migration should already be applied");
    }

    #[tokio::test]
    async fn test_rollback_last_migration() {
        let (db, _path) = test_db().await;
        let mut m = Migrator::new();
        m.add(Migration::new(
            "20260710_000001",
            "create_users",
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)",
            "DROP TABLE users",
        ));
        m.run(&db).await.unwrap();
        assert!(db.table_exists("users").await.unwrap());

        let rolled = m.rollback(&db).await.unwrap();
        assert_eq!(rolled, Some("20260710_000001".to_string()));
        assert!(!db.table_exists("users").await.unwrap());
        assert_eq!(m.applied_migrations().len(), 0);
    }

    #[tokio::test]
    async fn test_failed_migration_does_not_mark_applied() {
        let (db, _path) = test_db().await;
        let mut m = Migrator::new();
        // Invalid SQL should fail.
        m.add(Migration::new(
            "20260710_000001",
            "bad",
            "CREATE TABL foo (id INTEGER)", // typo
            "DROP TABLE foo",
        ));

        let result = m.run(&db).await;
        assert!(result.is_err(), "run should return an error");
        assert_eq!(m.applied_migrations().len(), 0);
    }

    #[tokio::test]
    async fn test_multi_statement_migration() {
        let (db, _path) = test_db().await;
        let mut m = Migrator::new();
        m.add(Migration::new(
            "20260710_000001",
            "create_two_tables",
            "CREATE TABLE a (id INTEGER PRIMARY KEY);\nCREATE TABLE b (id INTEGER PRIMARY KEY);\n-- comment\n",
            "DROP TABLE a; DROP TABLE b;",
        ));

        m.run(&db).await.unwrap();
        assert!(db.table_exists("a").await.unwrap());
        assert!(db.table_exists("b").await.unwrap());

        m.rollback(&db).await.unwrap();
        assert!(!db.table_exists("a").await.unwrap());
        assert!(!db.table_exists("b").await.unwrap());
    }

    #[test]
    fn test_split_statements_basic() {
        let stmts = split_statements("CREATE TABLE a (id INT); CREATE TABLE b (id INT);");
        assert_eq!(stmts.len(), 2);
        assert!(stmts[0].contains("CREATE TABLE a"));
        assert!(stmts[1].contains("CREATE TABLE b"));
    }

    #[test]
    fn test_split_statements_respects_string_literals() {
        let stmts = split_statements("INSERT INTO t VALUES ('a;b'); SELECT 1;");
        assert_eq!(stmts.len(), 2);
        assert!(stmts[0].contains("'a;b'"));
    }

    #[test]
    fn test_split_statements_handles_comments() {
        let stmts = split_statements(
            "CREATE TABLE a (id INT); -- this is a comment with ; semicolons\nSELECT 1;",
        );
        assert_eq!(stmts.len(), 2);
    }
}
