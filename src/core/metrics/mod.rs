// ============================================================
// Metrics & Monitoring - المراقبة والأداء
// ============================================================
// Real-time metrics collection for monitoring application
// health, performance, and usage patterns.
//
// جمع المقاييس الفورية لمراقبة صحة التطبيق والأداء.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicI64, Ordering};
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};
use std::time::{Duration, Instant};

/// Counter metric (monotonically increasing)
/// عداد (يزيد دائماً)
pub struct Counter {
    name: String,
    description: String,
    value: AtomicU64,
    labels: HashMap<String, String>,
}

impl Counter {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            value: AtomicU64::new(0),
            labels: HashMap::new(),
        }
    }
    
    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn inc_by(&self, delta: u64) {
        self.value.fetch_add(delta, Ordering::Relaxed);
    }
    
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }
    
    pub fn name(&self) -> &str {
        &self.name
    }
    
    pub fn reset(&self) {
        self.value.store(0, Ordering::Relaxed);
    }
}

/// Gauge metric (can go up or down)
/// مقياس (يرتفع وينخفض)
pub struct Gauge {
    name: String,
    description: String,
    value: AtomicI64,
}

impl Gauge {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            value: AtomicI64::new(0),
        }
    }
    
    pub fn set(&self, value: i64) {
        self.value.store(value, Ordering::Relaxed);
    }
    
    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn dec(&self) {
        self.value.fetch_sub(1, Ordering::Relaxed);
    }
    
    pub fn add(&self, delta: i64) {
        self.value.fetch_add(delta, Ordering::Relaxed);
    }
    
    pub fn get(&self) -> i64 {
        self.value.load(Ordering::Relaxed)
    }
    
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Histogram metric (tracks distribution of values)
/// مدرج تكراري (يتتبع توزيع القيم)
pub struct Histogram {
    name: String,
    description: String,
    buckets: Vec<f64>,
    counts: Vec<AtomicU64>,
    sum: RwLock<f64>,
    count: AtomicU64,
}

impl Histogram {
    pub fn new(name: &str, description: &str, buckets: Vec<f64>) -> Self {
        let counts = (0..buckets.len() + 1)
            .map(|_| AtomicU64::new(0))
            .collect();
        
        Self {
            name: name.to_string(),
            description: description.to_string(),
            buckets,
            counts,
            sum: RwLock::new(0.0),
            count: AtomicU64::new(0),
        }
    }
    
    /// Default latency buckets (in milliseconds)
    pub fn latency_buckets() -> Vec<f64> {
        vec![0.5, 1.0, 2.5, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0]
    }
    
    pub fn observe(&self, value: f64) {
        self.count.fetch_add(1, Ordering::Relaxed);
        *self.sum.write() += value;
        
        // Increment appropriate bucket
        for (i, &bucket) in self.buckets.iter().enumerate() {
            if value <= bucket {
                self.counts[i].fetch_add(1, Ordering::Relaxed);
                return;
            }
        }
        
        // Value exceeds all buckets - increment overflow bucket
        if let Some(last) = self.counts.last() {
            last.fetch_add(1, Ordering::Relaxed);
        }
    }
    
    pub fn count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }
    
    pub fn sum(&self) -> f64 {
        *self.sum.read()
    }
    
    pub fn avg(&self) -> f64 {
        let count = self.count();
        if count == 0 {
            0.0
        } else {
            self.sum() / count as f64
        }
    }
    
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Metrics registry
/// سجل المقاييس
pub struct MetricsRegistry {
    counters: Arc<RwLock<HashMap<String, Arc<Counter>>>>,
    gauges: Arc<RwLock<HashMap<String, Arc<Gauge>>>>,
    histograms: Arc<RwLock<HashMap<String, Arc<Histogram>>>>,
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsRegistry {
    pub fn new() -> Self {
        Self {
            counters: Arc::new(RwLock::new(HashMap::new())),
            gauges: Arc::new(RwLock::new(HashMap::new())),
            histograms: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register or get a counter
    pub fn counter(&self, name: &str, description: &str) -> Arc<Counter> {
        let mut counters = self.counters.write();
        counters
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(Counter::new(name, description)))
            .clone()
    }
    
    /// Register or get a gauge
    pub fn gauge(&self, name: &str, description: &str) -> Arc<Gauge> {
        let mut gauges = self.gauges.write();
        gauges
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(Gauge::new(name, description)))
            .clone()
    }
    
    /// Register or get a histogram
    pub fn histogram(&self, name: &str, description: &str) -> Arc<Histogram> {
        self.histogram_with_buckets(name, description, Histogram::latency_buckets())
    }
    
    /// Register or get a histogram with custom buckets
    pub fn histogram_with_buckets(&self, name: &str, description: &str, buckets: Vec<f64>) -> Arc<Histogram> {
        let mut histograms = self.histograms.write();
        histograms
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(Histogram::new(name, description, buckets)))
            .clone()
    }
    
    /// Export all metrics as JSON
    pub fn export_json(&self) -> serde_json::Value {
        let counters: Vec<serde_json::Value> = self.counters
            .read()
            .values()
            .map(|c| serde_json::json!({
                "name": c.name(),
                "value": c.get(),
            }))
            .collect();
        
        let gauges: Vec<serde_json::Value> = self.gauges
            .read()
            .values()
            .map(|g| serde_json::json!({
                "name": g.name(),
                "value": g.get(),
            }))
            .collect();
        
        let histograms: Vec<serde_json::Value> = self.histograms
            .read()
            .values()
            .map(|h| serde_json::json!({
                "name": h.name(),
                "count": h.count(),
                "sum": h.sum(),
                "avg": h.avg(),
            }))
            .collect();
        
        serde_json::json!({
            "counters": counters,
            "gauges": gauges,
            "histograms": histograms,
        })
    }
    
    /// Export metrics in Prometheus format
    pub fn export_prometheus(&self) -> String {
        let mut output = String::new();
        
        // Counters
        for counter in self.counters.read().values() {
            output.push_str(&format!("# HELP {} \n", counter.name()));
            output.push_str(&format!("# TYPE {} counter\n", counter.name()));
            output.push_str(&format!("{} {}\n", counter.name(), counter.get()));
        }
        
        // Gauges
        for gauge in self.gauges.read().values() {
            output.push_str(&format!("# HELP {} \n", gauge.name()));
            output.push_str(&format!("# TYPE {} gauge\n", gauge.name()));
            output.push_str(&format!("{} {}\n", gauge.name(), gauge.get()));
        }
        
        // Histograms
        for histogram in self.histograms.read().values() {
            output.push_str(&format!("# HELP {} \n", histogram.name()));
            output.push_str(&format!("# TYPE {} histogram\n", histogram.name()));
            output.push_str(&format!("{}_count {}\n", histogram.name(), histogram.count()));
            output.push_str(&format!("{}_sum {}\n", histogram.name(), histogram.sum()));
        }
        
        output
    }
}

/// Application metrics (pre-configured common metrics)
/// مقاييس التطبيق (مقاييس شائعة جاهزة)
pub struct AppMetrics {
    pub registry: Arc<MetricsRegistry>,
    pub requests_total: Arc<Counter>,
    pub requests_in_flight: Arc<Gauge>,
    pub request_duration: Arc<Histogram>,
    pub response_size: Arc<Histogram>,
    pub errors_total: Arc<Counter>,
    pub db_queries: Arc<Counter>,
    pub db_query_duration: Arc<Histogram>,
    pub cache_hits: Arc<Counter>,
    pub cache_misses: Arc<Counter>,
}

impl Default for AppMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl AppMetrics {
    pub fn new() -> Self {
        let registry = Arc::new(MetricsRegistry::new());
        
        Self {
            requests_total: registry.counter("http_requests_total", "Total HTTP requests"),
            requests_in_flight: registry.gauge("http_requests_in_flight", "Current in-flight requests"),
            request_duration: registry.histogram("http_request_duration_ms", "HTTP request duration in milliseconds"),
            response_size: registry.histogram("http_response_size_bytes", "HTTP response size in bytes"),
            errors_total: registry.counter("http_errors_total", "Total HTTP errors (5xx)"),
            db_queries: registry.counter("db_queries_total", "Total database queries"),
            db_query_duration: registry.histogram("db_query_duration_ms", "Database query duration in milliseconds"),
            cache_hits: registry.counter("cache_hits_total", "Cache hits"),
            cache_misses: registry.counter("cache_misses_total", "Cache misses"),
            registry,
        }
    }
    
    /// Record a request
    pub fn record_request(&self, duration: Duration, response_size: u64, is_error: bool) {
        self.requests_total.inc();
        self.request_duration.observe(duration.as_millis() as f64);
        self.response_size.observe(response_size as f64);
        
        if is_error {
            self.errors_total.inc();
        }
    }
    
    /// Record a cache hit
    pub fn record_cache_hit(&self) {
        self.cache_hits.inc();
    }
    
    /// Record a cache miss
    pub fn record_cache_miss(&self) {
        self.cache_misses.inc();
    }
    
    /// Record a database query
    pub fn record_db_query(&self, duration: Duration) {
        self.db_queries.inc();
        self.db_query_duration.observe(duration.as_millis() as f64);
    }
    
    /// Get cache hit rate
    pub fn cache_hit_rate(&self) -> f64 {
        let hits = self.cache_hits.get() as f64;
        let misses = self.cache_misses.get() as f64;
        let total = hits + misses;
        
        if total == 0.0 {
            0.0
        } else {
            hits / total * 100.0
        }
    }
}

/// Timer for measuring code execution duration
/// مؤقت لقياس مدة تنفيذ الكود
pub struct Timer {
    start: Instant,
    histogram: Option<Arc<Histogram>>,
}

impl Timer {
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
            histogram: None,
        }
    }
    
    pub fn start_with_metric(histogram: Arc<Histogram>) -> Self {
        Self {
            start: Instant::now(),
            histogram: Some(histogram),
        }
    }
    
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
    
    pub fn elapsed_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
    
    pub fn stop(self) -> Duration {
        let duration = self.start.elapsed();
        if let Some(histogram) = self.histogram {
            histogram.observe(duration.as_millis() as f64);
        }
        duration
    }
}

/// Health status for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,  // "healthy", "degraded", "unhealthy"
    pub timestamp: i64,
    pub uptime_seconds: u64,
    pub checks: Vec<HealthCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub name: String,
    pub status: String,
    pub message: String,
    pub duration_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_counter() {
        let counter = Counter::new("test", "Test counter");
        counter.inc();
        counter.inc();
        counter.inc_by(5);
        assert_eq!(counter.get(), 7);
    }
    
    #[test]
    fn test_gauge() {
        let gauge = Gauge::new("test", "Test gauge");
        gauge.set(10);
        gauge.inc();
        gauge.dec();
        assert_eq!(gauge.get(), 10);
    }
    
    #[test]
    fn test_histogram() {
        let histogram = Histogram::new("test", "Test", vec![1.0, 5.0, 10.0]);
        histogram.observe(0.5);
        histogram.observe(3.0);
        histogram.observe(7.0);
        histogram.observe(15.0);
        
        assert_eq!(histogram.count(), 4);
        assert!((histogram.sum() - 25.5).abs() < 0.001);
    }
    
    #[test]
    fn test_metrics_registry() {
        let registry = MetricsRegistry::new();
        let counter1 = registry.counter("requests", "Request count");
        let counter2 = registry.counter("requests", "Request count");
        
        // Should return the same counter
        assert!(Arc::ptr_eq(&counter1, &counter2));
    }
    
    #[test]
    fn test_app_metrics() {
        let metrics = AppMetrics::new();
        
        metrics.record_request(Duration::from_millis(50), 1024, false);
        metrics.record_request(Duration::from_millis(100), 2048, true);
        metrics.record_cache_hit();
        metrics.record_cache_hit();
        metrics.record_cache_miss();
        
        assert_eq!(metrics.requests_total.get(), 2);
        assert_eq!(metrics.errors_total.get(), 1);
        assert_eq!(metrics.cache_hits.get(), 2);
        assert_eq!(metrics.cache_misses.get(), 1);
        assert!((metrics.cache_hit_rate() - 66.67).abs() < 0.1);
    }
    
    #[test]
    fn test_timer() {
        let timer = Timer::start();
        std::thread::sleep(Duration::from_millis(10));
        let elapsed = timer.stop();
        
        assert!(elapsed.as_millis() >= 10);
    }
}
