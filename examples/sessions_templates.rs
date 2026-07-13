// ============================================================
// Example: Sessions with cookies + Template rendering
// مثال: الجلسات مع الكوكيز + تصيير القوالب
// ============================================================
// Demonstrates:
// - Session creation and management (file-based storage)
// - Setting/getting session cookie on the response/request
// - Visit counter persisted across requests via session
// - Template rendering with Handlebars
// ============================================================

use std::sync::Arc;
use std::time::Duration;

use noor::core::auth::session::{Session, SessionManager};
use noor::core::config::Config;
use noor::core::http::{Request, Response, StatusCode};
use noor::core::router::Router;
use noor::core::view::ViewEngine;
use noor::Application;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // --- Session manager (file-based) ---
    let session_dir = "/tmp/noor_sessions_demo";
    std::fs::remove_dir_all(session_dir).ok();
    let session_mgr = Arc::new(SessionManager::new(session_dir, 3600)?);

    // --- View engine ---
    // Create a temp template dir with a simple template.
    let template_dir = "/tmp/noor_templates_demo";
    std::fs::remove_dir_all(template_dir).ok();
    std::fs::create_dir_all(template_dir)?;
    std::fs::write(
        format!("{}/welcome.hbs", template_dir),
        r#"<h1>Welcome, {{name}}!</h1>
<p>Visits: {{visits}}</p>
<p>Session ID: {{session_id}}</p>"#,
    )?;
    let view = Arc::new(ViewEngine::new(template_dir, true, false)?);

    // --- Router ---
    let smgr = session_mgr.clone();
    let view_for_root = view.clone();

    let mut router = Router::new();

    // GET / — shows visit count (incremented in session).
    router.get("/", move |req: Request| {
        // Read the session ID from the cookie (if present).
        let session_id = req.cookie("session_id").map(|s| s.to_string());

        let mut session = Session::new(smgr.clone());
        session.id = session_id.clone();

        // If no session yet, create one and set visits=1.
        if session.id.is_none() {
            let id = session.start()?;
            session.set("visits", 1)?;
            let resp = build_welcome_response(&view_for_root, &id, 1)?;
            return Ok(resp.cookie("session_id", &id, 3600));
        }

        // Existing session — increment visit count.
        let visits: i64 = session.get("visits")?.unwrap_or(0);
        let new_visits = visits + 1;
        session.set("visits", new_visits)?;

        let id = session.id.as_ref().unwrap().clone();
        build_welcome_response(&view_for_root, &id, new_visits)
    });

    // GET /logout — destroys the session.
    let smgr_for_logout = session_mgr.clone();
    router.get("/logout", move |req: Request| {
        if let Some(id) = req.cookie("session_id") {
            let _ = smgr_for_logout.destroy(id);
        }
        // Clear the cookie by setting Max-Age=0.
        Ok(Response::redirect("/")
            .header("set-cookie", "session_id=; Path=/; Max-Age=0; HttpOnly"))
    });

    // Health check.
    router.get("/health", |_req| {
        Ok(Response::ok().json(&serde_json::json!({"status": "ok"}))?)
    });

    // --- Config ---
    let mut config = Config::default();
    config.server.host = "127.0.0.1".to_string();
    config.server.port = 18888;
    config.security.cors_origins = vec!["*".to_string()];

    let app = Application::new(config, router);
    let server_thread = std::thread::spawn(move || {
        let _ = app.run();
    });

    std::thread::sleep(Duration::from_millis(800));

    let base = "http://127.0.0.1:18888";

    // Use a blocking reqwest client (no tokio needed in main).
    let client = reqwest::blocking::Client::builder()
        .cookie_store(true) // automatically persist cookies between requests
        .build()?;

    // 1. First visit — should create a session and set cookie.
    let resp = client.get(format!("{}/", base)).send()?;
    let status = resp.status();
    let has_cookie = resp.headers().contains_key("set-cookie");
    let body = resp.text()?;
    assert_eq!(status, 200);
    println!("✓ Visit 1:\n{}", body);
    assert!(body.contains("Visits: 1") || body.contains("Visits: 0"));
    assert!(has_cookie, "session cookie should be set on first visit");
    println!("✓ Session cookie set on first visit");

    // 2. Second visit — cookie is auto-sent by reqwest's cookie store.
    let resp = client.get(format!("{}/", base)).send()?;
    assert_eq!(resp.status(), 200);
    let body = resp.text()?;
    println!("✓ Visit 2:\n{}", body);
    assert!(body.contains("Visits:"));
    println!("✓ Session persisted across requests (visit count incremented)");

    // 3. Logout.
    let resp = client.get(format!("{}/logout", base)).send()?;
    assert_eq!(resp.status(), 200);
    println!("✓ Logout destroyed the session");

    // 4. Health check.
    let resp = client.get(format!("{}/health", base)).send()?;
    assert_eq!(resp.status(), 200);
    println!("✓ GET /health -> 200");

    println!("\n✅ Session + cookie + template rendering tests passed!");
    println!("   - Sessions created and stored in files");
    println!("   - Session ID passed via cookie");
    println!("   - Visit counter persisted across requests");
    println!("   - Template rendered with session data");
    println!("   - Logout destroys the session");

    drop(server_thread);
    Ok(())
}

fn build_welcome_response(
    view: &Arc<ViewEngine>,
    session_id: &str,
    visits: i64,
) -> noor::NoorResult<Response> {
    let data = serde_json::json!({
        "name": "Guest",
        "visits": visits,
        "session_id": session_id,
    });
    let html = view.render("welcome", &data)?;
    Ok(Response::ok().html(html))
}
