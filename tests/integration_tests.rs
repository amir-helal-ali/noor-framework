// ============================================================
// Integration Tests - اختبارات التكامل
// ============================================================
// Tests that verify the framework's components work together correctly.
// اختبارات للتحقق من أن مكونات الإطار تعمل معاً بشكل صحيح.
// ============================================================

use noor::{
    core::http::{Method, Request, Response, StatusCode},
    core::router::Router,
    core::security::{Csrf, Xss, RateLimit, Encryption, Validator},
    core::cache::{FileCache, MemoryCache, Cache, CacheManager},
    core::auth::{Jwt, Rbac},
    core::orm::QueryBuilder,
    core::events::{EventEmitter, Event},
    core::queue::{Queue, Job, Priority},
    core::upload::{FileUploader, UploadConfig},
    core::testing::{TestClient, TestResponse},
};

// ============= HTTP Tests =============

#[test]
fn test_request_builder() {
    let client = TestClient::new();
    let request = client
        .get("/users/123")
        .header("x-custom", "value")
        .query("filter", "active")
        .build();
    
    assert_eq!(request.method, Method::Get);
    assert_eq!(request.path, "/users/123");
    assert_eq!(request.header("x-custom"), Some("value"));
    assert_eq!(request.query("filter"), Some("active"));
}

#[test]
fn test_response_json() {
    let response = Response::ok()
        .json(&serde_json::json!({"message": "hello"}))
        .unwrap();
    
    let test_response = TestResponse::new(response);
    test_response
        .assert_ok()
        .assert_json();
    
    let body: serde_json::Value = test_response.json();
    assert_eq!(body["message"], "hello");
}

// ============= Router Tests =============

#[test]
fn test_router_basic() {
    let mut router = Router::new();
    
    router.get("/", |_req| {
        Ok(Response::ok().html("Home"))
    });
    
    router.get("/users/{id}", |req: Request| {
        let id = req.param("id").unwrap_or("0");
        Ok(Response::ok().text(id))
    });
    
    let routes = router.routes();
    assert_eq!(routes.len(), 2);
}

#[test]
fn test_router_match() {
    let mut router = Router::new();
    
    router.get("/posts/{id}", |_req| {
        Ok(Response::ok().text("Post"))
    });
    
    let (route, params) = router.match_route(&Method::Get, "/posts/42").unwrap();
    assert_eq!(route.path, "/posts/{id}");
    assert_eq!(params.get("id"), Some(&"42".to_string()));
}

#[test]
fn test_router_not_found() {
    let router = Router::new();
    let result = router.match_route(&Method::Get, "/nonexistent");
    assert!(result.is_none());
}

// ============= Security Tests =============

#[test]
fn test_csrf_protection() {
    let csrf = Csrf::new(3600);
    
    let token = csrf.generate_token().unwrap();
    assert!(!token.is_empty());
    assert!(csrf.validate_token(&token));
    
    // Invalid token should fail
    assert!(!csrf.validate_token("invalid_token"));
}

#[test]
fn test_xss_protection() {
    // Test escaping
    let escaped = Xss::escape("<script>alert('xss')</script>");
    assert!(escaped.contains("&lt;script&gt;"));
    assert!(!escaped.contains("<script>"));
    
    // Test sanitization
    let xss = Xss::new();
    let cleaned = xss.sanitize("<script>bad</script><p>good</p>");
    assert!(!cleaned.contains("<script>"));
    assert!(cleaned.contains("<p>good</p>"));
    
    // Test safety check
    assert!(Xss::is_safe("hello world"));
    assert!(!Xss::is_safe("<script>alert(1)</script>"));
    assert!(!Xss::is_safe("javascript:alert(1)"));
}

#[test]
fn test_rate_limiting() {
    let limiter = RateLimit::new(3, 60);
    let ip = "192.168.1.100";
    
    assert!(limiter.check(ip).allowed);
    assert!(limiter.check(ip).allowed);
    assert!(limiter.check(ip).allowed);
    
    // 4th request should be blocked
    let result = limiter.check(ip);
    assert!(!result.allowed);
    assert_eq!(result.remaining, 0);
}

#[test]
fn test_encryption_password_hashing() {
    let password = "my_secure_password_123!";
    
    let hash = Encryption::hash_password(password).unwrap();
    assert!(!hash.is_empty());
    
    // Correct password should verify
    assert!(Encryption::verify_password(password, &hash));
    
    // Wrong password should fail
    assert!(!Encryption::verify_password("wrong_password", &hash));
}

#[test]
fn test_encryption_aes() {
    let enc = Encryption::new();
    let key = enc.generate_key().unwrap();
    
    let plaintext = b"sensitive data to encrypt";
    let ciphertext = enc.encrypt(plaintext, &key).unwrap();
    
    // Ciphertext should be different from plaintext
    assert_ne!(plaintext.as_slice(), ciphertext.as_slice());
    
    // Decrypt should return original
    let decrypted = enc.decrypt(&ciphertext, &key).unwrap();
    assert_eq!(plaintext.as_slice(), decrypted.as_slice());
}

#[test]
fn test_encryption_sha256() {
    let hash = Encryption::sha256_hex(b"hello");
    assert_eq!(hash, "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824");
}

#[test]
fn test_validator_email() {
    assert!(Validator::is_email("user@example.com"));
    assert!(Validator::is_email("test.user@domain.org"));
    assert!(!Validator::is_email("invalid"));
    assert!(!Validator::is_email("user@"));
    assert!(!Validator::is_email("@domain.com"));
}

#[test]
fn test_validator_password_strength() {
    assert!(Validator::is_strong_password("Str0ng!Pass"));
    assert!(Validator::is_strong_password("MyP@ssw0rd2024"));
    assert!(!Validator::is_strong_password("weak"));
    assert!(!Validator::is_strong_password("nouppercase1!"));
    assert!(!Validator::is_strong_password("NoNumbers!"));
}

#[test]
fn test_validator_required() {
    assert!(Validator::required("", "name").is_err());
    assert!(Validator::required("   ", "name").is_err());
    assert!(Validator::required("value", "name").is_ok());
}

// ============= JWT Tests =============

#[test]
fn test_jwt_generation_and_verification() {
    let jwt = Jwt::new("super_secret_key", "noor", "noor_app");
    
    let token = jwt.generate_access_token("user123", vec!["admin".to_string()]).unwrap();
    
    let claims = jwt.verify(&token).unwrap();
    assert_eq!(claims.sub, "user123");
    assert_eq!(claims.typ, "access");
    assert!(claims.roles.contains(&"admin".to_string()));
}

#[test]
fn test_jwt_revocation() {
    let jwt = Jwt::new("secret", "noor", "noor_app");
    
    let token = jwt.generate_access_token("user123", vec![]).unwrap();
    
    // Token should be valid
    assert!(jwt.verify(&token).is_ok());
    
    // Revoke token
    jwt.revoke(&token);
    
    // Token should now be invalid
    assert!(jwt.verify(&token).is_err());
}

#[test]
fn test_jwt_invalid_signature() {
    let jwt1 = Jwt::new("secret1", "noor", "noor_app");
    let jwt2 = Jwt::new("secret2", "noor", "noor_app");
    
    let token = jwt1.generate_access_token("user", vec![]).unwrap();
    
    // Verifying with different secret should fail
    assert!(jwt2.verify(&token).is_err());
}

// ============= RBAC Tests =============

#[test]
fn test_rbac_permissions() {
    let rbac = Rbac::new();
    
    rbac.assign_role("user1", "admin");
    
    assert!(rbac.can("user1", "users.read"));
    assert!(rbac.can("user1", "posts.write"));
    assert!(rbac.can("user1", "posts.delete"));
    assert!(!rbac.can("user1", "nonexistent.permission"));
}

#[test]
fn test_rbac_super_admin() {
    let rbac = Rbac::new();
    
    rbac.assign_role("super", "super_admin");
    
    // Super admin should have all permissions
    assert!(rbac.can("super", "anything"));
    assert!(rbac.can("super", "everything.here"));
}

#[test]
fn test_rbac_role_checking() {
    let rbac = Rbac::new();
    
    rbac.assign_role("user1", "editor");
    
    assert!(rbac.has_role("user1", "editor"));
    assert!(!rbac.has_role("user1", "admin"));
}

// ============= Cache Tests =============

#[test]
fn test_memory_cache() {
    let cache = MemoryCache::new(100, "test:");
    
    cache.set("key1", b"value1", 60).unwrap();
    assert_eq!(cache.get("key1"), Some(b"value1".to_vec()));
    
    cache.delete("key1").unwrap();
    assert_eq!(cache.get("key1"), None);
}

#[test]
fn test_cache_manager_remember() {
    let cache = CacheManager::memory_only(100);
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    let counter = std::sync::Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();
    
    // First call should compute
    let result1: i32 = cache.remember("test_key", 60, || {
        counter_clone.fetch_add(1, Ordering::SeqCst);
        Ok(42)
    }).unwrap();
    
    assert_eq!(result1, 42);
    assert_eq!(counter.load(Ordering::SeqCst), 1);
    
    // Second call should use cache
    let result2: i32 = cache.remember("test_key", 60, || {
        counter_clone.fetch_add(1, Ordering::SeqCst);
        Ok(99)
    }).unwrap();
    
    assert_eq!(result2, 42); // Cached value, not 99
    assert_eq!(counter.load(Ordering::SeqCst), 1); // Counter didn't increase
}

// ============= ORM Tests =============

#[test]
fn test_query_builder_select() {
    let (sql, params) = QueryBuilder::table("users")
        .select(&["id", "name", "email"])
        .where_("status", "=", "active")
        .where_("age", ">=", 18)
        .order_by("created_at", "desc")
        .limit(10)
        .to_sql();
    
    assert!(sql.contains("SELECT id, name, email FROM users"));
    assert!(sql.contains("WHERE"));
    assert!(sql.contains("ORDER BY created_at DESC"));
    assert!(sql.contains("LIMIT 10"));
    assert_eq!(params.len(), 2);
}

#[test]
fn test_query_builder_insert() {
    let (sql, params) = QueryBuilder::insert("users")
        .set("name", "John Doe")
        .set("email", "john@example.com")
        .set("age", 30)
        .to_sql();
    
    assert!(sql.contains("INSERT INTO users"));
    assert!(sql.contains("name"));
    assert!(sql.contains("email"));
    assert!(sql.contains("age"));
    assert_eq!(params.len(), 3);
}

#[test]
fn test_query_builder_update() {
    let (sql, params) = QueryBuilder::update("users")
        .set("name", "Jane Doe")
        .where_("id", "=", 1)
        .to_sql();
    
    assert!(sql.contains("UPDATE users SET"));
    assert!(sql.contains("WHERE"));
    assert_eq!(params.len(), 2);
}

// ============= Events Tests =============

#[test]
fn test_event_emitter() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    let emitter = EventEmitter::new();
    let counter = std::sync::Arc::new(AtomicUsize::new(0));
    
    let counter_clone = counter.clone();
    emitter.on("user.registered", std::sync::Arc::new(move |_event| {
        counter_clone.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }));
    
    emitter.fire("user.registered", serde_json::json!({"user_id": 123})).unwrap();
    emitter.fire("other.event", serde_json::json!({})).unwrap();
    
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[test]
fn test_event_wildcard() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    let emitter = EventEmitter::new();
    let counter = std::sync::Arc::new(AtomicUsize::new(0));
    
    let counter_clone = counter.clone();
    emitter.on_any(std::sync::Arc::new(move |_event| {
        counter_clone.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }));
    
    emitter.fire("event1", serde_json::json!({})).unwrap();
    emitter.fire("event2", serde_json::json!({})).unwrap();
    emitter.fire("event3", serde_json::json!({})).unwrap();
    
    assert_eq!(counter.load(Ordering::SeqCst), 3);
}

// ============= Queue Tests =============

#[test]
fn test_queue_basic() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    let queue = Queue::new();
    let counter = std::sync::Arc::new(AtomicUsize::new(0));
    
    let counter_clone = counter.clone();
    queue.register("test_job", std::sync::Arc::new(move |_job| {
        counter_clone.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }));
    
    queue.dispatch("test_job", serde_json::json!({})).unwrap();
    assert_eq!(queue.pending_count(), 1);
    
    let processed = queue.process_next().unwrap();
    assert!(processed);
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[test]
fn test_job_priority() {
    let job_high = Job::new("test", serde_json::json!({}))
        .with_priority(Priority::High);
    let job_low = Job::new("test", serde_json::json!({}))
        .with_priority(Priority::Low);
    
    assert_eq!(job_high.priority, Priority::High);
    assert_eq!(job_low.priority, Priority::Low);
}

// ============= Upload Tests =============

#[test]
fn test_upload_validation() {
    let uploader = FileUploader::new(UploadConfig::default()).unwrap();
    
    // Valid file
    assert!(uploader.validate("photo.jpg", "image/jpeg", 1024).is_ok());
    assert!(uploader.validate("document.pdf", "application/pdf", 5_000_000).is_ok());
    
    // Invalid extension
    assert!(uploader.validate("malware.exe", "application/octet-stream", 1024).is_err());
    
    // Too large
    assert!(uploader.validate("photo.jpg", "image/jpeg", 100_000_000).is_err());
}

#[test]
fn test_file_size_formatting() {
    assert_eq!(FileUploader::format_size(500), "500 B");
    assert_eq!(FileUploader::format_size(1024), "1.00 KB");
    assert_eq!(FileUploader::format_size(1048576), "1.00 MB");
}

// ============= Demo Model Tests =============

#[test]
fn test_post_search() {
    let results = noor::demo::blog::models::Post::search("rust");
    assert!(!results.is_empty());
    
    for post in &results {
        let lower_title = post.title.to_lowercase();
        let lower_content = post.content.to_lowercase();
        let tags_match = post.tags.iter().any(|t| t.to_lowercase().contains("rust"));
        assert!(lower_title.contains("rust") || lower_content.contains("rust") || tags_match);
    }
}

#[test]
fn test_post_popular() {
    let popular = noor::demo::blog::models::Post::popular(3);
    assert!(!popular.is_empty());
    assert!(popular.len() <= 3);
    
    // Should be sorted by views descending
    for i in 0..popular.len()-1 {
        assert!(popular[i].views >= popular[i+1].views);
    }
}

#[test]
fn test_category_operations() {
    let categories = noor::demo::blog::models::Category::all();
    assert!(!categories.is_empty());
    
    let tech = noor::demo::blog::models::Category::find_by_slug("technology");
    assert!(tech.is_some());
    
    let posts_in_tech = noor::demo::blog::models::Post::by_category(tech.unwrap().id);
    assert!(!posts_in_tech.is_empty());
}

#[test]
fn test_comment_operations() {
    let comments = noor::demo::blog::models::Comment::for_post(1);
    assert!(!comments.is_empty());
    
    let count = noor::demo::blog::models::Comment::count_for_post(1);
    assert_eq!(count, comments.len());
}

// ============= Integration Tests =============

#[test]
fn test_full_request_response_cycle() {
    let mut router = Router::new();
    
    router.get("/api/status", |_req| {
        Ok(Response::ok().json(&serde_json::json!({
            "status": "ok",
            "framework": "noor",
        }))?)
    });
    
    let request = Request::new(Method::Get, "/api/status".to_string());
    let response = router.dispatch(request).unwrap();
    
    let test_response = TestResponse::new(response);
    test_response
        .assert_ok()
        .assert_json()
        .assert_body_contains("ok");
    
    let body: serde_json::Value = test_response.json();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["framework"], "noor");
}

#[test]
fn test_security_headers() {
    let response = Response::ok().secure_headers();
    
    assert!(response.headers.contains_key("x-content-type-options"));
    assert!(response.headers.contains_key("x-frame-options"));
    assert!(response.headers.contains_key("x-xss-protection"));
    assert!(response.headers.contains_key("strict-transport-security"));
}
