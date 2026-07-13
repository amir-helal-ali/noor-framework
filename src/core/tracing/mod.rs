// ============================================================
// OpenTelemetry Tracing - تتبع الموزع
// ============================================================
// Distributed tracing for monitoring request flow across services.
// Supports spans, traces, and context propagation.
//
// تتبع موزع لمراقبة تدفق الطلبات عبر الخدمات.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};
use std::time::Instant;

/// Span status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpanStatus {
    Unset,
    Ok,
    Error,
}

impl SpanStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unset => "unset",
            Self::Ok => "ok",
            Self::Error => "error",
        }
    }
}

/// Span kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpanKind {
    Internal,
    Server,
    Client,
    Producer,
    Consumer,
}

/// A span represents a unit of work
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub name: String,
    pub kind: SpanKind,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub duration_ms: Option<f64>,
    pub status: SpanStatus,
    pub attributes: HashMap<String, serde_json::Value>,
    pub events: Vec<SpanEvent>,
    pub links: Vec<SpanLink>,
    pub resource: HashMap<String, String>,
}

/// Span event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanEvent {
    pub name: String,
    pub timestamp: i64,
    pub attributes: HashMap<String, serde_json::Value>,
}

/// Span link to another trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanLink {
    pub trace_id: String,
    pub span_id: String,
    pub attributes: HashMap<String, serde_json::Value>,
}

impl Span {
    /// Create a new span
    pub fn new(name: &str, kind: SpanKind) -> Self {
        Self {
            trace_id: Self::generate_trace_id(),
            span_id: Self::generate_span_id(),
            parent_span_id: None,
            name: name.to_string(),
            kind,
            start_time: chrono::Utc::now().timestamp_millis(),
            end_time: None,
            duration_ms: None,
            status: SpanStatus::Unset,
            attributes: HashMap::new(),
            events: Vec::new(),
            links: Vec::new(),
            resource: HashMap::new(),
        }
    }
    
    /// Create a child span
    pub fn child(parent: &Span, name: &str, kind: SpanKind) -> Self {
        let mut span = Self::new(name, kind);
        span.trace_id = parent.trace_id.clone();
        span.parent_span_id = Some(parent.span_id.clone());
        span
    }
    
    /// Set an attribute
    pub fn set_attribute(&mut self, key: &str, value: impl Into<serde_json::Value>) {
        self.attributes.insert(key.to_string(), value.into());
    }
    
    /// Add an event
    pub fn add_event(&mut self, name: &str) {
        self.events.push(SpanEvent {
            name: name.to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            attributes: HashMap::new(),
        });
    }
    
    /// Add an event with attributes
    pub fn add_event_with_attrs(&mut self, name: &str, attrs: HashMap<String, serde_json::Value>) {
        self.events.push(SpanEvent {
            name: name.to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            attributes: attrs,
        });
    }
    
    /// Set status to OK
    pub fn set_ok(&mut self) {
        self.status = SpanStatus::Ok;
    }
    
    /// Set status to Error
    pub fn set_error(&mut self, message: &str) {
        self.status = SpanStatus::Error;
        self.add_event_with_attrs(
            "exception",
            {
                let mut attrs = HashMap::new();
                attrs.insert("exception.message".to_string(), serde_json::Value::String(message.to_string()));
                attrs
            },
        );
    }
    
    /// End the span
    pub fn end(&mut self) {
        let end_time = chrono::Utc::now().timestamp_millis();
        self.duration_ms = Some((end_time - self.start_time) as f64);
        self.end_time = Some(end_time);
    }
    
    /// Check if the span has ended
    pub fn is_ended(&self) -> bool {
        self.end_time.is_some()
    }
    
    /// Generate a trace ID (32 hex chars)
    fn generate_trace_id() -> String {
        let enc = crate::core::security::Encryption::new();
        enc.random_string(16).unwrap_or_else(|_| "0000000000000000".to_string())
    }
    
    /// Generate a span ID (16 hex chars)
    fn generate_span_id() -> String {
        let enc = crate::core::security::Encryption::new();
        enc.random_string(8).unwrap_or_else(|_| "00000000".to_string())
    }
}

/// Tracer for creating spans
pub struct Tracer {
    service_name: String,
    completed_spans: Arc<RwLock<Vec<Span>>>,
    current_spans: Arc<RwLock<HashMap<String, Span>>>,
}

impl Tracer {
    pub fn new(service_name: &str) -> Self {
        Self {
            service_name: service_name.to_string(),
            completed_spans: Arc::new(RwLock::new(Vec::new())),
            current_spans: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Start a new span
    pub fn start_span(&self, name: &str, kind: SpanKind) -> Span {
        let span = Span::new(name, kind);
        self.current_spans.write().insert(span.span_id.clone(), span.clone());
        span
    }
    
    /// Start a child span
    pub fn start_child_span(&self, parent: &Span, name: &str, kind: SpanKind) -> Span {
        let span = Span::child(parent, name, kind);
        self.current_spans.write().insert(span.span_id.clone(), span.clone());
        span
    }
    
    /// End a span and store it
    pub fn end_span(&self, mut span: Span) {
        span.end();
        
        // Add resource info
        span.resource.insert("service.name".to_string(), self.service_name.clone());
        
        self.current_spans.write().remove(&span.span_id);
        self.completed_spans.write().push(span);
    }
    
    /// Execute a function within a span
    pub fn with_span<F, T>(&self, name: &str, kind: SpanKind, f: F) -> crate::NoorResult<T>
    where
        F: FnOnce(&mut Span) -> crate::NoorResult<T>,
    {
        let mut span = self.start_span(name, kind);
        
        match f(&mut span) {
            Ok(result) => {
                span.set_ok();
                self.end_span(span);
                Ok(result)
            }
            Err(e) => {
                span.set_error(&e.to_string());
                self.end_span(span);
                Err(e)
            }
        }
    }
    
    /// Get all completed spans
    pub fn completed_spans(&self) -> Vec<Span> {
        self.completed_spans.read().clone()
    }
    
    /// Get spans for a specific trace
    pub fn trace_spans(&self, trace_id: &str) -> Vec<Span> {
        self.completed_spans
            .read()
            .iter()
            .filter(|s| s.trace_id == trace_id)
            .cloned()
            .collect()
    }
    
    /// Get the number of completed spans
    pub fn span_count(&self) -> usize {
        self.completed_spans.read().len()
    }
    
    /// Get the number of active spans
    pub fn active_span_count(&self) -> usize {
        self.current_spans.read().len()
    }
    
    /// Clear all spans
    pub fn clear(&self) {
        self.completed_spans.write().clear();
        self.current_spans.write().clear();
    }
    
    /// Export spans as JSON
    pub fn export_json(&self) -> serde_json::Value {
        serde_json::to_value(self.completed_spans.read().clone())
            .unwrap_or(serde_json::json!({"error": "Export failed"}))
    }
    
    /// Get trace statistics
    pub fn stats(&self) -> TracerStats {
        let spans = self.completed_spans.read();
        
        let total = spans.len();
        let ok_count = spans.iter().filter(|s| s.status == SpanStatus::Ok).count();
        let error_count = spans.iter().filter(|s| s.status == SpanStatus::Error).count();
        
        let avg_duration = if total == 0 {
            0.0
        } else {
            spans.iter()
                .filter_map(|s| s.duration_ms)
                .sum::<f64>() / total as f64
        };
        
        let traces: std::collections::HashSet<String> = spans.iter().map(|s| s.trace_id.clone()).collect();
        
        TracerStats {
            total_spans: total,
            ok_spans: ok_count,
            error_spans: error_count,
            total_traces: traces.len(),
            avg_duration_ms: avg_duration,
        }
    }
}

/// Tracer statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracerStats {
    pub total_spans: usize,
    pub ok_spans: usize,
    pub error_spans: usize,
    pub total_traces: usize,
    pub avg_duration_ms: f64,
}

/// Context for propagating trace information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceContext {
    pub trace_id: String,
    pub span_id: String,
    pub flags: u8,
}

impl TraceContext {
    /// Create a new context from a span
    pub fn from_span(span: &Span) -> Self {
        Self {
            trace_id: span.trace_id.clone(),
            span_id: span.span_id.clone(),
            flags: 0,
        }
    }
    
    /// Convert to W3C Trace Context header format
    pub fn to_header(&self) -> String {
        format!("00-{}-{}-{:02x}", self.trace_id, self.span_id, self.flags)
    }
    
    /// Parse from W3C Trace Context header
    pub fn from_header(header: &str) -> Option<Self> {
        let parts: Vec<&str> = header.split('-').collect();
        
        if parts.len() != 4 {
            return None;
        }
        
        let trace_id = parts[1].to_string();
        let span_id = parts[2].to_string();
        let flags = u8::from_str_radix(parts[3], 16).ok()?;
        
        Some(Self {
            trace_id,
            span_id,
            flags,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_span_creation() {
        let span = Span::new("test_operation", SpanKind::Internal);
        
        assert!(!span.trace_id.is_empty());
        assert!(!span.span_id.is_empty());
        assert!(span.parent_span_id.is_none());
        assert_eq!(span.status, SpanStatus::Unset);
        assert!(!span.is_ended());
    }
    
    #[test]
    fn test_child_span() {
        let parent = Span::new("parent", SpanKind::Server);
        let child = Span::child(&parent, "child", SpanKind::Internal);
        
        assert_eq!(child.trace_id, parent.trace_id);
        assert_eq!(child.parent_span_id, Some(parent.span_id));
    }
    
    #[test]
    fn test_span_end() {
        let mut span = Span::new("test", SpanKind::Internal);
        
        assert!(!span.is_ended());
        
        span.end();
        
        assert!(span.is_ended());
        assert!(span.duration_ms.is_some());
    }
    
    #[test]
    fn test_span_attributes() {
        let mut span = Span::new("test", SpanKind::Server);
        
        span.set_attribute("http.method", "GET");
        span.set_attribute("http.url", "/api/users");
        
        assert_eq!(span.attributes.len(), 2);
    }
    
    #[test]
    fn test_span_events() {
        let mut span = Span::new("test", SpanKind::Server);
        
        span.add_event("request_started");
        span.add_event("request_completed");
        
        assert_eq!(span.events.len(), 2);
    }
    
    #[test]
    fn test_span_error() {
        let mut span = Span::new("test", SpanKind::Server);
        
        span.set_error("Something went wrong");
        
        assert_eq!(span.status, SpanStatus::Error);
        assert_eq!(span.events.len(), 1);
        assert_eq!(span.events[0].name, "exception");
    }
    
    #[test]
    fn test_tracer() {
        let tracer = Tracer::new("noor-app");
        
        let result: crate::NoorResult<i32> = tracer.with_span("operation", SpanKind::Internal, |span| {
            span.set_attribute("key", "value");
            Ok(42)
        });
        
        assert_eq!(result.unwrap(), 42);
        assert_eq!(tracer.span_count(), 1);
    }
    
    #[test]
    fn test_tracer_error() {
        let tracer = Tracer::new("noor-app");
        
        let result: crate::NoorResult<i32> = tracer.with_span("failing", SpanKind::Internal, |_| {
            Err(crate::NoorError::Internal("Failed".to_string()))
        });
        
        assert!(result.is_err());
        assert_eq!(tracer.span_count(), 1);
        
        let spans = tracer.completed_spans();
        assert_eq!(spans[0].status, SpanStatus::Error);
    }
    
    #[test]
    fn test_trace_context() {
        let span = Span::new("test", SpanKind::Server);
        let ctx = TraceContext::from_span(&span);
        
        let header = ctx.to_header();
        assert!(header.starts_with("00-"));
        
        let parsed = TraceContext::from_header(&header);
        assert!(parsed.is_some());
        
        let parsed = parsed.unwrap();
        assert_eq!(parsed.trace_id, ctx.trace_id);
        assert_eq!(parsed.span_id, ctx.span_id);
    }
    
    #[test]
    fn test_tracer_stats() {
        let tracer = Tracer::new("noor-app");
        
        tracer.with_span("op1", SpanKind::Internal, |_| Ok(())).unwrap();
        let fail_result: crate::NoorResult<()> = tracer.with_span("op2", SpanKind::Internal, |_| Err(crate::NoorError::Internal("err".to_string())));
        fail_result.ok();
        
        let stats = tracer.stats();
        
        assert_eq!(stats.total_spans, 2);
        assert_eq!(stats.ok_spans, 1);
        assert_eq!(stats.error_spans, 1);
    }
}
