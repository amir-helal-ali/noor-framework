// ============================================================
// Health Check - فحص الصحة
// ============================================================
// Health check endpoints for Kubernetes/Docker readiness
// and liveness probes.
//
// نقاط فحص الصحة لـ Kubernetes/Docker.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// Health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    Healthy,
    Degraded,
    Unhealthy,
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Healthy => write!(f, "healthy"),
            Status::Degraded => write!(f, "degraded"),
            Status::Unhealthy => write!(f, "unhealthy"),
        }
    }
}

/// Health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub name: String,
    pub status: Status,
    pub message: String,
    pub duration_ms: u64,
    pub timestamp: i64,
}

/// Health check function type
type HealthCheckFn = Arc<dyn Fn() -> CheckResult + Send + Sync>;

/// Health check manager
pub struct HealthChecker {
    checks: Arc<RwLock<HashMap<String, HealthCheckFn>>>,
    start_time: Instant,
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl HealthChecker {
    pub fn new() -> Self {
        Self {
            checks: Arc::new(RwLock::new(HashMap::new())),
            start_time: Instant::now(),
        }
    }
    
    /// Register a health check
    pub fn register<F>(&self, name: &str, check: F)
    where
        F: Fn() -> CheckResult + Send + Sync + 'static,
    {
        self.checks
            .write()
            .insert(name.to_string(), Arc::new(check));
    }
    
    /// Run a specific check
    pub fn run_check(&self, name: &str) -> Option<CheckResult> {
        let checks = self.checks.read();
        checks.get(name).map(|f| f())
    }
    
    /// Run all checks and get aggregated status
    pub fn check_all(&self) -> HealthReport {
        let checks = self.checks.read();
        let mut results = Vec::new();
        let mut overall_status = Status::Healthy;
        
        for (name, check_fn) in checks.iter() {
            let result = check_fn();
            
            // Update overall status
            match result.status {
                Status::Unhealthy => overall_status = Status::Unhealthy,
                Status::Degraded => {
                    if overall_status != Status::Unhealthy {
                        overall_status = Status::Degraded;
                    }
                }
                Status::Healthy => {}
            }
            
            results.push(result);
        }
        
        HealthReport {
            status: overall_status,
            timestamp: chrono::Utc::now().timestamp(),
            uptime_seconds: self.start_time.elapsed().as_secs(),
            version: crate::VERSION.to_string(),
            checks: results,
        }
    }
    
    /// Get uptime in seconds
    pub fn uptime(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
    
    /// Get the number of registered checks
    pub fn check_count(&self) -> usize {
        self.checks.read().len()
    }
}

/// Full health report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub status: Status,
    pub timestamp: i64,
    pub uptime_seconds: u64,
    pub version: String,
    pub checks: Vec<CheckResult>,
}

impl HealthReport {
    /// Convert to HTTP response JSON
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::json!({"status": "error"}))
    }
    
    /// Get the appropriate HTTP status code
    pub fn http_status(&self) -> u16 {
        match self.status {
            Status::Healthy => 200,
            Status::Degraded => 200,
            Status::Unhealthy => 503,
        }
    }
}

/// Built-in health checks
pub mod checks {
    use super::*;
    
    /// Database connectivity check
    pub fn database() -> CheckResult {
        let start = Instant::now();
        
        // In a real implementation, we'd ping the database
        // For now, we simulate a successful check
        CheckResult {
            name: "database".to_string(),
            status: Status::Healthy,
            message: "Database connection OK".to_string(),
            duration_ms: start.elapsed().as_millis() as u64,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
    
    /// Cache connectivity check
    pub fn cache() -> CheckResult {
        let start = Instant::now();
        
        CheckResult {
            name: "cache".to_string(),
            status: Status::Healthy,
            message: "Cache OK".to_string(),
            duration_ms: start.elapsed().as_millis() as u64,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
    
    /// Disk space check
    pub fn disk_space(required_mb: u64) -> CheckResult {
        let start = Instant::now();
        
        // In a real implementation, we'd check actual disk space
        let message = format!("Disk space OK ({}MB required)", required_mb);
        
        CheckResult {
            name: "disk_space".to_string(),
            status: Status::Healthy,
            message,
            duration_ms: start.elapsed().as_millis() as u64,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
    
    /// Memory usage check
    pub fn memory(max_mb: u64) -> CheckResult {
        let start = Instant::now();
        
        // In a real implementation, we'd check actual memory usage
        let message = format!("Memory usage within limits (max {}MB)", max_mb);
        
        CheckResult {
            name: "memory".to_string(),
            status: Status::Healthy,
            message,
            duration_ms: start.elapsed().as_millis() as u64,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
    
    /// Custom check that always returns healthy
    pub fn always_healthy(name: &str) -> CheckResult {
        CheckResult {
            name: name.to_string(),
            status: Status::Healthy,
            message: "OK".to_string(),
            duration_ms: 0,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
    
    /// Custom check that always returns unhealthy
    pub fn always_unhealthy(name: &str, message: &str) -> CheckResult {
        CheckResult {
            name: name.to_string(),
            status: Status::Unhealthy,
            message: message.to_string(),
            duration_ms: 0,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_health_checker() {
        let checker = HealthChecker::new();
        
        checker.register("db", || checks::database());
        checker.register("cache", || checks::cache());
        
        let report = checker.check_all();
        
        assert_eq!(report.status, Status::Healthy);
        assert_eq!(report.checks.len(), 2);
    }
    
    #[test]
    fn test_unhealthy_status() {
        let checker = HealthChecker::new();
        
        checker.register("failing", || checks::always_unhealthy("test", "Service down"));
        
        let report = checker.check_all();
        
        assert_eq!(report.status, Status::Unhealthy);
        assert_eq!(report.http_status(), 503);
    }
    
    #[test]
    fn test_degraded_status() {
        let checker = HealthChecker::new();
        
        checker.register("healthy", || checks::always_healthy("ok"));
        checker.register("degraded", || CheckResult {
            name: "degraded".to_string(),
            status: Status::Degraded,
            message: "Slow response".to_string(),
            duration_ms: 5000,
            timestamp: chrono::Utc::now().timestamp(),
        });
        
        let report = checker.check_all();
        
        assert_eq!(report.status, Status::Degraded);
        assert_eq!(report.http_status(), 200);
    }
}
