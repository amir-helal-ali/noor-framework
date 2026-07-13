// ============================================================
// View/Template Engine - محرك القوالب
// ============================================================
// Uses Handlebars with custom helpers for security and convenience.
// Caches compiled templates for performance.
//
// يستخدم Handlebars مع helpers مخصصة للأمان.
// ============================================================

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;
use handlebars::{Handlebars, Context, RenderContext, HelperResult, Output, Helper};
use serde::Serialize;
use crate::NoorResult;

/// View engine for rendering templates
/// محرك العرض لتصيير القوالب
pub struct ViewEngine {
    handlebars: Arc<RwLock<Handlebars<'static>>>,
    template_dir: PathBuf,
    cache_templates: bool,
    auto_reload: bool,
    /// Cache of compiled template timestamps
    template_times: Arc<RwLock<HashMap<String, std::time::SystemTime>>>,
}

impl ViewEngine {
    /// Create a new view engine
    pub fn new(template_dir: &str, cache_templates: bool, auto_reload: bool) -> crate::NoorResult<Self> {
        let mut handlebars = Handlebars::new();
        
        // Register built-in helpers
        Self::register_helpers(&mut handlebars);
        
        // Register partials if directory exists (requires dir_source feature)
        let partials_dir = Path::new(template_dir).join("partials");
        if partials_dir.exists() {
            // Directory source support is gated behind the `dir_source` feature of handlebars.
            // Skip partials registration if the feature is not enabled.
            let _ = partials_dir;
        }
        
        Ok(Self {
            handlebars: Arc::new(RwLock::new(handlebars)),
            template_dir: PathBuf::from(template_dir),
            cache_templates,
            auto_reload,
            template_times: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// Register built-in helpers
    fn register_helpers(handlebars: &mut Handlebars<'static>) {
        // Escape HTML helper (for security)
        handlebars.register_helper("escape", Box::new(|h: &Helper, _: &Handlebars, _: &Context, _: &mut RenderContext, out: &mut dyn Output| -> HelperResult {
            if let Some(param) = h.param(0) {
                let escaped = crate::core::security::Xss::escape(&param.value().to_string());
                out.write(&escaped)?;
            }
            Ok(())
        }));
        
        // Truncate helper
        handlebars.register_helper("truncate", Box::new(|h: &Helper, _: &Handlebars, _: &Context, _: &mut RenderContext, out: &mut dyn Output| -> HelperResult {
            if let (Some(text), Some(len)) = (h.param(0), h.param(1)) {
                let text = text.value().as_str().unwrap_or("");
                let len = len.value().as_u64().unwrap_or(100) as usize;
                let truncated = if text.len() > len {
                    format!("{}...", &text[..len])
                } else {
                    text.to_string()
                };
                out.write(&truncated)?;
            }
            Ok(())
        }));
        
        // Format date helper
        handlebars.register_helper("date", Box::new(|h: &Helper, _: &Handlebars, _: &Context, _: &mut RenderContext, out: &mut dyn Output| -> HelperResult {
            if let Some(date) = h.param(0) {
                let date_str = date.value().to_string();
                out.write(&date_str)?;
            }
            Ok(())
        }));
        
        // JSON stringify helper
        handlebars.register_helper("json", Box::new(|h: &Helper, _: &Handlebars, _: &Context, _: &mut RenderContext, out: &mut dyn Output| -> HelperResult {
            if let Some(param) = h.param(0) {
                let json = serde_json::to_string(param.value())
                    .map_err(|e| handlebars::RenderErrorReason::SerdeError(e))?;
                out.write(&json)?;
            }
            Ok(())
        }));
    }
    
    /// Render a template with data
    /// تصيير قالب ببيانات
    pub fn render<T: Serialize>(&self, template: &str, data: &T) -> crate::NoorResult<String> {
        let mut handlebars = self.handlebars.write();
        
        // Check if we need to (re)load the template
        let needs_reload = if self.auto_reload {
            self.template_needs_reload(template)
        } else {
            !handlebars.has_template(template)
        };
        
        if needs_reload {
            self.load_template(&mut handlebars, template)?;
        }
        
        handlebars.render(template, data)
            .map_err(|e| crate::NoorError::Template(format!("Template render error: {}", e)))
    }
    
    /// Render a template to an HTML response
    pub fn render_response<T: Serialize>(&self, template: &str, data: &T) -> crate::NoorResult<crate::core::http::Response> {
        let html = self.render(template, data)?;
        Ok(crate::core::http::Response::ok().html(html))
    }
    
    /// Load a template from file
    fn load_template(&self, handlebars: &mut Handlebars, name: &str) -> crate::NoorResult<()> {
        let path = self.template_dir.join(format!("{}.hbs", name));
        
        if !path.exists() {
            return Err(crate::NoorError::Template(format!("Template not found: {}", path.display())));
        }
        
        let content = std::fs::read_to_string(&path)?;
        
        handlebars.register_template_string(name, content)
            .map_err(|e| crate::NoorError::Template(format!("Template compile error: {}", e)))?;
        
        // Update timestamp
        if let Ok(metadata) = std::fs::metadata(&path) {
            if let Ok(modified) = metadata.modified() {
                self.template_times.write().insert(name.to_string(), modified);
            }
        }
        
        Ok(())
    }
    
    /// Check if a template needs to be reloaded
    fn template_needs_reload(&self, name: &str) -> bool {
        let path = self.template_dir.join(format!("{}.hbs", name));
        
        if !path.exists() {
            return false;
        }
        
        if let Ok(metadata) = std::fs::metadata(&path) {
            if let Ok(modified) = metadata.modified() {
                let times = self.template_times.read();
                if let Some(last) = times.get(name) {
                    return modified != *last;
                }
            }
        }
        
        true
    }
    
    /// Check if a template exists
    pub fn exists(&self, name: &str) -> bool {
        let path = self.template_dir.join(format!("{}.hbs", name));
        path.exists()
    }
}

/// A template instance for binding data
pub struct Template {
    name: String,
    data: serde_json::Value,
}

impl Template {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            data: serde_json::json!({}),
        }
    }
    
    pub fn with<T: Serialize>(mut self, key: &str, value: &T) -> Self {
        if let serde_json::Value::Object(ref mut map) = self.data {
            map.insert(key.to_string(), serde_json::to_value(value).unwrap_or(serde_json::Value::Null));
        }
        self
    }
    
    pub fn name(&self) -> &str {
        &self.name
    }
    
    pub fn data(&self) -> &serde_json::Value {
        &self.data
    }
}
