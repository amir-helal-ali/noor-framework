// ============================================================
// CSRF Protection - الحماية من CSRF
// ============================================================
// Generates and validates CSRF tokens for state-changing
// requests (POST, PUT, DELETE, PATCH).
//
// يولد ويتحقق من رموز CSRF لطللات تعديل الحالة.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use ring::rand::{SecureRandom, SystemRandom};
use crate::core::http::{Method, Request};
use crate::NoorResult;

/// CSRF token manager
/// مدير رموز CSRF
pub struct Csrf {
    tokens: Arc<RwLock<HashMap<String, u64>>>, // token -> expiry timestamp
    token_lifetime: u64,
    rng: SystemRandom,
}

impl Csrf {
    pub fn new(token_lifetime_secs: u64) -> Self {
        Self {
            tokens: Arc::new(RwLock::new(HashMap::new())),
            token_lifetime: token_lifetime_secs,
            rng: SystemRandom::new(),
        }
    }
    
    /// Generate a new CSRF token
    /// توليد رمز CSRF جديد
    pub fn generate_token(&self) -> crate::NoorResult<String> {
        let mut buf = [0u8; 32];
        self.rng
            .fill(&mut buf)
            .map_err(|e| crate::NoorError::Security(format!("RNG error: {}", e)))?;
        
        let token = hex::encode(buf);
        let expiry = chrono::Utc::now().timestamp() as u64 + self.token_lifetime;
        
        self.tokens.write().insert(token.clone(), expiry);
        
        Ok(token)
    }
    
    /// Validate a CSRF token
    /// التحقق من رمز CSRF
    pub fn validate_token(&self, token: &str) -> bool {
        let now = chrono::Utc::now().timestamp() as u64;
        
        let tokens = self.tokens.read();
        if let Some(&expiry) = tokens.get(token) {
            if expiry > now {
                return true;
            }
        }
        false
    }
    
    /// Check if a request needs CSRF protection
    /// فحص إذا كان الطلب يحتاج حماية CSRF
    pub fn needs_protection(method: &Method) -> bool {
        !method.is_safe()
    }
    
    /// Verify CSRF token from request
    /// التحقق من رمز CSRF من الطلب
    pub fn verify_request(&self, request: &Request) -> bool {
        if !Self::needs_protection(&request.method) {
            return true;
        }
        
        // Check header first
        if let Some(token) = request.header("x-csrf-token") {
            return self.validate_token(token);
        }
        
        // Check form data
        let form = request.form();
        if let Some(token) = form.get("_token") {
            return self.validate_token(token);
        }
        
        // Check query params (less secure but sometimes needed)
        if let Some(token) = request.query("_token") {
            return self.validate_token(token);
        }
        
        false
    }
    
    /// Clean expired tokens
    /// تنظيف الرموز المنتهية
    pub fn clean_expired(&self) {
        let now = chrono::Utc::now().timestamp() as u64;
        self.tokens.write().retain(|_, expiry| *expiry > now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_csrf_token_generation() {
        let csrf = Csrf::new(3600);
        let token = csrf.generate_token().unwrap();
        assert!(!token.is_empty());
        assert!(csrf.validate_token(&token));
    }
    
    #[test]
    fn test_csrf_needs_protection() {
        assert!(!Csrf::needs_protection(&Method::Get));
        assert!(Csrf::needs_protection(&Method::Post));
        assert!(Csrf::needs_protection(&Method::Put));
        assert!(Csrf::needs_protection(&Method::Delete));
    }
}
