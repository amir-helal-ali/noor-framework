// ============================================================
// ORM - Object-Relational Mapping
// ============================================================
// Lightweight ORM with:
// - Query builder (fluent API)
// - Model trait
// - Migrations
// - SQLite/PostgreSQL/MySQL support
// - Parameterized queries (SQL injection safe)
//
// ORM خفيف مع Query Builder و Model و Migrations.
// ============================================================

pub mod model;
pub mod query;
pub mod database;
pub mod migration;
pub mod json_bind;

pub use model::{Model, ModelMeta};
pub use query::{QueryBuilder, QueryResult};
pub use database::Database;
pub use migration::{Migration, Migrator};
