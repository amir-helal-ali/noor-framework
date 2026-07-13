// ============================================================
// Integration tests for sessions and template rendering.
// ============================================================

use noor::core::auth::session::{Session, SessionManager};
use noor::core::http::{Method, Request, Response};
use noor::core::router::Router;
use noor::core::view::ViewEngine;
use std::sync::Arc;

// ============= SessionManager =============

fn test_session_mgr() -> Arc<SessionManager> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = format!("/tmp/noor_session_test_{}_{}", std::process::id(), n);
    std::fs::remove_dir_all(&dir).ok();
    Arc::new(SessionManager::new(&dir, 3600).unwrap())
}

#[test]
fn test_session_create_and_get() {
    let mgr = test_session_mgr();
    let session = mgr.create().unwrap();
    assert!(!session.id.is_empty());

    let retrieved = mgr.get(&session.id).unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().id, session.id);
}

#[test]
fn test_session_set_and_get_value() {
    let mgr = test_session_mgr();
    let session = mgr.create().unwrap();

    mgr.put(&session.id, "user_id", serde_json::json!(42))
        .unwrap();
    mgr.put(&session.id, "username", serde_json::json!("alice"))
        .unwrap();

    let user_id = mgr.get_value(&session.id, "user_id").unwrap();
    assert_eq!(user_id, Some(serde_json::json!(42)));

    let username = mgr.get_value(&session.id, "username").unwrap();
    assert_eq!(username, Some(serde_json::json!("alice")));
}

#[test]
fn test_session_forget_value() {
    let mgr = test_session_mgr();
    let session = mgr.create().unwrap();
    mgr.put(&session.id, "temp", serde_json::json!("data"))
        .unwrap();
    assert!(mgr.get_value(&session.id, "temp").unwrap().is_some());

    mgr.forget(&session.id, "temp").unwrap();
    assert!(mgr.get_value(&session.id, "temp").unwrap().is_none());
}

#[test]
fn test_session_destroy() {
    let mgr = test_session_mgr();
    let session = mgr.create().unwrap();
    assert!(mgr.get(&session.id).unwrap().is_some());

    mgr.destroy(&session.id).unwrap();
    assert!(mgr.get(&session.id).unwrap().is_none());
}

#[test]
fn test_session_gc_cleans_expired() {
    use std::sync::atomic::{AtomicU64, Ordering};
    static GC_COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = GC_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = format!("/tmp/noor_session_gc_test_{}_{}", std::process::id(), n);
    std::fs::remove_dir_all(&dir).ok();
    // 1 second lifetime.
    let mgr = SessionManager::new(&dir, 1).unwrap();

    let s1 = mgr.create().unwrap();
    let s2 = mgr.create().unwrap();
    assert!(mgr.get(&s1.id).unwrap().is_some());

    // Wait for sessions to expire (lifetime is 1 second; sleep 2s to be safe).
    std::thread::sleep(std::time::Duration::from_secs(2));

    let cleaned = mgr.gc().unwrap();
    assert_eq!(cleaned, 2);
    assert!(mgr.get(&s1.id).unwrap().is_none());
    assert!(mgr.get(&s2.id).unwrap().is_none());
}

// ============= Session Facade =============

#[test]
fn test_session_facade_set_get() {
    let mgr = test_session_mgr();
    let mut session = Session::new(mgr.clone());
    let id = session.start().unwrap();

    session.set("count", 5).unwrap();
    let count: i64 = session.get("count").unwrap().unwrap();
    assert_eq!(count, 5);
}

#[test]
fn test_session_facade_get_missing() {
    let mgr = test_session_mgr();
    let mut session = Session::new(mgr);
    session.start().unwrap();

    let val: Option<i64> = session.get("nonexistent").unwrap();
    assert!(val.is_none());
}

#[test]
fn test_session_facade_destroy() {
    let mgr = test_session_mgr();
    let mut session = Session::new(mgr.clone());
    let id = session.start().unwrap();
    session.set("key", "value").unwrap();

    session.destroy().unwrap();
    assert!(mgr.get(&id).unwrap().is_none());
}

// ============= ViewEngine =============

fn test_view_engine() -> Arc<ViewEngine> {
    let dir = format!("/tmp/noor_view_test_{}", std::process::id());
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        format!("{}/greeting.hbs", dir),
        "<h1>Hello, {{name}}!</h1>",
    ).unwrap();
    std::fs::write(
        format!("{}/list.hbs", dir),
        "<ul>{{#each items}}<li>{{this}}</li>{{/each}}</ul>",
    ).unwrap();
    Arc::new(ViewEngine::new(&dir, true, false).unwrap())
}

#[test]
fn test_template_render_basic() {
    let view = test_view_engine();
    let data = serde_json::json!({"name": "World"});
    let html = view.render("greeting", &data).unwrap();
    assert_eq!(html, "<h1>Hello, World!</h1>");
}

#[test]
fn test_template_render_with_loop() {
    let view = test_view_engine();
    let data = serde_json::json!({"items": ["apple", "banana", "cherry"]});
    let html = view.render("list", &data).unwrap();
    assert!(html.contains("<li>apple</li>"));
    assert!(html.contains("<li>banana</li>"));
    assert!(html.contains("<li>cherry</li>"));
}

#[test]
fn test_template_render_response() {
    let view = test_view_engine();
    let data = serde_json::json!({"name": "Alice"});
    let resp = view.render_response("greeting", &data).unwrap();
    assert_eq!(resp.status, noor::core::http::StatusCode::OK);
    assert_eq!(resp.body, bytes::Bytes::from("<h1>Hello, Alice!</h1>"));
}

#[test]
fn test_template_not_found() {
    let view = test_view_engine();
    let data = serde_json::json!({});
    let result = view.render("nonexistent", &data);
    assert!(result.is_err());
}

#[test]
fn test_template_exists() {
    let view = test_view_engine();
    assert!(view.exists("greeting"));
    assert!(!view.exists("nonexistent"));
}

// ============= Session + Cookie in Handler =============

#[test]
fn test_session_in_handler_with_cookie() {
    let mgr = test_session_mgr();
    let mgr_for_handler = mgr.clone();

    let mut router = Router::new();
    router.get("/", move |req: Request| {
        let session_id = req.cookie("session_id").map(|s| s.to_string());
        let mut session = Session::new(mgr_for_handler.clone());
        session.id = session_id;

        if session.id.is_none() {
            let id = session.start()?;
            session.set("visits", 1)?;
            Ok(Response::ok()
                .text("visits=1")
                .cookie("session_id", &id, 3600))
        } else {
            let visits: i64 = session.get("visits")?.unwrap_or(0);
            session.set("visits", visits + 1)?;
            Ok(Response::ok().text(format!("visits={}", visits + 1)))
        }
    });

    // First request — no cookie, creates session.
    let req = Request::new(Method::Get, "/".to_string());
    let resp = router.dispatch(req).unwrap();
    assert_eq!(resp.body, bytes::Bytes::from("visits=1"));
    let cookie_header = resp.headers.get("set-cookie").unwrap();
    assert!(cookie_header.contains("session_id="));

    // Extract the session ID from the cookie.
    let session_id = cookie_header
        .split(';')
        .next()
        .unwrap()
        .split('=')
        .nth(1)
        .unwrap();

    // Second request — with cookie, increments visits.
    let mut req2 = Request::new(Method::Get, "/".to_string());
    req2.headers
        .insert("cookie".to_string(), format!("session_id={}", session_id));
    req2.parse_cookies();
    let resp2 = router.dispatch(req2).unwrap();
    assert_eq!(resp2.body, bytes::Bytes::from("visits=2"));
}
