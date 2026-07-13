// ============================================================
// Router - نظام التوجيه
// ============================================================
// A fast, tree-based router with support for:
// - Static and dynamic route parameters
// - Route groups with shared middleware
// - All HTTP methods
// - Named routes for URL generation
//
// راوتر سريع مع دعم المعاملات الديناميكية والمجموعات.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use crate::core::http::{Method, Request, Response, StatusCode};
use crate::core::middleware::MiddlewareStack;
use crate::NoorResult;

/// Handler function type
/// نوع دالة المعالج
pub type Handler = Arc<dyn Fn(Request) -> NoorResult<Response> + Send + Sync>;

/// A route definition
/// تعريف المسار
#[derive(Clone)]
pub struct Route {
    pub method: Method,
    pub path: String,
    pub handler: Handler,
    pub name: Option<String>,
    pub middleware: Vec<String>,
}

impl std::fmt::Debug for Route {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Route")
            .field("method", &self.method)
            .field("path", &self.path)
            .field("name", &self.name)
            .field("middleware", &self.middleware)
            .finish()
    }
}

/// A group of routes with shared attributes
/// مجموعة مسارات بسمات مشتركة
pub struct Group {
    pub prefix: String,
    pub middleware: Vec<String>,
    pub routes: Vec<Route>,
}

/// The main router
/// الراوتر الرئيسي
pub struct Router {
    routes: Vec<Route>,
    groups: Vec<Group>,
    named_routes: HashMap<String, usize>,
    middleware_stack: MiddlewareStack,
    not_found_handler: Option<Handler>,
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

impl Router {
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            groups: Vec::new(),
            named_routes: HashMap::new(),
            middleware_stack: MiddlewareStack::new(),
            not_found_handler: None,
        }
    }
    
    /// Register a global middleware
    /// تسجيل middleware عام
    pub fn use_middleware(&mut self, name: &str) -> &mut Self {
        self.middleware_stack.add(name);
        self
    }
    
    /// Register a GET route
    /// تسجيل مسار GET
    pub fn get(&mut self, path: &str, handler: impl HandlerFn) -> &mut Self {
        self.add_route(Method::Get, path, handler.into_handler(), None, vec![])
    }
    
    /// Register a POST route
    pub fn post(&mut self, path: &str, handler: impl HandlerFn) -> &mut Self {
        self.add_route(Method::Post, path, handler.into_handler(), None, vec![])
    }
    
    /// Register a PUT route
    pub fn put(&mut self, path: &str, handler: impl HandlerFn) -> &mut Self {
        self.add_route(Method::Put, path, handler.into_handler(), None, vec![])
    }
    
    /// Register a PATCH route
    pub fn patch(&mut self, path: &str, handler: impl HandlerFn) -> &mut Self {
        self.add_route(Method::Patch, path, handler.into_handler(), None, vec![])
    }
    
    /// Register a DELETE route
    pub fn delete(&mut self, path: &str, handler: impl HandlerFn) -> &mut Self {
        self.add_route(Method::Delete, path, handler.into_handler(), None, vec![])
    }

    /// Register a HEAD route
    pub fn head(&mut self, path: &str, handler: impl HandlerFn) -> &mut Self {
        self.add_route(Method::Head, path, handler.into_handler(), None, vec![])
    }

    /// Register an OPTIONS route
    pub fn options(&mut self, path: &str, handler: impl HandlerFn) -> &mut Self {
        self.add_route(Method::Options, path, handler.into_handler(), None, vec![])
    }
    
    /// Register a route with a name
    /// تسجيل مسار باسم
    pub fn add_route(
        &mut self,
        method: Method,
        path: &str,
        handler: Handler,
        name: Option<String>,
        middleware: Vec<String>,
    ) -> &mut Self {
        let route = Route {
            method,
            path: path.to_string(),
            handler,
            name: name.clone(),
            middleware,
        };
        
        if let Some(ref n) = name {
            self.named_routes.insert(n.clone(), self.routes.len());
        }
        
        self.routes.push(route);
        self
    }
    
    /// Create a route group
    /// إنشاء مجموعة مسارات
    pub fn group<F>(&mut self, prefix: &str, middleware: Vec<String>, f: F) -> &mut Self
    where
        F: FnOnce(&mut Group),
    {
        let mut group = Group {
            prefix: prefix.to_string(),
            middleware,
            routes: Vec::new(),
        };
        f(&mut group);
        
        // Register group routes with prefix
        for route in &group.routes {
            let full_path = format!("{}{}", group.prefix, route.path);
            let mut all_middleware = group.middleware.clone();
            all_middleware.extend(route.middleware.clone());
            
            self.add_route(
                route.method,
                &full_path,
                route.handler.clone(),
                route.name.clone(),
                all_middleware,
            );
        }
        
        self
    }
    
    /// Set a custom 404 handler
    /// تعيين معالج 404 مخصص
    pub fn not_found(&mut self, handler: impl HandlerFn) -> &mut Self {
        self.not_found_handler = Some(handler.into_handler());
        self
    }
    
    /// Match a request to a route
    /// مطابقة الطلب مع مسار
    pub fn match_route(&self, method: &Method, path: &str) -> Option<(&Route, HashMap<String, String>)> {
        for route in &self.routes {
            if &route.method != method {
                continue;
            }
            
            if let Some(params) = Self::match_path(&route.path, path) {
                return Some((route, params));
            }
        }
        None
    }
    
    /// Match a path pattern against an actual path
    /// مطابقة نمط المسار مع المسار الفعلي
    fn match_path(pattern: &str, path: &str) -> Option<HashMap<String, String>> {
        let pattern_parts: Vec<&str> = pattern.trim_matches('/').split('/').filter(|s| !s.is_empty()).collect();
        let path_parts: Vec<&str> = path.trim_matches('/').split('/').filter(|s| !s.is_empty()).collect();
        
        let mut params = HashMap::new();
        let mut i = 0;
        let mut j = 0;
        
        while i < pattern_parts.len() && j < path_parts.len() {
            let pp = pattern_parts[i];
            
            if pp == "*" {
                // Wildcard - matches everything remaining
                return Some(params);
            } else if pp.starts_with('{') && pp.ends_with('}') {
                // Dynamic parameter {param} or {param?}
                let param_name = pp
                    .trim_start_matches('{')
                    .trim_end_matches('}')
                    .trim_end_matches('?');
                params.insert(param_name.to_string(), path_parts[j].to_string());
            } else if pp != path_parts[j] {
                return None;
            }
            
            i += 1;
            j += 1;
        }
        
        if i == pattern_parts.len() && j == path_parts.len() {
            Some(params)
        } else if i < pattern_parts.len() {
            // Check if remaining pattern parts are optional
            for k in i..pattern_parts.len() {
                let pp = pattern_parts[k];
                if !(pp.starts_with('{') && pp.ends_with("?}")) && pp != "*}" {
                    return None;
                }
            }
            Some(params)
        } else {
            None
        }
    }
    
    /// Generate a URL for a named route
    /// توليد URL لمسار مسمى
    pub fn url_for(&self, name: &str, params: &HashMap<String, String>) -> Option<String> {
        let idx = self.named_routes.get(name)?;
        let route = &self.routes[*idx];
        
        let mut url = route.path.clone();
        for (k, v) in params {
            let placeholder = format!("{{{}}}", k);
            let optional_placeholder = format!("{{{}}}", k);
            url = url.replace(&placeholder, v);
            url = url.replace(&optional_placeholder, v);
        }
        
        Some(url)
    }
    
    /// Dispatch a request to the matched route
    /// توجيه الطلب إلى المسار المطابق
    ///
    /// Executes the global middleware stack first, then any route-specific
    /// middleware, and finally the route handler. If any middleware
    /// short-circuits with `MiddlewareOutcome::Stop(response)`, that
    /// response is returned immediately without calling the handler.
    pub fn dispatch(&self, mut request: Request) -> NoorResult<Response> {
        match self.match_route(&request.method, &request.path) {
            Some((route, params)) => {
                request.route_params = params;

                // --- Phase 1: global middleware stack ---
                match self.middleware_stack.execute(request.clone())? {
                    crate::core::middleware::MiddlewareOutcome::Stop(response) => {
                        return Ok(response);
                    }
                    crate::core::middleware::MiddlewareOutcome::Continue(req) => {
                        request = req;
                    }
                }

                // --- Phase 2: route-specific middleware ---
                // Each route can name additional middleware; run them in order.
                for name in &route.middleware {
                    if let Some(middleware) = self.middleware_stack.registry().get(name) {
                        match middleware(request.clone())? {
                            crate::core::middleware::MiddlewareOutcome::Stop(response) => {
                                return Ok(response);
                            }
                            crate::core::middleware::MiddlewareOutcome::Continue(req) => {
                                request = req;
                            }
                        }
                    }
                }

                // --- Phase 3: handler ---
                (route.handler)(request)
            }
            None => {
                // Run global middleware even for 404s (so logging, CORS, etc.
                // still apply).
                let request = match self.middleware_stack.execute(request)? {
                    crate::core::middleware::MiddlewareOutcome::Stop(response) => {
                        return Ok(response);
                    }
                    crate::core::middleware::MiddlewareOutcome::Continue(req) => req,
                };

                if let Some(handler) = &self.not_found_handler {
                    handler(request)
                } else {
                    Ok(Response::new(StatusCode::NOT_FOUND)
                        .html("<h1>404 - Page Not Found</h1><p>The page you requested could not be found.</p>"))
                }
            }
        }
    }
    
    /// Get all registered routes
    /// الحصول على جميع المسارات المسجلة
    pub fn routes(&self) -> &[Route] {
        &self.routes
    }

    /// Borrow the middleware stack (so external code can register named
    /// middleware on it, e.g. via `auth::register`).
    pub fn middleware_stack(&mut self) -> &mut MiddlewareStack {
        &mut self.middleware_stack
    }
}

/// Trait for converting closures into handlers
/// Trait لتحويل الـ closures إلى handlers
pub trait HandlerFn: Send + Sync + 'static {
    fn into_handler(self) -> Handler;
}

impl<F> HandlerFn for F
where
    F: Fn(Request) -> NoorResult<Response> + Send + Sync + 'static,
{
    fn into_handler(self) -> Handler {
        Arc::new(self)
    }
}

impl Group {
    pub fn get(&mut self, path: &str, handler: impl HandlerFn) -> &mut Self {
        self.routes.push(Route {
            method: Method::Get,
            path: path.to_string(),
            handler: handler.into_handler(),
            name: None,
            middleware: vec![],
        });
        self
    }
    
    pub fn post(&mut self, path: &str, handler: impl HandlerFn) -> &mut Self {
        self.routes.push(Route {
            method: Method::Post,
            path: path.to_string(),
            handler: handler.into_handler(),
            name: None,
            middleware: vec![],
        });
        self
    }
    
    pub fn put(&mut self, path: &str, handler: impl HandlerFn) -> &mut Self {
        self.routes.push(Route {
            method: Method::Put,
            path: path.to_string(),
            handler: handler.into_handler(),
            name: None,
            middleware: vec![],
        });
        self
    }
    
    pub fn delete(&mut self, path: &str, handler: impl HandlerFn) -> &mut Self {
        self.routes.push(Route {
            method: Method::Delete,
            path: path.to_string(),
            handler: handler.into_handler(),
            name: None,
            middleware: vec![],
        });
        self
    }
}
