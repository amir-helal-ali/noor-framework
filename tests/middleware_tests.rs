// ============================================================
// Integration tests for middleware execution, auth, and throttle.
// ============================================================

use noor::core::auth::Jwt;
use noor::core::http::{Method, Request, Response, StatusCode};
use noor::core::middleware::auth;
use noor::core::middleware::throttle::{ThrottleConfig, ThrottleMiddleware};
use noor::core::router::Router;
use std::sync::Arc;

// ============= Middleware Execution =============

#[test]
fn test_global_middleware_executes_before_handler() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let counter = Arc::new(AtomicUsize::new(0));
    let counter_for_mw = counter.clone();
    let counter_for_handler = counter.clone();

    let mut router = Router::new();

    // Register a middleware that increments the counter.
    router.middleware_stack().register(
        "counter",
        Arc::new(move |req| {
            counter_for_mw.fetch_add(1, Ordering::SeqCst);
            Ok(noor::core::middleware::MiddlewareOutcome::Continue(req))
        }),
    );
    router.use_middleware("counter");

    router.get("/", move |_req| {
        counter_for_handler.fetch_add(10, Ordering::SeqCst);
        Ok(Response::ok().text("hello"))
    });

    let req = Request::new(Method::Get, "/".to_string());
    let resp = router.dispatch(req).unwrap();

    assert_eq!(resp.status, StatusCode::OK);
    // Middleware ran (1) + handler ran (10) = 11.
    assert_eq!(counter.load(Ordering::SeqCst), 11);
}

#[test]
fn test_middleware_can_short_circuit() {
    let mut router = Router::new();

    router.middleware_stack().register(
        "blocker",
        Arc::new(|_req| {
            Ok(noor::core::middleware::MiddlewareOutcome::Stop(
                Response::new(StatusCode::FORBIDDEN).text("blocked"),
            ))
        }),
    );
    router.use_middleware("blocker");

    let handler_called = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let hc = handler_called.clone();
    router.get("/", move |_req| {
        hc.store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(Response::ok().text("should not reach"))
    });

    let req = Request::new(Method::Get, "/".to_string());
    let resp = router.dispatch(req).unwrap();

    assert_eq!(resp.status, StatusCode::FORBIDDEN);
    assert!(!handler_called.load(std::sync::atomic::Ordering::SeqCst));
}

#[test]
fn test_middleware_runs_on_404() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let counter = Arc::new(AtomicUsize::new(0));
    let c = counter.clone();

    let mut router = Router::new();
    router.middleware_stack().register(
        "logger",
        Arc::new(move |req| {
            c.fetch_add(1, Ordering::SeqCst);
            Ok(noor::core::middleware::MiddlewareOutcome::Continue(req))
        }),
    );
    router.use_middleware("logger");

    // No routes registered → 404.
    let req = Request::new(Method::Get, "/nonexistent".to_string());
    let resp = router.dispatch(req).unwrap();

    assert_eq!(resp.status, StatusCode::NOT_FOUND);
    // Middleware should still have run.
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

// ============= Auth Middleware =============

#[test]
fn test_auth_middleware_rejects_missing_token() {
    let jwt = Arc::new(Jwt::new("secret", "noor", "noor_app"));
    let mut router = Router::new();
    auth::register(&mut router, jwt, "auth");
    router.use_middleware("auth");

    router.get("/protected", |_req| Ok(Response::ok().text("secret")));

    let req = Request::new(Method::Get, "/protected".to_string());
    let resp = router.dispatch(req).unwrap();

    assert_eq!(resp.status, StatusCode::UNAUTHORIZED);
}

#[test]
fn test_auth_middleware_accepts_valid_token() {
    let jwt = Arc::new(Jwt::new("secret", "noor", "noor_app"));
    let token = jwt.generate_access_token("user1", vec!["user".to_string()]).unwrap();

    let mut router = Router::new();
    auth::register(&mut router, jwt, "auth");
    router.use_middleware("auth");

    router.get("/protected", |_req| Ok(Response::ok().text("secret")));

    let mut req = Request::new(Method::Get, "/protected".to_string());
    req.headers.insert("authorization".to_string(), format!("Bearer {}", token));

    let resp = router.dispatch(req).unwrap();
    assert_eq!(resp.status, StatusCode::OK);
    assert_eq!(resp.body, bytes::Bytes::from("secret"));
}

#[test]
fn test_auth_middleware_rejects_invalid_token() {
    let jwt = Arc::new(Jwt::new("secret", "noor", "noor_app"));
    let mut router = Router::new();
    auth::register(&mut router, jwt, "auth");
    router.use_middleware("auth");

    router.get("/protected", |_req| Ok(Response::ok().text("secret")));

    let mut req = Request::new(Method::Get, "/protected".to_string());
    req.headers.insert("authorization".to_string(), "Bearer invalid.token.here".to_string());

    let resp = router.dispatch(req).unwrap();
    assert_eq!(resp.status, StatusCode::UNAUTHORIZED);
}

// ============= Throttle Middleware =============

#[test]
fn test_throttle_middleware_blocks_after_limit() {
    let throttle = ThrottleMiddleware::new(ThrottleConfig {
        max_requests: 3,
        window_secs: 60,
        paths: None,
        excluded_ips: vec![],
    });

    let mut router = Router::new();
    noor::core::middleware::throttle::register(&mut router, throttle, "throttle");
    router.use_middleware("throttle");

    router.get("/", |_req| Ok(Response::ok().text("ok")));

    let mut req = Request::new(Method::Get, "/".to_string());
    req.client_ip = Some("10.0.0.1".to_string());

    // First 3 requests should pass.
    for _ in 0..3 {
        let resp = router.dispatch(req.clone()).unwrap();
        assert_eq!(resp.status, StatusCode::OK);
    }

    // 4th should be blocked.
    let resp = router.dispatch(req).unwrap();
    assert_eq!(resp.status, StatusCode::TOO_MANY_REQUESTS);
}

#[test]
fn test_throttle_middleware_independent_per_ip() {
    let throttle = ThrottleMiddleware::new(ThrottleConfig {
        max_requests: 2,
        window_secs: 60,
        paths: None,
        excluded_ips: vec![],
    });

    let mut router = Router::new();
    noor::core::middleware::throttle::register(&mut router, throttle, "throttle");
    router.use_middleware("throttle");

    router.get("/", |_req| Ok(Response::ok().text("ok")));

    // IP 1 exhausts its limit.
    let mut req1 = Request::new(Method::Get, "/".to_string());
    req1.client_ip = Some("10.0.0.1".to_string());
    for _ in 0..2 {
        assert_eq!(router.dispatch(req1.clone()).unwrap().status, StatusCode::OK);
    }
    assert_eq!(
        router.dispatch(req1).unwrap().status,
        StatusCode::TOO_MANY_REQUESTS
    );

    // IP 2 should still be allowed.
    let mut req2 = Request::new(Method::Get, "/".to_string());
    req2.client_ip = Some("10.0.0.2".to_string());
    assert_eq!(router.dispatch(req2).unwrap().status, StatusCode::OK);
}

// ============= Router HEAD/OPTIONS =============

#[test]
fn test_router_head_route() {
    let mut router = Router::new();
    router.head("/", |_req| Ok(Response::ok().text("head")));

    let req = Request::new(Method::Head, "/".to_string());
    let resp = router.dispatch(req).unwrap();
    assert_eq!(resp.status, StatusCode::OK);
}

#[test]
fn test_router_options_route() {
    let mut router = Router::new();
    router.options("/", |_req| Ok(Response::ok().text("options")));

    let req = Request::new(Method::Options, "/".to_string());
    let resp = router.dispatch(req).unwrap();
    assert_eq!(resp.status, StatusCode::OK);
}
