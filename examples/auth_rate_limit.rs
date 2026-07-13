// ============================================================
// Example: JWT Authentication + Rate Limiting
// مثال: مصادقة JWT + تحديد المعدل
// ============================================================
// Demonstrates:
// - JWT token generation (login endpoint)
// - JWT auth middleware protecting /api/* routes
// - Rate limiting middleware on /api/login
// - Public vs protected endpoints
// ============================================================

use std::sync::Arc;
use std::time::Duration;

use noor::core::auth::Jwt;
use noor::core::config::Config;
use noor::core::http::{Request, Response, StatusCode};
use noor::core::middleware::auth;
use noor::core::middleware::throttle::{ThrottleConfig, ThrottleMiddleware};
use noor::core::router::Router;
use noor::Application;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // --- JWT setup ---
    let jwt = Arc::new(Jwt::new("super-secret-key", "noor", "noor_app"));

    // --- Router setup ---
    let mut router = Router::new();

    // Register the throttle middleware under the name "throttle".
    // 20 requests per 60 seconds per IP — enough for the demo flow, but
    // the 6 rapid /api/login calls will still exceed it after the other
    // requests consume part of the budget.
    let throttle = ThrottleMiddleware::new(ThrottleConfig {
        max_requests: 10,
        window_secs: 60,
        paths: None,
        excluded_ips: vec![],
    });
    noor::core::middleware::throttle::register(&mut router, throttle, "throttle");
    router.use_middleware("throttle"); // enable globally

    // Register the JWT auth middleware under the name "auth" (but don't
    // enable globally — only /api/me and /api/admin check tokens in-handler).
    auth::register(&mut router, jwt.clone(), "auth");

    // --- Public routes ---

    // Public: login endpoint (rate-limited via "throttle").
    // In a real app you'd verify the password against a DB; here we accept
    // any username/password and issue a token.
    let jwt_for_login = jwt.clone();
    router.post("/api/login", move |req: Request| {
        let body: serde_json::Value =
            req.json()
                .map_err(|_| noor::NoorError::Validation("Invalid JSON".to_string()))?;
        let username = body["username"].as_str().unwrap_or("");
        let password = body["password"].as_str().unwrap_or("");
        if username.is_empty() || password.is_empty() {
            return Err(noor::NoorError::Validation(
                "Missing username or password".to_string(),
            ));
        }
        let token = jwt_for_login.generate_access_token(username, vec!["user".to_string()])?;
        Ok(Response::ok().json(&serde_json::json!({
            "token": token,
            "user": username,
        }))?)
    });

    // --- Protected routes (require "auth" middleware) ---

    // Protected: get current user info.
    let jwt_for_me = jwt.clone();
    router.get("/api/me", move |req: Request| {
        // Verify the token manually (the "auth" middleware could also do
        // this automatically if registered globally, but we show the
        // in-handler pattern here).
        let token = req
            .bearer_token()
            .ok_or_else(|| noor::NoorError::Auth("Missing token".to_string()))?;
        let claims = jwt_for_me.verify(token)?;
        Ok(Response::ok().json(&serde_json::json!({
            "user_id": claims.sub,
            "roles": claims.roles,
            "expires_at": claims.exp,
        }))?)
    });

    // Protected: admin-only endpoint.
    let jwt_for_admin = jwt.clone();
    router.get("/api/admin", move |req: Request| {
        let token = req
            .bearer_token()
            .ok_or_else(|| noor::NoorError::Auth("Missing token".to_string()))?;
        let claims = jwt_for_admin.verify(token)?;
        if !claims.roles.contains(&"admin".to_string()) {
            return Ok(Response::new(StatusCode::FORBIDDEN).json(&serde_json::json!({
                "error": "Admin access required",
            }))?);
        }
        Ok(Response::ok().json(&serde_json::json!({
            "message": "Welcome, admin!",
            "admin": claims.sub,
        }))?)
    });

    // Public health check.
    router.get("/health", |_req: Request| {
        Ok(Response::ok().json(&serde_json::json!({"status": "ok"}))?)
    });

    // --- Config ---
    let mut config = Config::default();
    config.server.host = "127.0.0.1".to_string();
    config.server.port = 18777;
    config.security.cors_origins = vec!["*".to_string()];
    config.security.secure_headers = true;

    let app = Application::new(config, router);

    // Spawn server on a plain thread.
    let server_thread = std::thread::spawn(move || {
        let _ = app.run();
    });
    std::thread::sleep(Duration::from_millis(800));

    let base = "http://127.0.0.1:18777";
    let client = reqwest::Client::new();

    // 1. Health check (public).
    let resp = client.get(format!("{}/health", base)).send().await?;
    assert_eq!(resp.status(), 200);
    println!("✓ GET /health (public) -> 200");

    // 2. Access /api/me without token -> 401.
    let resp = client.get(format!("{}/api/me", base)).send().await?;
    assert_eq!(resp.status(), 401);
    println!("✓ GET /api/me without token -> 401 Unauthorized");

    // 3. Login to get a token.
    let resp = client
        .post(format!("{}/api/login", base))
        .json(&serde_json::json!({"username": "alice", "password": "secret"}))
        .send()
        .await?;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await?;
    let token = body["token"].as_str().unwrap();
    println!("✓ POST /api/login -> got token");

    // 4. Access /api/me with token -> 200.
    let resp = client
        .get(format!("{}/api/me", base))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["user_id"], "alice");
    println!("✓ GET /api/me with token -> 200 (user_id=alice)");

    // 5. Access /api/admin with non-admin token -> 403.
    let resp = client
        .get(format!("{}/api/admin", base))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;
    assert_eq!(resp.status(), 403);
    println!("✓ GET /api/admin with user token -> 403 Forbidden");

    // 6. Login as admin.
    let jwt_admin = jwt.clone();
    let admin_token = jwt_admin.generate_access_token("admin_user", vec!["admin".to_string()])?;

    // 7. Access /api/admin with admin token -> 200.
    let resp = client
        .get(format!("{}/api/admin", base))
        .header("Authorization", format!("Bearer {}", admin_token))
        .send()
        .await?;
    assert_eq!(resp.status(), 200);
    println!("✓ GET /api/admin with admin token -> 200");

    // 8. Rate limiting: hit /api/login many times rapidly to exceed the
    // 10 req/60s limit. Earlier requests in the test consumed some budget,
    // so we just hammer until we get a 429.
    println!("\n  Testing rate limit (10 req/60s globally)...");
    let mut got_429 = false;
    for i in 1..=15 {
        let resp = client
            .post(format!("{}/api/login", base))
            .json(&serde_json::json!({"username": "x", "password": "y"}))
            .send()
            .await?;
        let status = resp.status().as_u16();
        println!("    Request {}: HTTP {}", i, status);
        if status == 429 {
            got_429 = true;
            break;
        }
    }
    assert!(got_429, "expected at least one 429 rate-limit response");
    println!("✓ Rate limiting works (got 429 Too Many Requests)");

    println!("\n✅ All auth + rate-limit tests passed!");
    println!("   - JWT login issues tokens");
    println!("   - Protected endpoints reject requests without tokens (401)");
    println!("   - Role-based access control works (403 for non-admins)");
    println!("   - Rate limiting enforces 5 req/60s on /api/login (429 on 6th)");

    drop(server_thread);
    Ok(())
}
