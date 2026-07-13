// ============================================================
// OAuth2 / Social Login - مصادقة OAuth2
// ============================================================
// OAuth2 provider integration for social login
// (Google, GitHub, Facebook, Twitter, etc.)
//
// تكامل OAuth2 لتسجيل الدخول الاجتماعي.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// OAuth2 provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthProvider {
    pub name: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    pub auth_url: String,
    pub token_url: String,
    pub user_info_url: String,
    pub enabled: bool,
}

/// OAuth2 user info from provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthUser {
    pub provider: String,
    pub provider_user_id: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub avatar: Option<String>,
    pub raw_data: serde_json::Value,
}

/// OAuth2 manager
pub struct OAuthManager {
    providers: Arc<RwLock<HashMap<String, OAuthProvider>>>,
}

impl Default for OAuthManager {
    fn default() -> Self {
        Self::new()
    }
}

impl OAuthManager {
    pub fn new() -> Self {
        Self {
            providers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register an OAuth provider
    pub fn register(&self, provider: OAuthProvider) {
        self.providers.write().insert(provider.name.clone(), provider);
    }
    
    /// Get a provider configuration
    pub fn get(&self, name: &str) -> Option<OAuthProvider> {
        self.providers.read().get(name).cloned()
    }
    
    /// List all registered providers
    pub fn list(&self) -> Vec<OAuthProvider> {
        self.providers.read().values().cloned().collect()
    }
    
    /// List enabled providers
    pub fn list_enabled(&self) -> Vec<OAuthProvider> {
        self.providers
            .read()
            .values()
            .filter(|p| p.enabled)
            .cloned()
            .collect()
    }
    
    /// Generate authorization URL
    pub fn authorization_url(&self, provider_name: &str, state: &str) -> crate::NoorResult<String> {
        let provider = self.get(provider_name)
            .ok_or_else(|| crate::NoorError::Auth(format!("Provider '{}' not found", provider_name)))?;
        
        let mut url = format!(
            "{}?response_type=code&client_id={}&redirect_uri={}&state={}",
            provider.auth_url,
            urlencoding::encode(&provider.client_id),
            urlencoding::encode(&provider.redirect_uri),
            urlencoding::encode(state),
        );
        
        if !provider.scopes.is_empty() {
            url.push_str(&format!("&scope={}", urlencoding::encode(&provider.scopes.join(" "))));
        }
        
        Ok(url)
    }
    
    /// Exchange authorization code for access token
    /// (In production, this would make an HTTP request)
    pub fn exchange_code(&self, provider_name: &str, code: &str) -> crate::NoorResult<String> {
        let provider = self.get(provider_name)
            .ok_or_else(|| crate::NoorError::Auth(format!("Provider '{}' not found", provider_name)))?;
        
        // In a real implementation:
        // let client = reqwest::Client::new();
        // let response = client.post(&provider.token_url)
        //     .form(&[
        //         ("grant_type", "authorization_code"),
        //         ("code", code),
        //         ("redirect_uri", &provider.redirect_uri),
        //         ("client_id", &provider.client_id),
        //         ("client_secret", &provider.client_secret),
        //     ])
        //     .send()
        //     .await?
        //     .json::<TokenResponse>()
        //     .await?;
        // Ok(response.access_token)
        
        tracing::info!("Exchanging code for token with provider: {}", provider_name);
        Ok(format!("mock_access_token_{}", code))
    }
    
    /// Get user info from provider
    pub fn get_user_info(&self, provider_name: &str, access_token: &str) -> crate::NoorResult<OAuthUser> {
        let provider = self.get(provider_name)
            .ok_or_else(|| crate::NoorError::Auth(format!("Provider '{}' not found", provider_name)))?;
        
        // In a real implementation:
        // let client = reqwest::Client::new();
        // let response = client.get(&provider.user_info_url)
        //     .bearer_auth(access_token)
        //     .send()
        //     .await?
        //     .json::<serde_json::Value>()
        //     .await?;
        
        // Mock user info for demonstration
        Ok(OAuthUser {
            provider: provider_name.to_string(),
            provider_user_id: format!("{}_user_123", provider_name),
            name: Some(format!("{} User", provider_name.to_uppercase())),
            email: Some(format!("user@{}.example.com", provider_name)),
            avatar: Some(format!("https://{}.example.com/avatar.png", provider_name)),
            raw_data: serde_json::json!({
                "id": "123",
                "provider": provider_name,
            }),
        })
    }
    
    /// Generate a random state parameter for CSRF protection
    pub fn generate_state(&self) -> crate::NoorResult<String> {
        crate::core::security::Encryption::new().random_string(32)
    }
    
    /// Validate the state parameter
    pub fn validate_state(&self, expected: &str, actual: &str) -> bool {
        crate::core::security::Encryption::constant_time_compare(
            expected.as_bytes(),
            actual.as_bytes(),
        )
    }
}

/// Pre-configured OAuth providers
pub mod providers {
    use super::*;
    
    /// Google OAuth2 configuration
    pub fn google(client_id: &str, client_secret: &str, redirect_uri: &str) -> OAuthProvider {
        OAuthProvider {
            name: "google".to_string(),
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            redirect_uri: redirect_uri.to_string(),
            scopes: vec![
                "openid".to_string(),
                "email".to_string(),
                "profile".to_string(),
            ],
            auth_url: "https://accounts.google.com/o/oauth2/auth".to_string(),
            token_url: "https://oauth2.googleapis.com/token".to_string(),
            user_info_url: "https://www.googleapis.com/oauth2/v2/userinfo".to_string(),
            enabled: true,
        }
    }
    
    /// GitHub OAuth2 configuration
    pub fn github(client_id: &str, client_secret: &str, redirect_uri: &str) -> OAuthProvider {
        OAuthProvider {
            name: "github".to_string(),
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            redirect_uri: redirect_uri.to_string(),
            scopes: vec![
                "user:email".to_string(),
                "read:user".to_string(),
            ],
            auth_url: "https://github.com/login/oauth/authorize".to_string(),
            token_url: "https://github.com/login/oauth/access_token".to_string(),
            user_info_url: "https://api.github.com/user".to_string(),
            enabled: true,
        }
    }
    
    /// Facebook OAuth2 configuration
    pub fn facebook(client_id: &str, client_secret: &str, redirect_uri: &str) -> OAuthProvider {
        OAuthProvider {
            name: "facebook".to_string(),
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            redirect_uri: redirect_uri.to_string(),
            scopes: vec![
                "email".to_string(),
                "public_profile".to_string(),
            ],
            auth_url: "https://www.facebook.com/v18.0/dialog/oauth".to_string(),
            token_url: "https://graph.facebook.com/v18.0/oauth/access_token".to_string(),
            user_info_url: "https://graph.facebook.com/v18.0/me".to_string(),
            enabled: true,
        }
    }
    
    /// Twitter/X OAuth2 configuration
    pub fn twitter(client_id: &str, client_secret: &str, redirect_uri: &str) -> OAuthProvider {
        OAuthProvider {
            name: "twitter".to_string(),
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            redirect_uri: redirect_uri.to_string(),
            scopes: vec![
                "tweet.read".to_string(),
                "users.read".to_string(),
            ],
            auth_url: "https://twitter.com/i/oauth2/authorize".to_string(),
            token_url: "https://api.twitter.com/2/oauth2/token".to_string(),
            user_info_url: "https://api.twitter.com/2/users/me".to_string(),
            enabled: true,
        }
    }
    
    /// Microsoft OAuth2 configuration
    pub fn microsoft(client_id: &str, client_secret: &str, redirect_uri: &str) -> OAuthProvider {
        OAuthProvider {
            name: "microsoft".to_string(),
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            redirect_uri: redirect_uri.to_string(),
            scopes: vec![
                "openid".to_string(),
                "email".to_string(),
                "profile".to_string(),
                "User.Read".to_string(),
            ],
            auth_url: "https://login.microsoftonline.com/common/oauth2/v2.0/authorize".to_string(),
            token_url: "https://login.microsoftonline.com/common/oauth2/v2.0/token".to_string(),
            user_info_url: "https://graph.microsoft.com/v1.0/me".to_string(),
            enabled: true,
        }
    }
}

/// Simple URL encoding (placeholder)
mod urlencoding {
    pub fn encode(s: &str) -> String {
        s.chars()
            .map(|c| match c {
                'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '.' | '_' | '~' => c.to_string(),
                _ => format!("%{:02X}", c as u8),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_oauth_provider_registration() {
        let manager = OAuthManager::new();
        
        let provider = providers::google("client_id", "secret", "http://localhost/callback");
        manager.register(provider);
        
        assert!(manager.get("google").is_some());
        assert_eq!(manager.list_enabled().len(), 1);
    }
    
    #[test]
    fn test_authorization_url() {
        let manager = OAuthManager::new();
        
        manager.register(providers::github("id", "secret", "http://localhost/callback"));
        
        let url = manager.authorization_url("github", "random_state").unwrap();
        
        assert!(url.contains("github.com"));
        assert!(url.contains("client_id=id"));
        assert!(url.contains("state=random_state"));
        assert!(url.contains("scope="));
    }
    
    #[test]
    fn test_state_validation() {
        let manager = OAuthManager::new();
        
        let state = manager.generate_state().unwrap();
        assert!(!state.is_empty());
        
        assert!(manager.validate_state(&state, &state));
        assert!(!manager.validate_state(&state, "wrong_state"));
    }
    
    #[test]
    fn test_get_user_info() {
        let manager = OAuthManager::new();
        
        manager.register(providers::google("id", "secret", "http://localhost/callback"));
        
        let user_info = manager.get_user_info("google", "access_token").unwrap();
        
        assert_eq!(user_info.provider, "google");
        assert!(user_info.email.is_some());
        assert!(user_info.name.is_some());
    }
    
    #[test]
    fn test_preconfigured_providers() {
        let google = providers::google("id", "secret", "callback");
        assert_eq!(google.name, "google");
        assert!(google.auth_url.contains("google"));
        
        let github = providers::github("id", "secret", "callback");
        assert_eq!(github.name, "github");
        assert!(github.auth_url.contains("github"));
        
        let facebook = providers::facebook("id", "secret", "callback");
        assert_eq!(facebook.name, "facebook");
    }
}
