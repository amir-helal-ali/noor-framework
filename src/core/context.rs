// ============================================================
// Request Context - سياق الطلب
// ============================================================
// Provides shared state and services to handlers
// يوفر حالة مشتركة وخدمات للمعالجات
// ============================================================

use std::sync::Arc;
use crate::core::config::Config;
use crate::core::cache::CacheManager;
use crate::core::view::ViewEngine;
use crate::core::security::Csrf;

/// Request context - shared state available to all handlers
/// سياق الطلب - حالة مشتركة لجميع المعالجات
#[derive(Clone)]
pub struct Context {
    pub config: Arc<Config>,
    pub cache: Arc<CacheManager>,
    pub view: Option<Arc<ViewEngine>>,
    pub csrf: Arc<Csrf>,
}

impl Context {
    pub fn new(
        config: Arc<Config>,
        cache: Arc<CacheManager>,
        view: Option<Arc<ViewEngine>>,
        csrf: Arc<Csrf>,
    ) -> Self {
        Self { config, cache, view, csrf }
    }
}
