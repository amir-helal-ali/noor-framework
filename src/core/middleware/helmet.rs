// ============================================================
// Helmet Middleware - middleware headers الأمان
// ============================================================
// Adds various HTTP headers for security (like Node.js helmet).
// يضيف headers HTTP للأمان (مثل helmet في Node.js).
// ============================================================

use crate::core::http::Response;

/// Helmet middleware for security headers
pub struct HelmetMiddleware {
    config: HelmetConfig,
}

#[derive(Debug, Clone)]
pub struct HelmetConfig {
    pub content_security_policy: Option<String>,
    pub x_content_type_options: bool,
    pub x_frame_options: Option<String>,
    pub x_xss_protection: bool,
    pub strict_transport_security: Option<String>,
    pub referrer_policy: Option<String>,
    pub permissions_policy: Option<String>,
    pub cross_origin_opener_policy: Option<String>,
    pub cross_origin_embedder_policy: Option<String>,
    pub cross_origin_resource_policy: Option<String>,
    pub origin_agent_cluster: bool,
    pub dns_prefetch_control: bool,
}

impl Default for HelmetConfig {
    fn default() -> Self {
        Self {
            content_security_policy: Some(
                "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self' data:; connect-src 'self'; object-src 'none'; base-uri 'self'; form-action 'self'; frame-ancestors 'none';"
                    .to_string()
            ),
            x_content_type_options: true,
            x_frame_options: Some("DENY".to_string()),
            x_xss_protection: true,
            strict_transport_security: Some(
                "max-age=31536000; includeSubDomains; preload".to_string()
            ),
            referrer_policy: Some("strict-origin-when-cross-origin".to_string()),
            permissions_policy: Some(
                "geolocation=(), microphone=(), camera=(), payment=(), usb=()".to_string()
            ),
            cross_origin_opener_policy: Some("same-origin".to_string()),
            cross_origin_embedder_policy: None,
            cross_origin_resource_policy: Some("same-origin".to_string()),
            origin_agent_cluster: true,
            dns_prefetch_control: true,
        }
    }
}

impl HelmetMiddleware {
    pub fn new() -> Self {
        Self { config: HelmetConfig::default() }
    }
    
    pub fn with_config(config: HelmetConfig) -> Self {
        Self { config }
    }
    
    /// Apply security headers to a response
    pub fn apply(&self, response: &mut Response) {
        if let Some(ref csp) = self.config.content_security_policy {
            response.headers.insert(
                "content-security-policy".to_string(),
                csp.clone(),
            );
        }
        
        if self.config.x_content_type_options {
            response.headers.insert(
                "x-content-type-options".to_string(),
                "nosniff".to_string(),
            );
        }
        
        if let Some(ref xfo) = self.config.x_frame_options {
            response.headers.insert(
                "x-frame-options".to_string(),
                xfo.clone(),
            );
        }
        
        if self.config.x_xss_protection {
            response.headers.insert(
                "x-xss-protection".to_string(),
                "1; mode=block".to_string(),
            );
        }
        
        if let Some(ref sts) = self.config.strict_transport_security {
            response.headers.insert(
                "strict-transport-security".to_string(),
                sts.clone(),
            );
        }
        
        if let Some(ref rp) = self.config.referrer_policy {
            response.headers.insert(
                "referrer-policy".to_string(),
                rp.clone(),
            );
        }
        
        if let Some(ref pp) = self.config.permissions_policy {
            response.headers.insert(
                "permissions-policy".to_string(),
                pp.clone(),
            );
        }
        
        if let Some(ref coop) = self.config.cross_origin_opener_policy {
            response.headers.insert(
                "cross-origin-opener-policy".to_string(),
                coop.clone(),
            );
        }
        
        if let Some(ref coep) = self.config.cross_origin_embedder_policy {
            response.headers.insert(
                "cross-origin-embedder-policy".to_string(),
                coep.clone(),
            );
        }
        
        if let Some(ref corp) = self.config.cross_origin_resource_policy {
            response.headers.insert(
                "cross-origin-resource-policy".to_string(),
                corp.clone(),
            );
        }
        
        if self.config.origin_agent_cluster {
            response.headers.insert(
                "origin-agent-cluster".to_string(),
                "?1".to_string(),
            );
        }
        
        if self.config.dns_prefetch_control {
            response.headers.insert(
                "x-dns-prefetch-control".to_string(),
                "off".to_string(),
            );
        }
    }
}

impl Default for HelmetMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_helmet_default_headers() {
        let helmet = HelmetMiddleware::new();
        let mut response = Response::ok();
        
        helmet.apply(&mut response);
        
        assert!(response.headers.contains_key("content-security-policy"));
        assert!(response.headers.contains_key("x-content-type-options"));
        assert!(response.headers.contains_key("x-frame-options"));
        assert!(response.headers.contains_key("strict-transport-security"));
    }
}
