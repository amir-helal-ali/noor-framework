// ============================================================
// HTTP Response - استجابة HTTP
// ============================================================

use std::collections::HashMap;
use bytes::Bytes;
use serde::Serialize;
use crate::core::http::StatusCode;

/// An HTTP response
/// استجابة HTTP
#[derive(Debug, Clone)]
pub struct Response {
    pub status: StatusCode,
    pub headers: HashMap<String, String>,
    pub body: Bytes,
    pub version: f32,
}

impl Default for Response {
    fn default() -> Self {
        Self::new(StatusCode::OK)
    }
}

impl Response {
    /// Create a new response with a status code
    /// إنشاء استجابة جديدة برمز حالة
    pub fn new(status: StatusCode) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body: Bytes::new(),
            version: 1.1,
        }
    }
    
    /// Create a 200 OK response
    /// إنشاء استجابة 200 OK
    pub fn ok() -> Self {
        Self::new(StatusCode::OK)
    }
    
    /// Create a 404 Not Found response
    /// إنشاء استجابة 404
    pub fn not_found() -> Self {
        Self::new(StatusCode::NOT_FOUND)
    }
    
    /// Create a 500 Internal Server Error response
    /// إنشاء استجابة 500
    pub fn server_error() -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR)
    }
    
    /// Set the response body
    /// تعيين الـ body
    pub fn body(mut self, body: impl Into<Bytes>) -> Self {
        self.body = body.into();
        self
    }
    
    /// Set the response body as text
    /// تعيين الـ body كنص
    pub fn text(mut self, text: impl Into<String>) -> Self {
        let s = text.into();
        self.headers.insert(
            "content-type".to_string(),
            "text/plain; charset=utf-8".to_string(),
        );
        self.body = Bytes::from(s);
        self
    }
    
    /// Set the response body as HTML
    /// تعيين الـ body كـ HTML
    pub fn html(mut self, html: impl Into<String>) -> Self {
        let s = html.into();
        self.headers.insert(
            "content-type".to_string(),
            "text/html; charset=utf-8".to_string(),
        );
        self.body = Bytes::from(s);
        self
    }
    
    /// Set the response body as JSON
    /// تعيين الـ body كـ JSON
    pub fn json<T: Serialize>(mut self, data: &T) -> crate::NoorResult<Self> {
        let json = serde_json::to_vec(data)?;
        self.headers.insert(
            "content-type".to_string(),
            "application/json; charset=utf-8".to_string(),
        );
        self.body = Bytes::from(json);
        Ok(self)
    }
    
    /// Set a header
    /// تعيين header
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.insert(name.to_string(), value.to_string());
        self
    }
    
    /// Set the status code
    /// تعيين رمز الحالة
    pub fn status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }
    
    /// Set a cookie
    /// تعيين cookie
    pub fn cookie(mut self, name: &str, value: &str, max_age_secs: u64) -> Self {
        let cookie = format!(
            "{}={}; Path=/; Max-Age={}; HttpOnly; SameSite=Strict",
            name, value, max_age_secs
        );
        self.headers
            .entry("set-cookie".to_string())
            .and_modify(|existing| {
                existing.push_str(&format!(", {}", cookie));
            })
            .or_insert(cookie);
        self
    }
    
    /// Create a redirect response
    /// إنشاء استجابة إعادة توجيه
    pub fn redirect(to: &str) -> Self {
        Self::new(StatusCode::FOUND).header("location", to)
    }
    
    /// Create a redirect response with permanent status
    /// إنشاء استجابة إعادة توجيه دائمة
    pub fn redirect_permanent(to: &str) -> Self {
        Self::new(StatusCode::MOVED_PERMANENTLY).header("location", to)
    }
    
    /// Add security headers for hardening
    /// إضافة headers أمان
    pub fn secure_headers(mut self) -> Self {
        self.headers.insert(
            "x-content-type-options".to_string(),
            "nosniff".to_string(),
        );
        self.headers.insert(
            "x-frame-options".to_string(),
            "DENY".to_string(),
        );
        self.headers.insert(
            "x-xss-protection".to_string(),
            "1; mode=block".to_string(),
        );
        self.headers.insert(
            "strict-transport-security".to_string(),
            "max-age=31536000; includeSubDomains".to_string(),
        );
        self.headers.insert(
            "referrer-policy".to_string(),
            "strict-origin-when-cross-origin".to_string(),
        );
        self.headers.insert(
            "content-security-policy".to_string(),
            "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'".to_string(),
        );
        self
    }
    
    /// Render the response to raw HTTP bytes
    /// تحويل الاستجابة إلى بايتات HTTP
    pub fn to_http(&self) -> Bytes {
        let mut output = Vec::new();
        
        // Status line
        output.extend_from_slice(
            format!("HTTP/1.1 {} {}\r\n", self.status.0, self.status.reason_phrase()).as_bytes(),
        );
        
        // Headers
        let body_len = self.body.len();
        let has_content_length = self.headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("content-length"));
        
        for (k, v) in &self.headers {
            output.extend_from_slice(format!("{}: {}\r\n", k, v).as_bytes());
        }
        
        if !has_content_length {
            output.extend_from_slice(format!("Content-Length: {}\r\n", body_len).as_bytes());
        }
        
        output.extend_from_slice(b"\r\n");
        output.extend_from_slice(&self.body);
        
        Bytes::from(output)
    }
}
