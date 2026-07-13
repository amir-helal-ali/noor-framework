// ============================================================
// Built-in Middleware - الوسائط المدمجة
// ============================================================
// Common middleware implementations for production use.
// تطبيقات middleware شائعة للاستخدام في الإنتاج.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use crate::core::http::{Request, Response};
use crate::NoorResult;

/// A middleware that can process a request before/after the handler
/// وسيط يعالج الطلب قبل/بعد المعالج
pub type MiddlewareFn = Arc<dyn Fn(Request) -> NoorResult<MiddlewareOutcome> + Send + Sync>;

/// The outcome of a middleware
/// نتيجة الـ middleware
pub enum MiddlewareOutcome {
    /// Continue to the next middleware or handler
    /// الاستمرار للوسيط التالي أو المعالج
    Continue(Request),
    /// Short-circuit with a response
    /// إيقاف باستجابة
    Stop(Response),
}

/// A middleware trait for object-oriented usage
/// Trait للـ middleware للاستخدام OOP
pub trait Middleware: Send + Sync {
    /// Handle the incoming request
    /// معالجة الطلب الوارد
    fn handle(&self, request: Request) -> NoorResult<MiddlewareOutcome>;
    
    /// Get the middleware name
    /// الحصول على اسم الـ middleware
    fn name(&self) -> &str;
}

/// A stack of middleware to execute
/// مجموعة middlewares للتنفيذ
pub struct MiddlewareStack {
    middlewares: Vec<String>,
    registry: HashMap<String, MiddlewareFn>,
}

impl Default for MiddlewareStack {
    fn default() -> Self {
        Self::new()
    }
}

impl MiddlewareStack {
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
            registry: HashMap::new(),
        }
    }
    
    /// Add a middleware to the stack
    /// إضافة middleware للمجموعة
    pub fn add(&mut self, name: &str) -> &mut Self {
        self.middlewares.push(name.to_string());
        self
    }
    
    /// Register a middleware in the registry
    /// تسجيل middleware في السجل
    pub fn register(&mut self, name: &str, middleware: MiddlewareFn) -> &mut Self {
        self.registry.insert(name.to_string(), middleware);
        self
    }
    
    /// Execute the middleware stack
    /// تنفيذ مجموعة الـ middlewares
    pub fn execute(&self, request: Request) -> NoorResult<MiddlewareOutcome> {
        let mut current_request = request;
        
        for name in &self.middlewares {
            if let Some(middleware) = self.registry.get(name) {
                match middleware(current_request)? {
                    MiddlewareOutcome::Continue(req) => {
                        current_request = req;
                    }
                    MiddlewareOutcome::Stop(response) => {
                        return Ok(MiddlewareOutcome::Stop(response));
                    }
                }
            }
        }
        
        Ok(MiddlewareOutcome::Continue(current_request))
    }
    
    /// Get the list of middleware names
    /// الحصول على قائمة أسماء الـ middlewares
    pub fn middlewares(&self) -> &[String] {
        &self.middlewares
    }

    /// Borrow the middleware registry (so the router can look up
    /// route-specific middleware by name).
    pub fn registry(&self) -> &HashMap<String, MiddlewareFn> {
        &self.registry
    }
}

pub mod cors;
pub mod throttle;
pub mod compression;
pub mod auth;
pub mod logging;
pub mod helmet;

pub use cors::CorsMiddleware;
pub use throttle::ThrottleMiddleware;
pub use compression::CompressionMiddleware;
pub use auth::AuthMiddleware;
pub use logging::LoggingMiddleware;
pub use helmet::HelmetMiddleware;
