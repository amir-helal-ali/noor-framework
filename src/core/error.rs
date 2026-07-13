// ============================================================
// Error Handling - معالجة الأخطاء
// ============================================================
// Centralized error types used throughout the framework.
// الأخطاء المركزية المستخدمة في جميع أنحاء الإطار.
// ============================================================

use thiserror::Error;

/// The main error type for the Noor Framework
/// النوع الرئيسي للأخطاء في إطار نور
#[derive(Error, Debug)]
pub enum NoorError {
    #[error("HTTP error: {0}")]
    Http(String),
    
    #[error("Router error: {0}")]
    Router(String),
    
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Authentication error: {0}")]
    Auth(String),
    
    #[error("Authorization error: {0}")]
    Authorization(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Security error: {0}")]
    Security(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Cache error: {0}")]
    Cache(String),
    
    #[error("Template error: {0}")]
    Template(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),
    
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type alias for Noor operations
/// نوع النتيجة لعمليات نور
pub type NoorResult<T> = Result<T, NoorError>;

impl NoorError {
    /// Convert error to HTTP status code
    /// تحويل الخطأ إلى رمز حالة HTTP
    pub fn status_code(&self) -> u16 {
        match self {
            NoorError::Auth(_) => 401,
            NoorError::Authorization(_) => 403,
            NoorError::Validation(_) => 422,
            NoorError::Router(_) | NoorError::Http(_) => 404,
            NoorError::Database(_) => 500,
            NoorError::Config(_) => 500,
            _ => 500,
        }
    }
    
    /// Check if this is a client error (4xx)
    /// فحص إذا كان خطأ من العميل
    pub fn is_client_error(&self) -> bool {
        let code = self.status_code();
        code >= 400 && code < 500
    }
    
    /// Check if this is a server error (5xx)
    /// فحص إذا كان خطأ من السيرفر
    pub fn is_server_error(&self) -> bool {
        self.status_code() >= 500
    }
}
