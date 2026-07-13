// ============================================================
// Content Security Policy (CSP) - سياسة أمان المحتوى
// ============================================================
// Configure CSP headers to prevent XSS, clickjacking,
// and other code injection attacks.
//
// تكوين headers CSP لمنع هجمات XSS والحقن.
// ============================================================

use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// CSP directive
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CspDirective {
    DefaultSrc,
    ScriptSrc,
    StyleSrc,
    ImgSrc,
    FontSrc,
    ConnectSrc,
    MediaSrc,
    ObjectSrc,
    FrameSrc,
    ChildSrc,
    WorkerSrc,
    FrameAncestors,
    FormAction,
    BaseUri,
    PluginTypes,
    ManifestSrc,
    NavigateTo,
    PrefetchSrc,
}

impl CspDirective {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DefaultSrc => "default-src",
            Self::ScriptSrc => "script-src",
            Self::StyleSrc => "style-src",
            Self::ImgSrc => "img-src",
            Self::FontSrc => "font-src",
            Self::ConnectSrc => "connect-src",
            Self::MediaSrc => "media-src",
            Self::ObjectSrc => "object-src",
            Self::FrameSrc => "frame-src",
            Self::ChildSrc => "child-src",
            Self::WorkerSrc => "worker-src",
            Self::FrameAncestors => "frame-ancestors",
            Self::FormAction => "form-action",
            Self::BaseUri => "base-uri",
            Self::PluginTypes => "plugin-types",
            Self::ManifestSrc => "manifest-src",
            Self::NavigateTo => "navigate-to",
            Self::PrefetchSrc => "prefetch-src",
        }
    }
}

/// CSP source
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CspSource {
    /// 'self' - same origin
    Self_,
    /// 'unsafe-inline' - allows inline resources
    UnsafeInline,
    /// 'unsafe-eval' - allows eval()
    UnsafeEval,
    /// 'none' - no sources allowed
    None,
    /// 'strict-dynamic' - allows scripts from dynamically added
    StrictDynamic,
    /// Specific URL
    Url(String),
    /// Wildcard domain (e.g., *.example.com)
    Wildcard(String),
    /// Nonce (e.g., 'nonce-abc123')
    Nonce(String),
    /// Hash (e.g., 'sha256-abc123...')
    Hash(String, String),
    /// Data URI
    Data,
    /// Blob URI
    Blob,
    /// Filesystem URI
    Filesystem,
    /// mediastream URI
    Mediastream,
}

impl CspSource {
    pub fn as_str(&self) -> String {
        match self {
            Self::Self_ => "'self'".to_string(),
            Self::UnsafeInline => "'unsafe-inline'".to_string(),
            Self::UnsafeEval => "'unsafe-eval'".to_string(),
            Self::None => "'none'".to_string(),
            Self::StrictDynamic => "'strict-dynamic'".to_string(),
            Self::Url(url) => url.clone(),
            Self::Wildcard(domain) => format!("*.{}", domain),
            Self::Nonce(nonce) => format!("'nonce-{}'", nonce),
            Self::Hash(algo, hash) => format!("'{}-{}'", algo, hash),
            Self::Data => "data:".to_string(),
            Self::Blob => "blob:".to_string(),
            Self::Filesystem => "filesystem:".to_string(),
            Self::Mediastream => "mediastream:".to_string(),
        }
    }
}

/// Content Security Policy builder
pub struct ContentSecurityPolicy {
    directives: HashMap<CspDirective, Vec<CspSource>>,
    /// Whether to use Report-Only mode
    report_only: bool,
    /// URL to report violations to
    report_uri: Option<String>,
}

impl Default for ContentSecurityPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentSecurityPolicy {
    pub fn new() -> Self {
        Self {
            directives: HashMap::new(),
            report_only: false,
            report_uri: None,
        }
    }
    
    /// Set default-src
    pub fn default_src(mut self, sources: Vec<CspSource>) -> Self {
        self.directives.insert(CspDirective::DefaultSrc, sources);
        self
    }
    
    /// Set script-src
    pub fn script_src(mut self, sources: Vec<CspSource>) -> Self {
        self.directives.insert(CspDirective::ScriptSrc, sources);
        self
    }
    
    /// Set style-src
    pub fn style_src(mut self, sources: Vec<CspSource>) -> Self {
        self.directives.insert(CspDirective::StyleSrc, sources);
        self
    }
    
    /// Set img-src
    pub fn img_src(mut self, sources: Vec<CspSource>) -> Self {
        self.directives.insert(CspDirective::ImgSrc, sources);
        self
    }
    
    /// Set font-src
    pub fn font_src(mut self, sources: Vec<CspSource>) -> Self {
        self.directives.insert(CspDirective::FontSrc, sources);
        self
    }
    
    /// Set connect-src
    pub fn connect_src(mut self, sources: Vec<CspSource>) -> Self {
        self.directives.insert(CspDirective::ConnectSrc, sources);
        self
    }
    
    /// Set frame-ancestors (prevents clickjacking)
    pub fn frame_ancestors(mut self, sources: Vec<CspSource>) -> Self {
        self.directives.insert(CspDirective::FrameAncestors, sources);
        self
    }
    
    /// Set form-action
    pub fn form_action(mut self, sources: Vec<CspSource>) -> Self {
        self.directives.insert(CspDirective::FormAction, sources);
        self
    }
    
    /// Set base-uri
    pub fn base_uri(mut self, sources: Vec<CspSource>) -> Self {
        self.directives.insert(CspDirective::BaseUri, sources);
        self
    }
    
    /// Enable Report-Only mode (don't enforce, just report)
    pub fn report_only(mut self) -> Self {
        self.report_only = true;
        self
    }
    
    /// Set report URI
    pub fn report_uri(mut self, uri: &str) -> Self {
        self.report_uri = Some(uri.to_string());
        self
    }
    
    /// Build the CSP header value
    pub fn build(&self) -> String {
        let mut parts = Vec::new();
        
        // Default order for directives
        let order = [
            CspDirective::DefaultSrc,
            CspDirective::ScriptSrc,
            CspDirective::StyleSrc,
            CspDirective::ImgSrc,
            CspDirective::FontSrc,
            CspDirective::ConnectSrc,
            CspDirective::MediaSrc,
            CspDirective::ObjectSrc,
            CspDirective::FrameSrc,
            CspDirective::ChildSrc,
            CspDirective::WorkerSrc,
            CspDirective::FrameAncestors,
            CspDirective::FormAction,
            CspDirective::BaseUri,
            CspDirective::ManifestSrc,
        ];
        
        for directive in &order {
            if let Some(sources) = self.directives.get(directive) {
                let sources_str: Vec<String> = sources.iter().map(|s| s.as_str()).collect();
                parts.push(format!("{} {}", directive.as_str(), sources_str.join(" ")));
            }
        }
        
        // Add report-uri
        if let Some(ref uri) = self.report_uri {
            parts.push(format!("report-uri {}", uri));
        }
        
        parts.join("; ")
    }
    
    /// Get the header name (Content-Security-Policy or Content-Security-Policy-Report-Only)
    pub fn header_name(&self) -> &'static str {
        if self.report_only {
            "Content-Security-Policy-Report-Only"
        } else {
            "Content-Security-Policy"
        }
    }
    
    /// Build as a (header_name, header_value) tuple
    pub fn to_header(&self) -> (String, String) {
        (self.header_name().to_string(), self.build())
    }
}

/// Pre-configured CSP policies
pub mod presets {
    use super::*;
    
    /// Strict CSP policy (recommended for new apps)
    pub fn strict() -> ContentSecurityPolicy {
        ContentSecurityPolicy::new()
            .default_src(vec![CspSource::None])
            .script_src(vec![CspSource::Self_])
            .style_src(vec![CspSource::Self_])
            .img_src(vec![CspSource::Self_, CspSource::Data])
            .font_src(vec![CspSource::Self_])
            .connect_src(vec![CspSource::Self_])
            .frame_ancestors(vec![CspSource::None])
            .form_action(vec![CspSource::Self_])
            .base_uri(vec![CspSource::Self_])
    }
    
    /// Moderate CSP policy (allows some inline)
    pub fn moderate() -> ContentSecurityPolicy {
        ContentSecurityPolicy::new()
            .default_src(vec![CspSource::Self_])
            .script_src(vec![CspSource::Self_, CspSource::UnsafeInline])
            .style_src(vec![CspSource::Self_, CspSource::UnsafeInline])
            .img_src(vec![CspSource::Self_, CspSource::Data, CspSource::Wildcard("".to_string())])
            .font_src(vec![CspSource::Self_, CspSource::Data])
            .connect_src(vec![CspSource::Self_])
            .frame_ancestors(vec![CspSource::Self_])
    }
    
    /// Permissive CSP (development only)
    pub fn permissive() -> ContentSecurityPolicy {
        ContentSecurityPolicy::new()
            .default_src(vec![CspSource::None])
            .script_src(vec![CspSource::Self_, CspSource::UnsafeInline, CspSource::UnsafeEval])
            .style_src(vec![CspSource::Self_, CspSource::UnsafeInline])
            .img_src(vec![CspSource::Self_, CspSource::Data, CspSource::Wildcard("".to_string())])
            .font_src(vec![CspSource::Self_, CspSource::Data])
            .connect_src(vec![CspSource::Self_, CspSource::Wildcard("".to_string())])
    }
    
    /// API-only CSP (very strict, no resources)
    pub fn api() -> ContentSecurityPolicy {
        ContentSecurityPolicy::new()
            .default_src(vec![CspSource::None])
            .frame_ancestors(vec![CspSource::None])
            .base_uri(vec![CspSource::None])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_csp_basic() {
        let csp = ContentSecurityPolicy::new()
            .default_src(vec![CspSource::Self_])
            .script_src(vec![CspSource::Self_, CspSource::UnsafeInline]);
        
        let header = csp.build();
        
        assert!(header.contains("default-src 'self'"));
        assert!(header.contains("script-src 'self' 'unsafe-inline'"));
    }
    
    #[test]
    fn test_csp_sources() {
        assert_eq!(CspSource::Self_.as_str(), "'self'");
        assert_eq!(CspSource::None.as_str(), "'none'");
        assert_eq!(CspSource::UnsafeInline.as_str(), "'unsafe-inline'");
        assert_eq!(CspSource::Url("https://example.com".to_string()).as_str(), "https://example.com");
        assert_eq!(CspSource::Wildcard("example.com".to_string()).as_str(), "*.example.com");
        assert_eq!(CspSource::Nonce("abc123".to_string()).as_str(), "'nonce-abc123'");
        assert_eq!(CspSource::Hash("sha256".to_string(), "abc123".to_string()).as_str(), "'sha256-abc123'");
    }
    
    #[test]
    fn test_csp_report_only() {
        let csp = ContentSecurityPolicy::new()
            .default_src(vec![CspSource::Self_])
            .report_only()
            .report_uri("/csp-report");
        
        assert_eq!(csp.header_name(), "Content-Security-Policy-Report-Only");
        assert!(csp.build().contains("report-uri /csp-report"));
    }
    
    #[test]
    fn test_csp_presets() {
        let strict = presets::strict();
        let header = strict.build();
        
        assert!(header.contains("default-src 'none'"));
        assert!(header.contains("script-src 'self'"));
        assert!(header.contains("frame-ancestors 'none'"));
    }
    
    #[test]
    fn test_csp_moderate_preset() {
        let csp = presets::moderate();
        let header = csp.build();
        
        assert!(header.contains("default-src 'self'"));
        assert!(header.contains("'unsafe-inline'"));
    }
    
    #[test]
    fn test_csp_api_preset() {
        let csp = presets::api();
        let (name, value) = csp.to_header();
        
        assert_eq!(name, "Content-Security-Policy");
        assert!(value.contains("default-src 'none'"));
        assert!(value.contains("frame-ancestors 'none'"));
    }
}
