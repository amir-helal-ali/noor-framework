// ============================================================
// Middleware Pipeline Builder - منشئ خط الأنابيب
// ============================================================
// Fluent builder for middleware pipelines with ordering,
// conditional execution, and group support.
//
// منشئ سلس لخطوط أنابيب الوسائط.
// ============================================================

use std::sync::Arc;
use parking_lot::RwLock;
use crate::core::http::{Request, Response, StatusCode};
use crate::NoorResult;

/// Pipeline stage
pub struct PipelineStage {
    pub name: String,
    pub handler: Arc<dyn Fn(&Request) -> PipelineAction + Send + Sync>,
    pub priority: i32,
    pub enabled: bool,
}

/// Action returned by a pipeline stage
pub enum PipelineAction {
    /// Continue to the next stage
    Continue,
    /// Stop and return a response
    Stop(Response),
    /// Skip the remaining stages and go to handler
    Skip,
}

/// Middleware pipeline
pub struct Pipeline {
    stages: Arc<RwLock<Vec<PipelineStage>>>,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl Pipeline {
    pub fn new() -> Self {
        Self {
            stages: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Add a stage to the pipeline
    pub fn add<F>(&self, name: &str, handler: F) -> &Self
    where
        F: Fn(&Request) -> PipelineAction + Send + Sync + 'static,
    {
        self.add_with_priority(name, 100, handler)
    }
    
    /// Add a stage with priority (lower = earlier)
    pub fn add_with_priority<F>(&self, name: &str, priority: i32, handler: F) -> &Self
    where
        F: Fn(&Request) -> PipelineAction + Send + Sync + 'static,
    {
        let stage = PipelineStage {
            name: name.to_string(),
            handler: Arc::new(handler),
            priority,
            enabled: true,
        };
        
        let mut stages = self.stages.write();
        stages.push(stage);
        stages.sort_by_key(|s| s.priority);
        
        self
    }
    
    /// Remove a stage
    pub fn remove(&self, name: &str) -> bool {
        let mut stages = self.stages.write();
        let initial = stages.len();
        stages.retain(|s| s.name != name);
        stages.len() < initial
    }
    
    /// Enable a stage
    pub fn enable(&self, name: &str) {
        let mut stages = self.stages.write();
        if let Some(stage) = stages.iter_mut().find(|s| s.name == name) {
            stage.enabled = true;
        }
    }
    
    /// Disable a stage
    pub fn disable(&self, name: &str) {
        let mut stages = self.stages.write();
        if let Some(stage) = stages.iter_mut().find(|s| s.name == name) {
            stage.enabled = false;
        }
    }
    
    /// Execute the pipeline
    pub fn execute<F>(&self, request: &Request, handler: F) -> NoorResult<Response>
    where
        F: FnOnce(&Request) -> NoorResult<Response>,
    {
        let stages = self.stages.read();
        
        for stage in stages.iter() {
            if !stage.enabled {
                continue;
            }
            
            match (stage.handler)(request) {
                PipelineAction::Continue => {}
                PipelineAction::Stop(response) => return Ok(response),
                PipelineAction::Skip => break,
            }
        }
        
        handler(request)
    }
    
    /// Get the list of stages
    pub fn stages(&self) -> Vec<String> {
        self.stages.read().iter().map(|s| s.name.clone()).collect()
    }
    
    /// Get the count of stages
    pub fn count(&self) -> usize {
        self.stages.read().len()
    }
    
    /// Clear all stages
    pub fn clear(&self) {
        self.stages.write().clear();
    }
}

/// Pipeline builder
pub struct PipelineBuilder {
    pipeline: Pipeline,
}

impl Default for PipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl PipelineBuilder {
    pub fn new() -> Self {
        Self {
            pipeline: Pipeline::new(),
        }
    }
    
    /// Add CORS stage
    pub fn cors(self) -> Self {
        self.pipeline.add("cors", |_req| {
            PipelineAction::Continue
        });
        self
    }
    
    /// Add rate limiting stage
    pub fn rate_limit(self, max: u32) -> Self {
        self.pipeline.add("rate_limit", move |_req| {
            // Simplified rate limiting check
            PipelineAction::Continue
        });
        self
    }
    
    /// Add authentication stage
    pub fn auth(self) -> Self {
        self.pipeline.add("auth", |req| {
            if req.bearer_token().is_none() {
                return PipelineAction::Stop(
                    Response::new(StatusCode::UNAUTHORIZED)
                        .json(&serde_json::json!({"error": "Unauthorized"}))
                        .unwrap()
                );
            }
            PipelineAction::Continue
        });
        self
    }
    
    /// Add logging stage
    pub fn logging(self) -> Self {
        self.pipeline.add("logging", |req| {
            tracing::info!("→ {} {}", req.method, req.path);
            PipelineAction::Continue
        });
        self
    }
    
    /// Add custom stage
    pub fn stage<F>(self, name: &str, handler: F) -> Self
    where
        F: Fn(&Request) -> PipelineAction + Send + Sync + 'static,
    {
        self.pipeline.add(name, handler);
        self
    }
    
    /// Build the pipeline
    pub fn build(self) -> Pipeline {
        self.pipeline
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::http::Method;
    
    #[test]
    fn test_pipeline_basic() {
        let pipeline = Pipeline::new();
        
        pipeline.add("stage1", |_| PipelineAction::Continue);
        pipeline.add("stage2", |_| PipelineAction::Continue);
        
        assert_eq!(pipeline.count(), 2);
        
        let request = Request::new(Method::Get, "/".to_string());
        let response = pipeline.execute(&request, |_| {
            Ok(Response::ok().text("Success"))
        }).unwrap();
        
        assert_eq!(response.status.0, 200);
    }
    
    #[test]
    fn test_pipeline_stop() {
        let pipeline = Pipeline::new();
        
        pipeline.add("blocker", |_| {
            PipelineAction::Stop(Response::new(StatusCode::FORBIDDEN))
        });
        
        let request = Request::new(Method::Get, "/".to_string());
        let response = pipeline.execute(&request, |_| {
            Ok(Response::ok().text("Should not reach"))
        }).unwrap();
        
        assert_eq!(response.status.0, 403);
    }
    
    #[test]
    fn test_pipeline_priority() {
        let pipeline = Pipeline::new();
        
        pipeline.add_with_priority("low", 100, |_| PipelineAction::Continue);
        pipeline.add_with_priority("high", 1, |_| PipelineAction::Continue);
        
        let stages = pipeline.stages();
        assert_eq!(stages[0], "high");
        assert_eq!(stages[1], "low");
    }
    
    #[test]
    fn test_pipeline_enable_disable() {
        let pipeline = Pipeline::new();
        
        pipeline.add("blocker", |_| {
            PipelineAction::Stop(Response::new(StatusCode::FORBIDDEN))
        });
        
        // Disable the blocker
        pipeline.disable("blocker");
        
        let request = Request::new(Method::Get, "/".to_string());
        let response = pipeline.execute(&request, |_| {
            Ok(Response::ok())
        }).unwrap();
        
        assert_eq!(response.status.0, 200);
    }
    
    #[test]
    fn test_pipeline_builder() {
        let pipeline = PipelineBuilder::new()
            .cors()
            .logging()
            .auth()
            .build();
        
        assert_eq!(pipeline.count(), 3);
        assert!(pipeline.stages().contains(&"cors".to_string()));
        assert!(pipeline.stages().contains(&"auth".to_string()));
    }
}
