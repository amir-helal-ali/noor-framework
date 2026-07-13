// ============================================================
// HTTP Request - طلب HTTP
// ============================================================
// Represents an incoming HTTP request with all its components:
// method, URI, headers, body, query params, and parsed body.
//
// يمثل طلب HTTP وارد مع جميع مكوناته.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use crate::core::http::Method;

/// An HTTP request
/// طلب HTTP
#[derive(Debug, Clone)]
pub struct Request {
    pub method: Method,
    pub uri: String,
    pub path: String,
    pub query_string: String,
    pub headers: HashMap<String, String>,
    pub body: Bytes,
    pub query_params: HashMap<String, String>,
    pub route_params: HashMap<String, String>,
    pub cookies: HashMap<String, String>,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
    pub is_secure: bool,
}

impl Request {
    /// Create a new request
    /// إنشاء طلب جديد
    pub fn new(method: Method, uri: String) -> Self {
        let (path, query_string) = if let Some(pos) = uri.find('?') {
            (uri[..pos].to_string(), uri[pos + 1..].to_string())
        } else {
            (uri.clone(), String::new())
        };

        let query_params = if query_string.is_empty() {
            HashMap::new()
        } else {
            serde_urlencoded::from_str(&query_string).unwrap_or_default()
        };

        Self {
            method,
            uri,
            path,
            query_string,
            headers: HashMap::new(),
            body: Bytes::new(),
            query_params,
            route_params: HashMap::new(),
            cookies: HashMap::new(),
            client_ip: None,
            user_agent: None,
            is_secure: false,
        }
    }

    /// Parse cookies from the `Cookie` request header into `self.cookies`.
    /// Called automatically by the server before dispatching; can also be
    /// called manually for testing.
    ///
    /// Cookie header format: `name1=value1; name2=value2`
    pub fn parse_cookies(&mut self) {
        self.cookies.clear();
        // Extract the header value first to avoid borrowing `self` twice.
        let cookie_header = self
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("cookie"))
            .map(|(_, v)| v.clone());
        if let Some(cookie_header) = cookie_header {
            for pair in cookie_header.split(';') {
                let pair = pair.trim();
                if let Some(pos) = pair.find('=') {
                    let name = pair[..pos].trim().to_string();
                    let value = pair[pos + 1..].trim().to_string();
                    if !name.is_empty() {
                        self.cookies.insert(name, value);
                    }
                } else if !pair.is_empty() {
                    self.cookies.insert(pair.to_string(), String::new());
                }
            }
        }
    }
    
    /// Get a header value
    /// الحصول على قيمة header
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
    
    /// Get a query parameter
    /// الحصول على معامل query
    pub fn query(&self, name: &str) -> Option<&str> {
        self.query_params.get(name).map(|s| s.as_str())
    }
    
    /// Get a route parameter
    /// الحصول على معامل route
    pub fn param(&self, name: &str) -> Option<&str> {
        self.route_params.get(name).map(|s| s.as_str())
    }
    
    /// Get a cookie value
    /// الحصول على قيمة cookie
    pub fn cookie(&self, name: &str) -> Option<&str> {
        self.cookies.get(name).map(|s| s.as_str())
    }
    
    /// Parse body as JSON
    /// تحليل الـ body كـ JSON
    pub fn json<T: for<'de> Deserialize<'de>>(&self) -> crate::NoorResult<T> {
        serde_json::from_slice(&self.body)
            .map_err(|e| crate::NoorError::Http(format!("JSON parse error: {}", e)))
    }
    
    /// Parse body as form data
    /// تحليل الـ body كـ form data
    pub fn form(&self) -> HashMap<String, String> {
        let content_type = self.header("content-type").unwrap_or("");
        if content_type.contains("application/x-www-form-urlencoded") {
            serde_urlencoded::from_bytes::<HashMap<String, String>>(&self.body)
                .unwrap_or_default()
        } else {
            HashMap::new()
        }
    }
    
    /// Get the content type
    /// الحصول على نوع المحتوى
    pub fn content_type(&self) -> Option<&str> {
        self.header("content-type")
    }
    
    /// Check if request wants JSON response
    /// فحص إذا كان الطلب يريد استجابة JSON
    pub fn wants_json(&self) -> bool {
        self.header("accept")
            .map(|a| a.contains("application/json"))
            .unwrap_or(false)
    }
    
    /// Get the bearer token from Authorization header
    /// الحصول على Bearer token من header
    pub fn bearer_token(&self) -> Option<&str> {
        self.header("authorization")
            .and_then(|h| h.strip_prefix("Bearer "))
    }
    
    /// Check if request is AJAX
    /// فحص إذا كان الطلب AJAX
    pub fn is_ajax(&self) -> bool {
        self.header("x-requested-with")
            .map(|h| h.eq_ignore_ascii_case("XMLHttpRequest"))
            .unwrap_or(false)
    }
}

/// Builder for constructing requests (used in testing)
/// بناء الطلبات للاختبار
pub struct RequestBuilder {
    request: Request,
}

impl RequestBuilder {
    pub fn new(method: Method, uri: &str) -> Self {
        Self {
            request: Request::new(method, uri.to_string()),
        }
    }
    
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.request.headers.insert(name.to_string(), value.to_string());
        self
    }
    
    pub fn body(mut self, body: impl Into<Bytes>) -> Self {
        self.request.body = body.into();
        self
    }
    
    pub fn json_body<T: Serialize>(mut self, data: &T) -> crate::NoorResult<Self> {
        self.request.body = Bytes::from(serde_json::to_vec(data)?);
        self.request.headers.insert(
            "content-type".to_string(),
            "application/json".to_string(),
        );
        Ok(self)
    }
    
    pub fn build(self) -> Request {
        self.request
    }
}
