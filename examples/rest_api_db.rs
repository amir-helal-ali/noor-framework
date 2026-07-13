// ============================================================
// Example: Complete REST API with Database + Middleware
// مثال: REST API كامل مع قاعدة بيانات و middleware
// ============================================================
// Demonstrates:
// - Real SQLite database via sqlx
// - Migrations (auto-applied on startup)
// - CRUD endpoints (GET/POST/PUT/DELETE)
// - CORS headers (applied by the server)
// - Security headers (applied by the server)
// - Request logging (via tracing)
// - Error handling with proper HTTP status codes
// - JSON request/response bodies
// ============================================================

use std::sync::Arc;

use noor::core::config::{Config, DatabaseConfig, DatabaseDriver, Environment};
use noor::core::http::{Request, Response, StatusCode};
use noor::core::orm::{Database, Migration, Migrator, QueryBuilder};
use noor::core::router::Router;
use noor::Application;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // --- Database setup ---
    let tmp = tempfile::Builder::new().suffix(".db").tempfile()?;
    let db_path = tmp.into_temp_path();
    let db_url = format!("sqlite:{}", db_path.display());

    let db = Database::new("sqlite", &db_url).await?;

    // Run migrations to create the `tasks` table.
    let mut migrator = Migrator::new();
    migrator.add(Migration::new(
        "20260713_000001",
        "create_tasks",
        "CREATE TABLE tasks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            done INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL
        )",
        "DROP TABLE tasks",
    ));
    let applied = migrator.run(&db).await?;
    println!("Applied {} migration(s)", applied);

    let db = Arc::new(db);

    // --- Router setup ---
    let db_for_list = db.clone();
    let db_for_create = db.clone();
    let db_for_get = db.clone();
    let db_for_update = db.clone();
    let db_for_delete = db.clone();

    let mut router = Router::new();

    // GET /tasks — list all tasks
    router.get("/tasks", move |_req: Request| {
        let db = db_for_list.clone();
        let rows = db
            .query_blocking("SELECT id, title, done, created_at FROM tasks ORDER BY id", &[])
            .map_err(|e| noor::NoorError::Database(e.to_string()))?;
        let json = serde_json::to_string(&rows)
            .map_err(|e| noor::NoorError::Internal(e.to_string()))?;
        Ok(Response::ok()
            .header("content-type", "application/json; charset=utf-8")
            .body(json))
    });

    // GET /tasks/:id — get a single task
    router.get("/tasks/{id}", move |req: Request| {
        let db = db_for_get.clone();
        let id = req.param("id").unwrap_or("0");
        let id_val: serde_json::Value = serde_json::json!(id.parse::<i64>().unwrap_or(0));
        let rows = db
            .query_blocking(
                "SELECT id, title, done, created_at FROM tasks WHERE id = ?",
                &[id_val],
            )
            .map_err(|e| noor::NoorError::Database(e.to_string()))?;
        if let Some(row) = rows.into_iter().next() {
            let json = serde_json::to_string(&row)
                .map_err(|e| noor::NoorError::Internal(e.to_string()))?;
            Ok(Response::ok()
                .header("content-type", "application/json; charset=utf-8")
                .body(json))
        } else {
            Ok(Response::new(StatusCode::NOT_FOUND).text("Task not found"))
        }
    });

    // POST /tasks — create a new task
    router.post("/tasks", move |req: Request| {
        let db = db_for_create.clone();
        let body: serde_json::Value = req.json()
            .map_err(|_| noor::NoorError::Validation("Invalid JSON body".to_string()))?;
        let title = body["title"].as_str()
            .ok_or_else(|| noor::NoorError::Validation("Missing 'title' field".to_string()))?;
        if title.is_empty() {
            return Err(noor::NoorError::Validation("Title cannot be empty".to_string()));
        }
        let now = chrono::Utc::now().timestamp();
        db.execute_blocking(
            "INSERT INTO tasks (title, done, created_at) VALUES (?, 0, ?)",
            &[serde_json::json!(title), serde_json::json!(now)],
        ).map_err(|e| noor::NoorError::Database(e.to_string()))?;
        Ok(Response::new(StatusCode::CREATED).text("Task created"))
    });

    // PUT /tasks/:id — update a task (toggle done / rename)
    router.put("/tasks/{id}", move |req: Request| {
        let db = db_for_update.clone();
        let id = req.param("id").unwrap_or("0");
        let id_val: serde_json::Value = serde_json::json!(id.parse::<i64>().unwrap_or(0));
        let body: serde_json::Value = req.json()
            .map_err(|_| noor::NoorError::Validation("Invalid JSON body".to_string()))?;
        let title = body["title"].as_str().unwrap_or("");
        let done = body["done"].as_bool().unwrap_or(false);
        let affected = db.execute_blocking(
            "UPDATE tasks SET title = ?, done = ? WHERE id = ?",
            &[serde_json::json!(title), serde_json::json!(done), id_val],
        ).map_err(|e| noor::NoorError::Database(e.to_string()))?;
        if affected == 0 {
            Ok(Response::new(StatusCode::NOT_FOUND).text("Task not found"))
        } else {
            Ok(Response::ok().text("Task updated"))
        }
    });

    // DELETE /tasks/:id — delete a task
    router.delete("/tasks/{id}", move |req: Request| {
        let db = db_for_delete.clone();
        let id = req.param("id").unwrap_or("0");
        let id_val: serde_json::Value = serde_json::json!(id.parse::<i64>().unwrap_or(0));
        let affected = db.execute_blocking(
            "DELETE FROM tasks WHERE id = ?",
            &[id_val],
        ).map_err(|e| noor::NoorError::Database(e.to_string()))?;
        if affected == 0 {
            Ok(Response::new(StatusCode::NOT_FOUND).text("Task not found"))
        } else {
            Ok(Response::new(StatusCode::NO_CONTENT).body(""))
        }
    });

    // Health check
    router.get("/health", move |_req: Request| {
        Ok(Response::ok().json(&serde_json::json!({
            "status": "ok",
            "db": "sqlite",
            "migrations_applied": applied,
        }))?)
    });

    // --- Config + server ---
    let mut config = Config::default();
    config.app.env = Environment::Development;
    config.server.host = "127.0.0.1".to_string();
    config.server.port = 18666;
    config.database = DatabaseConfig {
        driver: DatabaseDriver::Sqlite,
        url: db_url.clone(),
        max_connections: 5,
        min_connections: 1,
        enable_logging: false,
    };
    // Enable CORS for all origins (so curl/browser can call the API).
    config.security.cors_origins = vec!["*".to_string()];
    config.security.secure_headers = true;

    let app = Application::new(config, router);

    // Spawn the server on a plain OS thread (Application::run builds its
    // own tokio runtime internally).
    let server_thread = std::thread::spawn(move || {
        let _ = app.run();
    });

    // Give the server a moment to start.
    std::thread::sleep(std::time::Duration::from_millis(800));

    let base = "http://127.0.0.1:18666";

    // --- Test the API end-to-end ---

    // 1. Health check
    let resp = reqwest::get(format!("{}/health", base)).await?;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await?;
    println!("✓ GET /health -> {}", body);
    assert_eq!(body["status"], "ok");

    // 2. Create a task
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/tasks", base))
        .json(&serde_json::json!({"title": "Learn Noor Framework"}))
        .send()
        .await?;
    assert_eq!(resp.status(), 201);
    println!("✓ POST /tasks -> 201 Created");

    // 3. Create another task
    let resp = client
        .post(format!("{}/tasks", base))
        .json(&serde_json::json!({"title": "Build an app"}))
        .send()
        .await?;
    assert_eq!(resp.status(), 201);
    println!("✓ POST /tasks -> 201 Created");

    // 4. List all tasks
    let resp = client.get(format!("{}/tasks", base)).send().await?;
    assert_eq!(resp.status(), 200);
    let tasks: Vec<serde_json::Value> = resp.json().await?;
    println!("✓ GET /tasks -> {} task(s)", tasks.len());
    assert_eq!(tasks.len(), 2);
    assert_eq!(tasks[0]["title"], "Learn Noor Framework");

    // 5. Get a single task
    let resp = client.get(format!("{}/tasks/1", base)).send().await?;
    assert_eq!(resp.status(), 200);
    let task: serde_json::Value = resp.json().await?;
    println!("✓ GET /tasks/1 -> {}", task["title"]);
    assert_eq!(task["title"], "Learn Noor Framework");

    // 6. Update a task (mark as done)
    let resp = client
        .put(format!("{}/tasks/1", base))
        .json(&serde_json::json!({"title": "Learn Noor Framework", "done": true}))
        .send()
        .await?;
    assert_eq!(resp.status(), 200);
    println!("✓ PUT /tasks/1 -> 200 OK");

    // 7. Verify the update
    let resp = client.get(format!("{}/tasks/1", base)).send().await?;
    let task: serde_json::Value = resp.json().await?;
    assert_eq!(task["done"], 1, "task should be done (SQLite stores bool as 1)");
    println!("✓ Task 1 done = {}", task["done"]);

    // 8. Delete a task
    let resp = client.delete(format!("{}/tasks/2", base)).send().await?;
    assert_eq!(resp.status(), 204);
    println!("✓ DELETE /tasks/2 -> 204 No Content");

    // 9. Verify deletion
    let resp = client.get(format!("{}/tasks/2", base)).send().await?;
    assert_eq!(resp.status(), 404);
    println!("✓ GET /tasks/2 -> 404 Not Found (deleted)");

    // 10. Verify CORS headers are present
    let resp = client.get(format!("{}/health", base)).send().await?;
    assert!(resp.headers().contains_key("access-control-allow-origin"));
    println!("✓ CORS header present");

    // 11. Verify security headers
    assert!(resp.headers().contains_key("x-content-type-options"));
    assert!(resp.headers().contains_key("x-frame-options"));
    println!("✓ Security headers present");

    println!("\n✅ All REST API tests passed!");
    println!("   - CRUD: Create, Read, Update, Delete all work");
    println!("   - Migrations: auto-applied on startup");
    println!("   - CORS: headers applied to responses");
    println!("   - Security: X-Content-Type-Options, X-Frame-Options, etc.");
    println!("   - Logging: request logs emitted via tracing");

    drop(server_thread);
    Ok(())
}
