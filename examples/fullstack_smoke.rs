// ============================================================
// Integration smoke test: HTTP server + Database + Migrator
// ============================================================
// This binary boots a real Noor server backed by a SQLite database,
// applies a migration that creates a `visits` table, exposes a
// `/visit` endpoint that inserts a row on every hit and a `/visits`
// endpoint that counts them, then curls both endpoints to prove the
// full stack works end-to-end.
// ============================================================

use std::sync::Arc;
use std::time::Duration;

use noor::core::config::{Config, DatabaseConfig, DatabaseDriver, Environment};
use noor::core::http::{Request, Response};
use noor::core::orm::{Database, Migration, Migrator};
use noor::core::router::Router;
use noor::Application;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Use a temp file DB so the test is fully isolated.
    let tmp = tempfile::Builder::new().suffix(".db").tempfile()?;
    let db_path = tmp.into_temp_path();
    let db_url = format!("sqlite:{}", db_path.display());

    // Connect to the database and run a migration.
    let db = Database::new("sqlite", &db_url).await?;

    let mut migrator = Migrator::new();
    migrator.add(Migration::new(
        "20260713_000001",
        "create_visits",
        "CREATE TABLE visits (id INTEGER PRIMARY KEY, ua TEXT, ts INTEGER)",
        "DROP TABLE visits",
    ));
    let applied = migrator.run(&db).await?;
    println!("Applied {} migration(s)", applied);
    assert_eq!(applied, 1);

    // Share the DB across handlers via Arc.
    let db = Arc::new(db);

    // Build a small router that uses the DB.
    let db_for_visit = db.clone();
    let db_for_count = db.clone();

    let mut router = Router::new();
    router.get("/", |_req| {
        Ok(Response::ok().html("<h1>Noor full-stack smoke test</h1>"))
    });
    router.get("/visit", move |_req: Request| {
        let db = db_for_visit.clone();
        let ua = _req.user_agent.clone().unwrap_or_default();
        // `execute_blocking` spawns a fresh thread with its own current-thread
        // tokio runtime, so it is safe to call from inside a sync handler
        // that lives in the multi-threaded server runtime.
        let _ = db.execute_blocking(
            "INSERT INTO visits (ua, ts) VALUES (?, ?)",
            &[
                serde_json::json!(ua),
                serde_json::json!(chrono::Utc::now().timestamp()),
            ],
        );
        Ok(Response::ok().text("visit recorded"))
    });
    router.get("/visits", move |_req: Request| {
        let db = db_for_count.clone();
        let rows = db
            .query_blocking("SELECT COUNT(*) AS c FROM visits", &[])
            .map_err(|e| noor::NoorError::Database(e.to_string()))?;
        let count = rows
            .first()
            .and_then(|r| r["c"].as_i64())
            .unwrap_or(0);
        Ok(Response::ok().text(format!("visits={}", count)))
    });
    router.get("/health", move |_req: Request| {
        Ok(Response::ok().json(&serde_json::json!({
            "status": "ok",
            "db": "sqlite",
            "migrations_applied": applied,
        }))?)
    });

    // Use a custom config so we pick a free port (0 = OS-assigned).
    // We can't easily read the bound port back from the running server, so
    // we pick a fixed high port and hope it's free.
    let mut config = Config::default();
    config.app.env = Environment::Development;
    config.server.host = "127.0.0.1".to_string();
    config.server.port = 18555;
    config.database = DatabaseConfig {
        driver: DatabaseDriver::Sqlite,
        url: db_url.clone(),
        max_connections: 5,
        min_connections: 1,
        enable_logging: false,
    };

    // Spawn the server on a plain OS thread (NOT a tokio task), because
    // `Application::run` builds its own tokio runtime internally — calling
    // `Runtime::block_on` from inside an existing runtime panics.
    let app = Application::new(config, router);
    let server_thread = std::thread::spawn(move || {
        let _ = app.run();
    });

    // Give the server a moment to start.
    std::thread::sleep(Duration::from_millis(800));

    let base = "http://127.0.0.1:18555";

    // Hit /health.
    let resp = reqwest::get(format!("{}/health", base)).await?;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await?;
    println!("GET /health -> {}", body);
    assert_eq!(body["status"], "ok");
    assert_eq!(body["migrations_applied"], 1);

    // Hit /visit three times.
    for _ in 0..3 {
        reqwest::get(format!("{}/visit", base)).await?;
    }

    // Check the count.
    let resp = reqwest::get(format!("{}/visits", base)).await?;
    assert_eq!(resp.status(), 200);
    let text = resp.text().await?;
    println!("GET /visits -> {}", text);
    assert!(text.contains("visits=3"), "expected visits=3, got {}", text);

    // Confirm the DB also sees the rows directly.
    let rows = db.query("SELECT COUNT(*) AS c FROM visits", &[]).await?;
    let db_count = rows[0]["c"].as_i64().unwrap_or(0);
    println!("Direct DB count = {}", db_count);
    assert_eq!(db_count, 3);

    println!("\n✓ Full-stack smoke test passed: server + database + migrator all work end-to-end.");

    // Stop the server thread (it will be killed when the process exits, but
    // we explicitly drop the join handle here to make the intent clear).
    drop(server_thread);
    Ok(())
}
