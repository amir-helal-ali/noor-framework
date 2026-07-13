// ============================================================
// JWT Authentication - مصادقة JWT
// ============================================================
// Secure JSON Web Token implementation with:
// - HS256 signing
// - Refresh tokens
// - Blacklist support
// - Configurable expiry
//
// تنفيذ آمن لـ JWT مع دعم refresh tokens و blacklist.
// ============================================================

use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use parking_lot::RwLock;
use base64::Engine as _;
use crate::core::security::Encryption;

/// JWT claims
/// مطالبات JWT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// Expiration time (Unix timestamp)
    pub exp: i64,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// Issuer
    pub iss: String,
    /// Audience
    pub aud: String,
    /// Token type (access/refresh)
    pub typ: String,
    /// User roles
    pub roles: Vec<String>,
    /// Custom claims
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom: Option<serde_json::Value>,
}

/// JWT manager
/// مدير JWT
pub struct Jwt {
    secret: Vec<u8>,
    issuer: String,
    audience: String,
    access_token_expiry: i64,
    refresh_token_expiry: i64,
    /// Blacklisted tokens (for logout/revocation)
    blacklist: Arc<RwLock<HashSet<String>>>,
}

impl Jwt {
    /// Create a new JWT manager
    /// إنشاء مدير JWT جديد
    pub fn new(secret: &str, issuer: &str, audience: &str) -> Self {
        Self {
            secret: secret.as_bytes().to_vec(),
            issuer: issuer.to_string(),
            audience: audience.to_string(),
            access_token_expiry: 3600, // 1 hour
            refresh_token_expiry: 86400 * 7, // 7 days
            blacklist: Arc::new(RwLock::new(HashSet::new())),
        }
    }
    
    /// Set custom expiry times
    /// تعيين أوقات انتهاء مخصصة
    pub fn with_expiry(mut self, access_secs: i64, refresh_secs: i64) -> Self {
        self.access_token_expiry = access_secs;
        self.refresh_token_expiry = refresh_secs;
        self
    }
    
    /// Generate an access token for a user
    /// توليد access token لمستخدم
    pub fn generate_access_token(&self, user_id: &str, roles: Vec<String>) -> crate::NoorResult<String> {
        let now = Utc::now();
        let claims = Claims {
            sub: user_id.to_string(),
            exp: (now + Duration::seconds(self.access_token_expiry)).timestamp(),
            iat: now.timestamp(),
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            typ: "access".to_string(),
            roles,
            custom: None,
        };
        
        self.encode(&claims)
    }
    
    /// Generate a refresh token
    /// توليد refresh token
    pub fn generate_refresh_token(&self, user_id: &str) -> crate::NoorResult<String> {
        let now = Utc::now();
        let claims = Claims {
            sub: user_id.to_string(),
            exp: (now + Duration::seconds(self.refresh_token_expiry)).timestamp(),
            iat: now.timestamp(),
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            typ: "refresh".to_string(),
            roles: vec![],
            custom: None,
        };
        
        self.encode(&claims)
    }
    
    /// Verify and decode a token
    /// التحقق وفك تشفير الرمز
    pub fn verify(&self, token: &str) -> crate::NoorResult<Claims> {
        // Check blacklist
        if self.blacklist.read().contains(token) {
            return Err(crate::NoorError::Auth("Token has been revoked".to_string()));
        }
        
        let claims = self.decode(token)?;
        
        // Check expiration
        let now = Utc::now().timestamp();
        if claims.exp < now {
            return Err(crate::NoorError::Auth("Token has expired".to_string()));
        }
        
        // Check issuer
        if claims.iss != self.issuer {
            return Err(crate::NoorError::Auth("Invalid token issuer".to_string()));
        }
        
        Ok(claims)
    }
    
    /// Revoke a token (add to blacklist)
    /// إلغاء رمز (إضافته للقائمة السوداء)
    pub fn revoke(&self, token: &str) {
        self.blacklist.write().insert(token.to_string());
    }
    
    /// Encode claims to a JWT string
    /// تشفير المطالبات إلى JWT
    fn encode(&self, claims: &Claims) -> crate::NoorResult<String> {
        let header = serde_json::json!({"alg": "HS256", "typ": "JWT"});
        let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header)?);
        let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(serde_json::to_vec(claims)?);
        
        let signing_input = format!("{}.{}", header_b64, payload_b64);
        let signature = Encryption::hmac_sha256(&self.secret, signing_input.as_bytes())?;
        let signature_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&signature);
        
        Ok(format!("{}.{}", signing_input, signature_b64))
    }
    
    /// Decode a JWT string to claims
    /// فك تشفير JWT إلى مطالبات
    fn decode(&self, token: &str) -> crate::NoorResult<Claims> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(crate::NoorError::Auth("Invalid token format".to_string()));
        }
        
        // Verify signature
        let signing_input = format!("{}.{}", parts[0], parts[1]);
        let expected_signature = Encryption::hmac_sha256(&self.secret, signing_input.as_bytes())?;
        let expected_signature_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&expected_signature);
        
        if !Encryption::constant_time_compare(parts[2].as_bytes(), expected_signature_b64.as_bytes()) {
            return Err(crate::NoorError::Auth("Invalid token signature".to_string()));
        }
        
        // Decode payload
        let payload_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(parts[1])
            .map_err(|e| crate::NoorError::Auth(format!("Decode error: {}", e)))?;
        
        let claims: Claims = serde_json::from_slice(&payload_bytes)?;
        
        Ok(claims)
    }
    
    /// Clean expired tokens from blacklist
    /// تنظيف الرموز المنتهية من القائمة السوداء
    pub fn clean_blacklist(&self) {
        let now = Utc::now().timestamp();
        let blacklist = self.blacklist.read();
        // In a real implementation, we'd need to decode each token to check expiry
        // For now, we just keep the blacklist bounded
        if blacklist.len() > 10000 {
            drop(blacklist);
            self.blacklist.write().clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_jwt_generation_and_verification() {
        let jwt = Jwt::new("secret_key", "noor", "noor_app");
        
        let token = jwt.generate_access_token("user123", vec!["admin".to_string()]).unwrap();
        let claims = jwt.verify(&token).unwrap();
        
        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.typ, "access");
        assert!(claims.roles.contains(&"admin".to_string()));
    }
    
    #[test]
    fn test_jwt_revocation() {
        let jwt = Jwt::new("secret_key", "noor", "noor_app");
        
        let token = jwt.generate_access_token("user123", vec![]).unwrap();
        jwt.revoke(&token);
        
        assert!(jwt.verify(&token).is_err());
    }
}
