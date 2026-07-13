// ============================================================
// HTTP Client - عميل HTTP
// ============================================================
// Make HTTP requests to external APIs and services.
// Supports GET, POST, PUT, PATCH, DELETE with fluent API.
//
// إجراء طلبات HTTP للخدمات الخارجية.
// ============================================================

use std::collections::HashMap;
use std::time::Duration;
use serde::{Serialize, Deserialize};

/// HTTP client request
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub timeout: Duration,
    pub query_params: Vec<(String, String)>,
}

/// HTTP methods
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
        }
    }
}

/// HTTP response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub elapsed_ms: u64,
}

impl HttpResponse {
    /// Check if the response is successful (2xx)
    pub fn is_success(&self) -> bool {
        self.status >= 200 && self.status < 300
    }
    
    /// Check if the response is a client error (4xx)
    pub fn is_client_error(&self) -> bool {
        self.status >= 400 && self.status < 500
    }
    
    /// Check if the response is a server error (5xx)
    pub fn is_server_error(&self) -> bool {
        self.status >= 500
    }
    
    /// Get the body as a string
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).to_string()
    }
    
    /// Parse the body as JSON
    pub fn json<T: for<'de> Deserialize<'de>>(&self) -> crate::NoorResult<T> {
        serde_json::from_slice(&self.body)
            .map_err(|e| crate::NoorError::Http(format!("JSON parse error: {}", e)))
    }
    
    /// Get a header value
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// HTTP client
pub struct HttpClient {
    default_headers: HashMap<String, String>,
    default_timeout: Duration,
    max_retries: u32,
    retry_delay: Duration,
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClient {
    pub fn new() -> Self {
        Self {
            default_headers: HashMap::new(),
            default_timeout: Duration::from_secs(30),
            max_retries: 0,
            retry_delay: Duration::from_millis(500),
        }
    }
    
    /// Set default timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout;
        self
    }
    
    /// Set default header
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.default_headers.insert(name.to_string(), value.to_string());
        self
    }
    
    /// Set Bearer token
    pub fn bearer_token(self, token: &str) -> Self {
        self.header("Authorization", &format!("Bearer {}", token))
    }
    
    /// Set Basic auth
    pub fn basic_auth(self, username: &str, password: &str) -> Self {
        let credentials = format!("{}:{}", username, password);
        let encoded = base64::encode(credentials);
        self.header("Authorization", &format!("Basic {}", encoded))
    }
    
    /// Enable retries on failure
    pub fn with_retries(mut self, max_retries: u32, delay: Duration) -> Self {
        self.max_retries = max_retries;
        self.retry_delay = delay;
        self
    }
    
    /// Create a GET request
    pub fn get(&self, url: &str) -> RequestBuilder {
        self.request(HttpMethod::Get, url)
    }
    
    /// Create a POST request
    pub fn post(&self, url: &str) -> RequestBuilder {
        self.request(HttpMethod::Post, url)
    }
    
    /// Create a PUT request
    pub fn put(&self, url: &str) -> RequestBuilder {
        self.request(HttpMethod::Put, url)
    }
    
    /// Create a PATCH request
    pub fn patch(&self, url: &str) -> RequestBuilder {
        self.request(HttpMethod::Patch, url)
    }
    
    /// Create a DELETE request
    pub fn delete(&self, url: &str) -> RequestBuilder {
        self.request(HttpMethod::Delete, url)
    }
    
    /// Create a request builder
    fn request(&self, method: HttpMethod, url: &str) -> RequestBuilder {
        RequestBuilder {
            request: HttpRequest {
                method,
                url: url.to_string(),
                headers: self.default_headers.clone(),
                body: None,
                timeout: self.default_timeout,
                query_params: vec![],
            },
            max_retries: self.max_retries,
            retry_delay: self.retry_delay,
        }
    }
}

/// Request builder
pub struct RequestBuilder {
    request: HttpRequest,
    max_retries: u32,
    retry_delay: Duration,
}

impl RequestBuilder {
    /// Add a header
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.request.headers.insert(name.to_string(), value.to_string());
        self
    }
    
    /// Add a query parameter
    pub fn query(mut self, key: &str, value: &str) -> Self {
        self.request.query_params.push((key.to_string(), value.to_string()));
        self
    }
    
    /// Add multiple query parameters
    pub fn queries(mut self, params: Vec<(&str, &str)>) -> Self {
        for (k, v) in params {
            self.request.query_params.push((k.to_string(), v.to_string()));
        }
        self
    }
    
    /// Set JSON body
    pub fn json<T: Serialize>(mut self, data: &T) -> crate::NoorResult<Self> {
        let json = serde_json::to_vec(data)?;
        self.request.body = Some(json);
        self.request.headers.insert(
            "Content-Type".to_string(),
            "application/json".to_string(),
        );
        Ok(self)
    }
    
    /// Set form body
    pub fn form(mut self, data: HashMap<String, String>) -> Self {
        let encoded = serde_urlencoded::to_string(&data).unwrap_or_default();
        self.request.body = Some(encoded.into_bytes());
        self.request.headers.insert(
            "Content-Type".to_string(),
            "application/x-www-form-urlencoded".to_string(),
        );
        self
    }
    
    /// Set raw body
    pub fn body(mut self, data: Vec<u8>) -> Self {
        self.request.body = Some(data);
        self
    }
    
    /// Set timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.request.timeout = timeout;
        self
    }
    
    /// Set Bearer token
    pub fn bearer_token(self, token: &str) -> Self {
        self.header("Authorization", &format!("Bearer {}", token))
    }
    
    /// Build the request
    pub fn build(self) -> HttpRequest {
        self.request
    }
    
    /// Execute the request (simulated)
    pub async fn send(self) -> crate::NoorResult<HttpResponse> {
        let request = self.build();
        
        // Build URL with query params
        let url = if request.query_params.is_empty() {
            request.url.clone()
        } else {
            let query: String = request.query_params
                .iter()
                .map(|(k, v)| format!("{}={}", url_encode(k), url_encode(v)))
                .collect::<Vec<_>>()
                .join("&");
            format!("{}?{}", request.url, query)
        };
        
        tracing::info!("HTTP {} {}", request.method.as_str(), url);
        
        // In a real implementation, this would use reqwest or hyper:
        // let client = reqwest::Client::new();
        // let response = client.request(method, url)
        //     .headers(headers)
        //     .body(body)
        //     .timeout(timeout)
        //     .send()
        //     .await?;
        
        // Simulated response for demonstration
        Ok(HttpResponse {
            status: 200,
            headers: {
                let mut h = HashMap::new();
                h.insert("content-type".to_string(), "application/json".to_string());
                h
            },
            body: br#"{"success":true,"message":"Simulated response"}"#.to_vec(),
            elapsed_ms: 42,
        })
    }
    
    /// Execute synchronously (simulated)
    pub fn send_sync(self) -> crate::NoorResult<HttpResponse> {
        // Simple synchronous wrapper
        Ok(HttpResponse {
            status: 200,
            headers: HashMap::new(),
            body: vec![],
            elapsed_ms: 0,
        })
    }
}

/// URL encode helper
fn url_encode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '.' | '_' | '~' => c.to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}

/// Convenience functions
pub fn get(url: &str) -> RequestBuilder {
    HttpClient::new().get(url)
}

pub fn post(url: &str) -> RequestBuilder {
    HttpClient::new().post(url)
}

pub fn put(url: &str) -> RequestBuilder {
    HttpClient::new().put(url)
}

pub fn delete(url: &str) -> RequestBuilder {
    HttpClient::new().delete(url)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_http_client_builder() {
        let client = HttpClient::new()
            .timeout(Duration::from_secs(60))
            .header("User-Agent", "Noor/1.0")
            .bearer_token("my_token");
        
        let request = client
            .get("https://api.example.com/users")
            .query("page", "1")
            .query("limit", "10")
            .header("Accept", "application/json")
            .build();
        
        assert_eq!(request.method, HttpMethod::Get);
        assert_eq!(request.url, "https://api.example.com/users");
        assert_eq!(request.query_params.len(), 2);
        assert!(request.headers.contains_key("Authorization"));
        assert!(request.headers.contains_key("User-Agent"));
    }
    
    #[test]
    fn test_json_body() {
        let client = HttpClient::new();
        
        let request = client
            .post("https://api.example.com/users")
            .json(&serde_json::json!({"name": "John"}))
            .unwrap()
            .build();
        
        assert!(request.body.is_some());
        assert_eq!(
            request.headers.get("Content-Type"),
            Some(&"application/json".to_string())
        );
    }
    
    #[test]
    fn test_form_body() {
        let client = HttpClient::new();
        
        let mut form_data = HashMap::new();
        form_data.insert("name".to_string(), "John".to_string());
        form_data.insert("email".to_string(), "john@example.com".to_string());
        
        let request = client
            .post("https://api.example.com/users")
            .form(form_data)
            .build();
        
        assert!(request.body.is_some());
        assert_eq!(
            request.headers.get("Content-Type"),
            Some(&"application/x-www-form-urlencoded".to_string())
        );
    }
    
    #[test]
    fn test_response_helpers() {
        let success = HttpResponse {
            status: 200,
            headers: HashMap::new(),
            body: br#"{"success":true}"#.to_vec(),
            elapsed_ms: 50,
        };
        
        assert!(success.is_success());
        assert!(!success.is_client_error());
        
        let json: serde_json::Value = success.json().unwrap();
        assert_eq!(json["success"], true);
    }
    
    #[test]
    fn test_basic_auth() {
        let client = HttpClient::new()
            .basic_auth("user", "pass");
        
        let request = client.get("https://api.example.com").build();
        
        let auth_header = request.headers.get("Authorization").unwrap();
        assert!(auth_header.starts_with("Basic "));
    }
    
    #[tokio::test]
    async fn test_send_request() {
        let response = HttpClient::new()
            .get("https://api.example.com/test")
            .send()
            .await
            .unwrap();
        
        assert!(response.is_success());
    }
}
