// ============================================================
// Integration tests for cookies, body limits, and request parsing.
// ============================================================

use noor::core::http::{Method, Request, Response, StatusCode};
use noor::core::router::Router;

// ============= Cookie Parsing =============

#[test]
fn test_parse_cookies_from_header() {
    let mut req = Request::new(Method::Get, "/".to_string());
    req.headers
        .insert("cookie".to_string(), "session=abc123; theme=dark".to_string());
    req.parse_cookies();

    assert_eq!(req.cookie("session"), Some("abc123"));
    assert_eq!(req.cookie("theme"), Some("dark"));
    assert_eq!(req.cookie("nonexistent"), None);
}

#[test]
fn test_parse_cookies_empty_header() {
    let mut req = Request::new(Method::Get, "/".to_string());
    req.parse_cookies();
    assert!(req.cookies.is_empty());
}

#[test]
fn test_parse_cookies_with_spaces() {
    let mut req = Request::new(Method::Get, "/".to_string());
    req.headers.insert(
        "cookie".to_string(),
        "  name=value  ;  foo=bar  ".to_string(),
    );
    req.parse_cookies();

    assert_eq!(req.cookie("name"), Some("value"));
    assert_eq!(req.cookie("foo"), Some("bar"));
}

#[test]
fn test_parse_cookies_flag_cookie() {
    let mut req = Request::new(Method::Get, "/".to_string());
    req.headers
        .insert("cookie".to_string(), "secure; foo=bar".to_string());
    req.parse_cookies();

    assert_eq!(req.cookie("secure"), Some(""));
    assert_eq!(req.cookie("foo"), Some("bar"));
}

#[test]
fn test_cookie_access_in_handler() {
    let mut router = Router::new();
    router.get("/", |req: Request| {
        let session = req.cookie("session").unwrap_or("none");
        Ok(Response::ok().text(format!("session={}", session)))
    });

    let mut req = Request::new(Method::Get, "/".to_string());
    req.headers
        .insert("cookie".to_string(), "session=xyz789".to_string());
    req.parse_cookies();

    let resp = router.dispatch(req).unwrap();
    assert_eq!(resp.status, StatusCode::OK);
    assert_eq!(resp.body, bytes::Bytes::from("session=xyz789"));
}

// ============= Response Cookies =============

#[test]
fn test_response_set_cookie() {
    let resp = Response::ok().cookie("session", "abc123", 3600);
    // The cookie header should contain the name=value and attributes.
    let cookie_header = resp
        .headers
        .get("set-cookie")
        .expect("set-cookie header should exist");
    assert!(cookie_header.contains("session=abc123"));
    assert!(cookie_header.contains("Max-Age=3600"));
    assert!(cookie_header.contains("HttpOnly"));
    assert!(cookie_header.contains("SameSite=Strict"));
}

// ============= Query Params =============

#[test]
fn test_query_params_parsed() {
    let req = Request::new(Method::Get, "/search?q=rust&page=2".to_string());
    assert_eq!(req.query("q"), Some("rust"));
    assert_eq!(req.query("page"), Some("2"));
    assert_eq!(req.query("missing"), None);
}

#[test]
fn test_query_params_empty() {
    let req = Request::new(Method::Get, "/".to_string());
    assert!(req.query_params.is_empty());
}

// ============= Route Params =============

#[test]
fn test_route_params_extracted() {
    let mut router = Router::new();
    router.get("/users/{id}", |req: Request| {
        let id = req.param("id").unwrap_or("unknown");
        Ok(Response::ok().text(format!("user_id={}", id)))
    });

    let req = Request::new(Method::Get, "/users/42".to_string());
    let resp = router.dispatch(req).unwrap();
    assert_eq!(resp.body, bytes::Bytes::from("user_id=42"));
}

#[test]
fn test_route_params_multiple() {
    let mut router = Router::new();
    router.get("/posts/{category}/{slug}", |req: Request| {
        Ok(Response::ok().text(format!(
            "{}/{}",
            req.param("category").unwrap_or(""),
            req.param("slug").unwrap_or("")
        )))
    });

    let req = Request::new(Method::Get, "/posts/rust/getting-started".to_string());
    let resp = router.dispatch(req).unwrap();
    assert_eq!(resp.body, bytes::Bytes::from("rust/getting-started"));
}

// ============= JSON Body =============

#[test]
fn test_json_body_parsing() {
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct User {
        name: String,
        age: u32,
    }

    let mut router = Router::new();
    router.post("/users", |req: Request| {
        let user: User = req.json()?;
        Ok(Response::ok().text(format!("{}-{}", user.name, user.age)))
    });

    let mut req = Request::new(Method::Post, "/users".to_string());
    req.body = bytes::Bytes::from(r#"{"name":"Alice","age":30}"#);

    let resp = router.dispatch(req).unwrap();
    assert_eq!(resp.body, bytes::Bytes::from("Alice-30"));
}

// ============= Form Body =============

#[test]
fn test_form_body_parsing() {
    let mut router = Router::new();
    router.post("/login", |req: Request| {
        let form = req.form();
        let username = form.get("username").map(|s| s.as_str()).unwrap_or("");
        let password = form.get("password").map(|s| s.as_str()).unwrap_or("");
        Ok(Response::ok().text(format!("{}:{}", username, password)))
    });

    let mut req = Request::new(Method::Post, "/login".to_string());
    req.headers.insert(
        "content-type".to_string(),
        "application/x-www-form-urlencoded".to_string(),
    );
    req.body = bytes::Bytes::from("username=alice&password=secret");

    let resp = router.dispatch(req).unwrap();
    assert_eq!(resp.body, bytes::Bytes::from("alice:secret"));
}

// ============= Bearer Token =============

#[test]
fn test_bearer_token_extraction() {
    let mut router = Router::new();
    router.get("/", |req: Request| {
        let token = req.bearer_token().unwrap_or("none");
        Ok(Response::ok().text(token))
    });

    let mut req = Request::new(Method::Get, "/".to_string());
    req.headers
        .insert("authorization".to_string(), "Bearer my-jwt-token".to_string());

    let resp = router.dispatch(req).unwrap();
    assert_eq!(resp.body, bytes::Bytes::from("my-jwt-token"));
}

#[test]
fn test_bearer_token_missing() {
    let mut router = Router::new();
    router.get("/", |req: Request| {
        let token = req.bearer_token().unwrap_or("none");
        Ok(Response::ok().text(token))
    });

    let req = Request::new(Method::Get, "/".to_string());
    let resp = router.dispatch(req).unwrap();
    assert_eq!(resp.body, bytes::Bytes::from("none"));
}

// ============= Redirect =============

#[test]
fn test_redirect_response() {
    let resp = Response::redirect("/new-path");
    assert_eq!(resp.status, StatusCode::FOUND);
    assert_eq!(
        resp.headers.get("location").map(|s| s.as_str()),
        Some("/new-path")
    );
}

#[test]
fn test_redirect_permanent_response() {
    let resp = Response::redirect_permanent("/permanent");
    assert_eq!(resp.status, StatusCode::MOVED_PERMANENTLY);
    assert_eq!(
        resp.headers.get("location").map(|s| s.as_str()),
        Some("/permanent")
    );
}

// ============= 404 Handler =============

#[test]
fn test_custom_404_handler() {
    let mut router = Router::new();
    router.not_found(|_req| {
        Ok(Response::new(StatusCode::NOT_FOUND).html("<h1>Custom 404</h1>"))
    });

    let req = Request::new(Method::Get, "/nonexistent".to_string());
    let resp = router.dispatch(req).unwrap();
    assert_eq!(resp.status, StatusCode::NOT_FOUND);
    assert!(resp.body.starts_with(b"<h1>Custom 404</h1>"));
}
