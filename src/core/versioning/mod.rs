// ============================================================
// إصدارات الـ API | API Versioning
// ============================================================

use std::collections::HashMap;
use parking_lot::RwLock;
use std::sync::Arc;

/// استراتيجية الإصدار
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VersioningStrategy {
    /// إصدار في URL: /v1/users
    Uri,
    /// إصدار في header: Accept: application/vnd.noor.v1+json
    Header,
    /// إصدار في query param: /users?version=1
    QueryParam,
}

/// مدير الإصدارات
pub struct VersionManager {
    strategy: VersioningStrategy,
    versions: Arc<RwLock<Vec<String>>>,
    default_version: String,
    header_name: String,
    query_param: String,
}

impl Default for VersionManager {
    fn default() -> Self {
        Self {
            strategy: VersioningStrategy::Uri,
            versions: Arc::new(RwLock::new(vec!["v1".to_string()])),
            default_version: "v1".to_string(),
            header_name: "Accept".to_string(),
            query_param: "version".to_string(),
        }
    }
}

impl VersionManager {
    pub fn new(strategy: VersioningStrategy, default_version: &str) -> Self {
        Self {
            strategy,
            versions: Arc::new(RwLock::new(vec![default_version.to_string()])),
            default_version: default_version.to_string(),
            header_name: "Accept".to_string(),
            query_param: "version".to_string(),
        }
    }
    
    /// تسجيل إصدار جديد
    pub fn register_version(&self, version: &str) {
        self.versions.write().push(version.to_string());
    }
    
    /// الحصول على الإصدار من الطلب
    pub fn extract_version(&self, request: &crate::core::http::Request) -> String {
        match self.strategy {
            VersioningStrategy::Uri => {
                // استخراج من المسار: /v1/users -> v1
                let parts: Vec<&str> = request.path.split('/').filter(|s| !s.is_empty()).collect();
                if let Some(first) = parts.first() {
                    if first.starts_with('v') && first[1..].parse::<u32>().is_ok() {
                        return first.to_string();
                    }
                }
                self.default_version.clone()
            }
            VersioningStrategy::Header => {
                // استخراج من header
                request.header(&self.header_name)
                    .and_then(|h| {
                        // البحث عن vnd.noor.v1+json
                        let parts: Vec<&str> = h.split('.').collect();
                        if parts.len() >= 3 {
                            let version_part = parts[2];
                            if version_part.starts_with('v') {
                                return Some(version_part.to_string());
                            }
                        }
                        None
                    })
                    .unwrap_or_else(|| self.default_version.clone())
            }
            VersioningStrategy::QueryParam => {
                request.query(&self.query_param)
                    .map(|s| format!("v{}", s))
                    .unwrap_or_else(|| self.default_version.clone())
            }
        }
    }
    
    /// التحقق من وجود الإصدار
    pub fn is_supported(&self, version: &str) -> bool {
        self.versions.read().contains(&version.to_string())
    }
    
    /// الحصول على جميع الإصدارات المسجلة
    pub fn versions(&self) -> Vec<String> {
        self.versions.read().clone()
    }
    
    /// الحصول على الإصدار الافتراضي
    pub fn default_version(&self) -> &str {
        &self.default_version
    }
    
    /// بناء مسار مع بادئة الإصدار
    pub fn versioned_path(&self, version: &str, path: &str) -> String {
        match self.strategy {
            VersioningStrategy::Uri => {
                format!("/{}{}", version, path)
            }
            _ => path.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::http::{Request, Method};
    
    #[test]
    fn test_uri_versioning() {
        let manager = VersionManager::new(VersioningStrategy::Uri, "v1");
        
        let request = Request::new(Method::Get, "/v1/users".to_string());
        assert_eq!(manager.extract_version(&request), "v1");
        
        let request = Request::new(Method::Get, "/v2/users".to_string());
        assert_eq!(manager.extract_version(&request), "v2");
        
        let request = Request::new(Method::Get, "/users".to_string());
        assert_eq!(manager.extract_version(&request), "v1");
    }
    
    #[test]
    fn test_query_param_versioning() {
        let manager = VersionManager::new(VersioningStrategy::QueryParam, "v1");
        
        let mut request = Request::new(Method::Get, "/users?version=2".to_string());
        request.query_params.insert("version".to_string(), "2".to_string());
        assert_eq!(manager.extract_version(&request), "v2");
    }
    
    #[test]
    fn test_versioned_path() {
        let manager = VersionManager::new(VersioningStrategy::Uri, "v1");
        
        assert_eq!(manager.versioned_path("v1", "/users"), "/v1/users");
        assert_eq!(manager.versioned_path("v2", "/posts"), "/v2/posts");
    }
}
