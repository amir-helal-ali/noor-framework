// ============================================================
// Noor Framework - Performance Benchmarks
// معايير قياس الأداء
// ============================================================
// Comprehensive benchmarks for all framework components.
// قياسات شاملة لجميع مكونات الإطار.
// ============================================================

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Benchmark result
#[derive(Debug, Clone, serde::Serialize)]
pub struct BenchmarkResult {
    pub name: String,
    pub iterations: u64,
    pub total_time_ms: u128,
    pub avg_time_ns: u64,
    pub ops_per_second: u64,
    pub min_time_ns: u64,
    pub max_time_ns: u64,
}

/// Benchmark runner
pub struct Benchmark {
    results: Vec<BenchmarkResult>,
}

impl Default for Benchmark {
    fn default() -> Self {
        Self::new()
    }
}

impl Benchmark {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }
    
    /// Run a benchmark
    pub fn run<F>(&mut self, name: &str, iterations: u64, mut f: F) -> &mut Self
    where
        F: FnMut(),
    {
        // Warmup
        for _ in 0..100 {
            f();
        }
        
        let mut min_time = u64::MAX;
        let mut max_time = 0u64;
        let start = Instant::now();
        
        for _ in 0..iterations {
            let iter_start = Instant::now();
            f();
            let elapsed = iter_start.elapsed().as_nanos() as u64;
            min_time = min_time.min(elapsed);
            max_time = max_time.max(elapsed);
        }
        
        let total_time = start.elapsed();
        
        let avg_time_ns = total_time.as_nanos() as u64 / iterations;
        let ops_per_second = if total_time.as_secs() > 0 {
            (iterations * 1_000_000_000) / total_time.as_nanos() as u64
        } else {
            0
        };
        
        let result = BenchmarkResult {
            name: name.to_string(),
            iterations,
            total_time_ms: total_time.as_millis(),
            avg_time_ns,
            ops_per_second,
            min_time_ns: min_time,
            max_time_ns: max_time,
        };
        
        self.results.push(result);
        self
    }
    
    /// Get all results
    pub fn results(&self) -> &[BenchmarkResult] {
        &self.results
    }
    
    /// Print results as a formatted table
    pub fn print_report(&self) {
        println!("\n{'='=<80}");
        println!("Noor Framework - Performance Benchmarks");
        println!("{'='=<80}");
        println!();
        println!(
            "{:<35} {:>10} {:>15} {:>15}",
            "Benchmark", "Iter", "Avg (ns)", "Ops/sec"
        );
        println!("{}", "-".repeat(80));
        
        for result in &self.results {
            println!(
                "{:<35} {:>10} {:>15} {:>15}",
                result.name,
                result.iterations,
                result.avg_time_ns,
                result.ops_per_second
            );
        }
        
        println!("{}", "-".repeat(80));
        println!();
    }
    
    /// Export results as JSON
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(&self.results).unwrap_or(serde_json::json!([]))
    }
}

/// Run all framework benchmarks
pub fn run_all_benchmarks() -> Benchmark {
    let mut bench = Benchmark::new();
    
    // HTTP routing
    bench.run("Router::match_route", 100_000, || {
        let router = crate::core::router::Router::new();
        let _ = router.match_route(&crate::core::http::Method::Get, "/users/123");
    });
    
    // Query builder
    bench.run("QueryBuilder::select", 100_000, || {
        let _ = crate::core::orm::QueryBuilder::table("users")
            .select(&["id", "name"])
            .where_("id", "=", 1)
            .to_sql();
    });
    
    // Advanced query builder
    bench.run("AdvancedQueryBuilder::select", 100_000, || {
        let _ = crate::core::advanced_query::AdvancedQueryBuilder::table("users")
            .select(&["id", "name"])
            .join("posts", "posts.user_id = users.id")
            .where_("users.active", "=", true)
            .order_by("users.created_at", "desc")
            .limit(10)
            .to_sql();
    });
    
    // Cache operations
    bench.run("MemoryCache::set", 100_000, || {
        let cache = crate::core::cache::MemoryCache::new(1000, "bench:");
        cache.set("key", b"value", 60).unwrap();
    });
    
    bench.run("MemoryCache::get", 100_000, || {
        let cache = crate::core::cache::MemoryCache::new(1000, "bench:");
        cache.set("key", b"value", 60).unwrap();
        let _ = cache.get("key");
    });
    
    // Encryption
    bench.run("Encryption::sha256", 10_000, || {
        let _ = crate::core::security::Encryption::sha256_hex(b"benchmark data");
    });
    
    // CSRF
    bench.run("Csrf::validate_token", 10_000, || {
        let csrf = crate::core::security::Csrf::new(3600);
        let token = csrf.generate_token().unwrap();
        let _ = csrf.validate_token(&token);
    });
    
    // JWT
    bench.run("Jwt::generate_token", 10_000, || {
        let jwt = crate::core::auth::Jwt::new("secret", "noor", "noor_app");
        let _ = jwt.generate_access_token("user123", vec!["admin".to_string()]);
    });
    
    bench.run("Jwt::verify", 10_000, || {
        let jwt = crate::core::auth::Jwt::new("secret", "noor", "noor_app");
        let token = jwt.generate_access_token("user123", vec![]).unwrap();
        let _ = jwt.verify(&token);
    });
    
    // Validation
    bench.run("Validator::is_email", 100_000, || {
        let _ = crate::core::security::Validator::is_email("test@example.com");
    });
    
    // XSS
    bench.run("Xss::escape", 100_000, || {
        let _ = crate::core::security::Xss::escape("<script>alert('xss')</script>");
    });
    
    // Rate limiting
    bench.run("RateLimit::check", 100_000, || {
        let limiter = crate::core::security::RateLimit::new(1000, 60);
        let _ = limiter.check("192.168.1.1");
    });
    
    // Pagination
    bench.run("Pagination::from", 100_000, || {
        let _ = crate::core::pagination::Pagination::from(1000, 5, 20);
    });
    
    // Event emitter
    bench.run("EventEmitter::fire", 100_000, || {
        let emitter = crate::core::events::EventEmitter::new();
        emitter.on("test", std::sync::Arc::new(|_| Ok(())));
        let _ = emitter.fire("test", serde_json::json!({}));
    });
    
    // Notification
    bench.run("Notification::create", 100_000, || {
        let _ = crate::core::notification::Notification::new(
            "user1",
            "Test",
            "Body",
            crate::core::notification::Channel::InApp,
        );
    });
    
    // Feature flags
    bench.run("FeatureFlag::is_enabled", 100_000, || {
        let manager = crate::core::features::FeatureFlagManager::new();
        manager.boolean("test", "Test", true);
        let _ = manager.is_enabled("test");
    });
    
    // Tracing
    bench.run("Tracer::with_span", 10_000, || {
        let tracer = crate::core::tracing::Tracer::new("bench");
        let _ = tracer.with_span("operation", crate::core::tracing::SpanKind::Internal, |_| Ok(()));
    });
    
    // Transaction
    bench.run("TransactionManager::begin_commit", 100_000, || {
        let manager = crate::core::transactions::TransactionManager::new();
        manager.begin().unwrap();
        manager.commit().unwrap();
    });
    
    // Circuit breaker
    bench.run("CircuitBreaker::allow_request", 100_000, || {
        let breaker = crate::core::circuit_breaker::CircuitBreaker::new(
            "test",
            crate::core::circuit_breaker::CircuitConfig::default(),
        );
        let _ = breaker.allow_request();
    });
    
    // Cookie
    bench.run("Cookie::to_header", 100_000, || {
        let cookie = crate::core::cookies::Cookie::new("session", "abc123")
            .max_age(3600)
            .secure()
            .http_only();
        let _ = cookie.to_header();
    });
    
    // DTO validation
    bench.run("DTO::validate", 10_000, || {
        let dto = crate::core::dto::LoginDto {
            email: "user@example.com".to_string(),
            password: "password123".to_string(),
            remember: None,
        };
        let _ = dto.validate();
    });
    
    bench
}

/// Main benchmark entry point
pub fn main() {
    println!("\n🚀 Running Noor Framework benchmarks...\n");
    
    let bench = run_all_benchmarks();
    bench.print_report();
    
    // Export results
    let json = bench.to_json();
    println!("📊 Results exported as JSON:");
    println!("{}", serde_json::to_string_pretty(&json).unwrap());
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_benchmark_runs() {
        let mut bench = Benchmark::new();
        
        bench.run("test_operation", 1000, || {
            let _ = 1 + 1;
        });
        
        assert_eq!(bench.results().len(), 1);
        assert_eq!(bench.results()[0].iterations, 1000);
        assert!(bench.results()[0].ops_per_second > 0);
    }
    
    #[test]
    fn test_benchmark_json_export() {
        let mut bench = Benchmark::new();
        
        bench.run("test", 100, || {});
        
        let json = bench.to_json();
        assert!(json.is_array());
        assert_eq!(json[0]["name"], "test");
    }
}
