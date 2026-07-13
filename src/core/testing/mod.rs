// ============================================================
// Testing Utilities - أدوات الاختبار
// ============================================================
// Helpers for testing Noor applications:
// - Test request builder
// - Test response assertions
// - Test database
// - Mock services
//
// مساعدات لاختبار تطبيقات نور.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};
use crate::core::http::{Request, Response, Method, StatusCode};

/// Test client for making requests to the application
/// عميل اختبار لإجراء الطلبات
pub struct TestClient {
    base_url: String,
    default_headers: HashMap<String, String>,
    auth_token: Option<String>,
}

impl Default for TestClient {
    fn default() -> Self {
        Self::new()
    }
}

impl TestClient {
    pub fn new() -> Self {
        Self {
            base_url: "http://localhost:8080".to_string(),
            default_headers: HashMap::new(),
            auth_token: None,
        }
    }
    
    pub fn with_base_url(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            default_headers: HashMap::new(),
            auth_token: None,
        }
    }
    
    /// Set the auth token (Bearer)
    pub fn with_token(mut self, token: &str) -> Self {
        self.auth_token = Some(token.to_string());
        self
    }
    
    /// Add a default header
    pub fn with_header(mut self, name: &str, value: &str) -> Self {
        self.default_headers.insert(name.to_string(), value.to_string());
        self
    }
    
    /// Build a GET request
    pub fn get(&self, path: &str) -> TestRequestBuilder {
        self.build_request(Method::Get, path)
    }
    
    /// Build a POST request
    pub fn post(&self, path: &str) -> TestRequestBuilder {
        self.build_request(Method::Post, path)
    }
    
    /// Build a PUT request
    pub fn put(&self, path: &str) -> TestRequestBuilder {
        self.build_request(Method::Put, path)
    }
    
    /// Build a DELETE request
    pub fn delete(&self, path: &str) -> TestRequestBuilder {
        self.build_request(Method::Delete, path)
    }
    
    fn build_request(&self, method: Method, path: &str) -> TestRequestBuilder {
        let mut request = Request::new(method, path.to_string());
        
        // Add default headers
        for (k, v) in &self.default_headers {
            request.headers.insert(k.clone(), v.clone());
        }
        
        // Add auth token if set
        if let Some(ref token) = self.auth_token {
            request.headers.insert(
                "authorization".to_string(),
                format!("Bearer {}", token),
            );
        }
        
        TestRequestBuilder { request }
    }
}

/// Test request builder
/// منشئ طلب الاختبار
pub struct TestRequestBuilder {
    request: Request,
}

impl TestRequestBuilder {
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.request.headers.insert(name.to_string(), value.to_string());
        self
    }
    
    pub fn query(mut self, key: &str, value: &str) -> Self {
        self.request.query_params.insert(key.to_string(), value.to_string());
        self
    }
    
    pub fn json_body<T: Serialize>(mut self, body: &T) -> Self {
        let json = serde_json::to_vec(body).unwrap();
        self.request.body = bytes::Bytes::from(json);
        self.request.headers.insert(
            "content-type".to_string(),
            "application/json".to_string(),
        );
        self
    }
    
    pub fn form_body(mut self, data: HashMap<String, String>) -> Self {
        let encoded = serde_urlencoded::to_string(&data).unwrap();
        self.request.body = bytes::Bytes::from(encoded);
        self.request.headers.insert(
            "content-type".to_string(),
            "application/x-www-form-urlencoded".to_string(),
        );
        self
    }
    
    pub fn body(mut self, body: &[u8]) -> Self {
        self.request.body = bytes::Bytes::from(body.to_vec());
        self
    }
    
    pub fn build(self) -> Request {
        self.request
    }
}

/// Test response assertions
/// تأكيدات استجابة الاختبار
pub struct TestResponse {
    response: Response,
}

impl From<Response> for TestResponse {
    fn from(response: Response) -> Self {
        Self { response }
    }
}

impl TestResponse {
    pub fn new(response: Response) -> Self {
        Self { response }
    }
    
    /// Assert status code
    pub fn assert_status(&self, expected: StatusCode) -> &Self {
        assert_eq!(
            self.response.status.0, expected.0,
            "Expected status {} but got {}",
            expected.0, self.response.status.0
        );
        self
    }
    
    /// Assert 200 OK
    pub fn assert_ok(&self) -> &Self {
        self.assert_status(StatusCode::OK)
    }
    
    /// Assert 201 Created
    pub fn assert_created(&self) -> &Self {
        self.assert_status(StatusCode::CREATED)
    }
    
    /// Assert 204 No Content
    pub fn assert_no_content(&self) -> &Self {
        self.assert_status(StatusCode::NO_CONTENT)
    }
    
    /// Assert 401 Unauthorized
    pub fn assert_unauthorized(&self) -> &Self {
        self.assert_status(StatusCode::UNAUTHORIZED)
    }
    
    /// Assert 403 Forbidden
    pub fn assert_forbidden(&self) -> &Self {
        self.assert_status(StatusCode::FORBIDDEN)
    }
    
    /// Assert 404 Not Found
    pub fn assert_not_found(&self) -> &Self {
        self.assert_status(StatusCode::NOT_FOUND)
    }
    
    /// Assert 422 Unprocessable Entity
    pub fn assert_validation_error(&self) -> &Self {
        self.assert_status(StatusCode(422))
    }
    
    /// Assert 500 Internal Server Error
    pub fn assert_server_error(&self) -> &Self {
        self.assert_status(StatusCode::INTERNAL_SERVER_ERROR)
    }
    
    /// Assert header exists
    pub fn assert_header(&self, name: &str) -> &Self {
        assert!(
            self.response.headers
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case(name)),
            "Header '{}' not found in response",
            name
        );
        self
    }
    
    /// Assert header value
    pub fn assert_header_value(&self, name: &str, expected: &str) -> &Self {
        let found = self.response.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v);
        
        assert!(
            found.is_some(),
            "Header '{}' not found",
            name
        );
        assert_eq!(
            found.unwrap(),
            expected,
            "Header '{}' expected '{}' but got '{}'",
            name,
            expected,
            found.unwrap()
        );
        self
    }
    
    /// Assert content type
    pub fn assert_content_type(&self, content_type: &str) -> &Self {
        self.assert_header_value("content-type", content_type)
    }
    
    /// Assert JSON content type
    pub fn assert_json(&self) -> &Self {
        self.assert_header_value("content-type", "application/json; charset=utf-8")
    }
    
    /// Assert HTML content type
    pub fn assert_html(&self) -> &Self {
        self.assert_header_value("content-type", "text/html; charset=utf-8")
    }
    
    /// Get response body as string
    pub fn body_string(&self) -> String {
        String::from_utf8_lossy(&self.response.body).to_string()
    }
    
    /// Get response body as JSON
    pub fn json<T: for<'de> Deserialize<'de>>(&self) -> T {
        serde_json::from_slice(&self.response.body)
            .expect("Failed to parse response body as JSON")
    }
    
    /// Assert body contains string
    pub fn assert_body_contains(&self, needle: &str) -> &Self {
        let body = self.body_string();
        assert!(
            body.contains(needle),
            "Response body does not contain '{}'. Body: {}",
            needle,
            body
        );
        self
    }
    
    /// Get the raw response
    pub fn response(&self) -> &Response {
        &self.response
    }
    
    /// Consume and return the raw response
    pub fn into_response(self) -> Response {
        self.response
    }
}

/// Assertion helpers
pub mod assert {
    use super::*;
    
    /// Assert that a condition is true with a message
    pub fn is_true(condition: bool, message: &str) {
        assert!(condition, "{}", message);
    }
    
    /// Assert that two values are equal
    pub fn equals<T: PartialEq + std::fmt::Debug>(actual: T, expected: T) {
        assert_eq!(actual, expected);
    }
    
    /// Assert that a value is Some
    pub fn is_some<T: std::fmt::Debug>(value: Option<T>) {
        assert!(value.is_some(), "Expected Some but got None");
    }
    
    /// Assert that a value is None
    pub fn is_none<T: std::fmt::Debug>(value: Option<T>) {
        assert!(value.is_none(), "Expected None but got Some({:?})", value);
    }
    
    /// Assert that a string is not empty
    pub fn not_empty(s: &str) {
        assert!(!s.is_empty(), "Expected non-empty string");
    }
    
    /// Assert that a collection is not empty
    pub fn not_empty_collection<T>(coll: &[T]) {
        assert!(!coll.is_empty(), "Expected non-empty collection");
    }
}

/// Mock services for testing
pub mod mock {
    use super::*;
    
    /// Mock cache for testing
    pub struct MockCache {
        data: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    }
    
    impl Default for MockCache {
        fn default() -> Self {
            Self::new()
        }
    }
    
    impl MockCache {
        pub fn new() -> Self {
            Self {
                data: Arc::new(RwLock::new(HashMap::new())),
            }
        }
        
        pub fn get(&self, key: &str) -> Option<Vec<u8>> {
            self.data.read().get(key).cloned()
        }
        
        pub fn set(&self, key: &str, value: &[u8], _ttl: u64) {
            self.data.write().insert(key.to_string(), value.to_vec());
        }
        
        pub fn delete(&self, key: &str) {
            self.data.write().remove(key);
        }
        
        pub fn clear(&self) {
            self.data.write().clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_test_client() {
        let client = TestClient::new();
        let request = client.get("/users").build();
        
        assert_eq!(request.method, Method::Get);
        assert_eq!(request.path, "/users");
    }
    
    #[test]
    fn test_test_client_with_auth() {
        let client = TestClient::new().with_token("my_token");
        let request = client.get("/profile").build();
        
        assert_eq!(
            request.header("authorization"),
            Some("Bearer my_token")
        );
    }
    
    #[test]
    fn test_json_body() {
        let client = TestClient::new();
        let request = client
            .post("/users")
            .json_body(&serde_json::json!({"name": "John"}))
            .build();
        
        assert_eq!(request.content_type(), Some("application/json"));
    }
    
    #[test]
    fn test_test_response_assertions() {
        let response = Response::ok().html("<h1>Hello</h1>");
        let test_response = TestResponse::new(response);
        
        test_response
            .assert_ok()
            .assert_html()
            .assert_body_contains("Hello");
    }
    
    #[test]
    fn test_mock_cache() {
        let cache = mock::MockCache::new();
        cache.set("key1", b"value1", 60);
        
        assert_eq!(cache.get("key1"), Some(b"value1".to_vec()));
        
        cache.delete("key1");
        assert_eq!(cache.get("key1"), None);
    }
}
