// ============================================================
// Cookie Management - إدارة الكوكيز
// ============================================================
// Advanced cookie handling with signing, encryption,
// and expiration management.
//
// إدارة متقدمة للكوكيز مع التوقيع والتشفير.
// ============================================================

use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// Cookie
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub expires: Option<i64>,
    pub max_age: Option<u64>,
    pub domain: Option<String>,
    pub path: Option<String>,
    pub secure: bool,
    pub http_only: bool,
    pub same_site: SameSite,
}

/// SameSite attribute
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SameSite {
    Strict,
    Lax,
    None,
}

impl SameSite {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Strict => "Strict",
            Self::Lax => "Lax",
            Self::None => "None",
        }
    }
}

impl Cookie {
    /// Create a new cookie
    pub fn new(name: &str, value: &str) -> Self {
        Self {
            name: name.to_string(),
            value: value.to_string(),
            expires: None,
            max_age: None,
            domain: None,
            path: Some("/".to_string()),
            secure: false,
            http_only: true,
            same_site: SameSite::Lax,
        }
    }
    
    /// Set expiration timestamp
    pub fn expires(mut self, timestamp: i64) -> Self {
        self.expires = Some(timestamp);
        self
    }
    
    /// Set max age in seconds
    pub fn max_age(mut self, seconds: u64) -> Self {
        self.max_age = Some(seconds);
        self
    }
    
    /// Set domain
    pub fn domain(mut self, domain: &str) -> Self {
        self.domain = Some(domain.to_string());
        self
    }
    
    /// Set path
    pub fn path(mut self, path: &str) -> Self {
        self.path = Some(path.to_string());
        self
    }
    
    /// Mark as secure (HTTPS only)
    pub fn secure(mut self) -> Self {
        self.secure = true;
        self
    }
    
    /// Mark as HTTP only (not accessible via JavaScript)
    pub fn http_only(mut self) -> Self {
        self.http_only = true;
        self
    }
    
    /// Set SameSite attribute
    pub fn same_site(mut self, same_site: SameSite) -> Self {
        self.same_site = same_site;
        self
    }
    
    /// Convert to Set-Cookie header value
    pub fn to_header(&self) -> String {
        let mut header = format!("{}={}", self.name, self.value);
        
        if let Some(ref path) = self.path {
            header.push_str(&format!("; Path={}", path));
        }
        
        if let Some(ref domain) = self.domain {
            header.push_str(&format!("; Domain={}", domain));
        }
        
        if let Some(max_age) = self.max_age {
            header.push_str(&format!("; Max-Age={}", max_age));
        }
        
        if let Some(expires) = self.expires {
            let datetime = chrono::DateTime::from_timestamp(expires, 0);
            if let Some(dt) = datetime {
                header.push_str(&format!("; Expires={}", dt.format("%a, %d %b %Y %H:%M:%S GMT")));
            }
        }
        
        if self.secure {
            header.push_str("; Secure");
        }
        
        if self.http_only {
            header.push_str("; HttpOnly");
        }
        
        header.push_str(&format!("; SameSite={}", self.same_site.as_str()));
        
        header
    }
    
    /// Parse a cookie from a Cookie header value
    pub fn parse(header: &str) -> HashMap<String, String> {
        let mut cookies = HashMap::new();
        
        for pair in header.split(';') {
            let pair = pair.trim();
            
            if let Some(pos) = pair.find('=') {
                let name = pair[..pos].trim().to_string();
                let value = pair[pos + 1..].trim().to_string();
                cookies.insert(name, value);
            }
        }
        
        cookies
    }
    
    /// Create an expired cookie (for deletion)
    pub fn expired(name: &str) -> Self {
        Self::new(name, "")
            .max_age(0)
            .expires(0)
    }
}

/// Cookie jar for managing multiple cookies
#[derive(Debug, Clone, Default)]
pub struct CookieJar {
    cookies: HashMap<String, Cookie>,
}

impl CookieJar {
    pub fn new() -> Self {
        Self {
            cookies: HashMap::new(),
        }
    }
    
    /// Add a cookie
    pub fn add(&mut self, cookie: Cookie) {
        self.cookies.insert(cookie.name.clone(), cookie);
    }
    
    /// Get a cookie by name
    pub fn get(&self, name: &str) -> Option<&Cookie> {
        self.cookies.get(name)
    }
    
    /// Get a cookie value
    pub fn get_value(&self, name: &str) -> Option<&str> {
        self.cookies.get(name).map(|c| c.value.as_str())
    }
    
    /// Remove a cookie
    pub fn remove(&mut self, name: &str) -> Option<Cookie> {
        self.cookies.remove(name)
    }
    
    /// Check if a cookie exists
    pub fn has(&self, name: &str) -> bool {
        self.cookies.contains_key(name)
    }
    
    /// Get all cookies
    pub fn all(&self) -> Vec<&Cookie> {
        self.cookies.values().collect()
    }
    
    /// Convert all cookies to Set-Cookie headers
    pub fn to_headers(&self) -> Vec<String> {
        self.cookies.values().map(|c| c.to_header()).collect()
    }
    
    /// Parse cookies from a Cookie header
    pub fn from_header(header: &str) -> Self {
        let parsed = Cookie::parse(header);
        let mut jar = Self::new();
        
        for (name, value) in parsed {
            jar.add(Cookie::new(&name, &value));
        }
        
        jar
    }
    
    /// Clear all cookies
    pub fn clear(&mut self) {
        self.cookies.clear();
    }
    
    /// Get the count of cookies
    pub fn count(&self) -> usize {
        self.cookies.len()
    }
}

/// Signed cookie manager (prevents tampering)
pub struct SignedCookieManager {
    secret: String,
}

impl SignedCookieManager {
    pub fn new(secret: &str) -> Self {
        Self {
            secret: secret.to_string(),
        }
    }
    
    /// Create a signed cookie
    pub fn sign(&self, name: &str, value: &str) -> crate::NoorResult<Cookie> {
        let signature = crate::core::security::Encryption::hmac_sha256(
            self.secret.as_bytes(),
            value.as_bytes(),
        )?;
        
        let signed_value = format!("{}.{}", value, hex::encode(&signature));
        
        Ok(Cookie::new(name, &signed_value))
    }
    
    /// Verify and extract a signed cookie value
    pub fn verify(&self, cookie: &Cookie) -> Option<String> {
        let parts: Vec<&str> = cookie.value.split('.').collect();
        
        if parts.len() != 2 {
            return None;
        }
        
        let value = parts[0];
        let signature = parts[1];
        
        let expected = crate::core::security::Encryption::hmac_sha256(
            self.secret.as_bytes(),
            value.as_bytes(),
        ).ok()?;
        
        let expected_hex = hex::encode(&expected);
        
        if crate::core::security::Encryption::constant_time_compare(
            signature.as_bytes(),
            expected_hex.as_bytes(),
        ) {
            Some(value.to_string())
        } else {
            None
        }
    }
}

/// Encrypted cookie manager
pub struct EncryptedCookieManager {
    key: [u8; 32],
}

impl EncryptedCookieManager {
    pub fn new(secret: &str) -> Self {
        // Derive a 32-byte key from the secret
        let hash = crate::core::security::Encryption::sha256(secret.as_bytes());
        let mut key = [0u8; 32];
        key.copy_from_slice(&hash);
        
        Self { key }
    }
    
    /// Create an encrypted cookie
    pub fn encrypt(&self, name: &str, value: &str) -> crate::NoorResult<Cookie> {
        use base64::Engine as _;

        let enc = crate::core::security::Encryption::new();
        let ciphertext = enc.encrypt(value.as_bytes(), &self.key)?;

        let encoded = base64::engine::general_purpose::STANDARD.encode(&ciphertext);
        Ok(Cookie::new(name, &encoded))
    }

    /// Decrypt a cookie value
    pub fn decrypt(&self, cookie: &Cookie) -> Option<String> {
        use base64::Engine as _;

        let enc = crate::core::security::Encryption::new();

        let ciphertext = base64::engine::general_purpose::STANDARD
            .decode(&cookie.value)
            .ok()?;
        let plaintext = enc.decrypt(&ciphertext, &self.key).ok()?;

        String::from_utf8(plaintext).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cookie_creation() {
        let cookie = Cookie::new("session", "abc123")
            .max_age(3600)
            .secure()
            .http_only()
            .same_site(SameSite::Strict);
        
        let header = cookie.to_header();
        
        assert!(header.contains("session=abc123"));
        assert!(header.contains("Max-Age=3600"));
        assert!(header.contains("Secure"));
        assert!(header.contains("HttpOnly"));
        assert!(header.contains("SameSite=Strict"));
    }
    
    #[test]
    fn test_cookie_parse() {
        let cookies = Cookie::parse("name=John; theme=dark; lang=ar");
        
        assert_eq!(cookies.get("name"), Some(&"John".to_string()));
        assert_eq!(cookies.get("theme"), Some(&"dark".to_string()));
        assert_eq!(cookies.get("lang"), Some(&"ar".to_string()));
    }
    
    #[test]
    fn test_cookie_jar() {
        let mut jar = CookieJar::new();
        
        jar.add(Cookie::new("user", "john"));
        jar.add(Cookie::new("theme", "dark"));
        
        assert_eq!(jar.count(), 2);
        assert!(jar.has("user"));
        assert_eq!(jar.get_value("user"), Some("john"));
        
        jar.remove("user");
        assert!(!jar.has("user"));
    }
    
    #[test]
    fn test_signed_cookie() {
        let manager = SignedCookieManager::new("secret_key");
        
        let cookie = manager.sign("session", "user123").unwrap();
        
        // Verify should return the original value
        let verified = manager.verify(&cookie).unwrap();
        assert_eq!(verified, "user123");
        
        // Tampered cookie should fail
        let mut tampered = cookie.clone();
        tampered.value = "user456.fake_signature".to_string();
        assert!(manager.verify(&tampered).is_none());
    }
    
    #[test]
    fn test_encrypted_cookie() {
        let manager = EncryptedCookieManager::new("secret_key");
        
        let cookie = manager.encrypt("data", "sensitive_value").unwrap();
        
        // The encrypted value should not contain the plaintext
        assert!(!cookie.value.contains("sensitive_value"));
        
        // Decrypt should return the original value
        let decrypted = manager.decrypt(&cookie).unwrap();
        assert_eq!(decrypted, "sensitive_value");
    }
    
    #[test]
    fn test_expired_cookie() {
        let cookie = Cookie::expired("session");
        let header = cookie.to_header();
        
        assert!(header.contains("Max-Age=0"));
    }
}
