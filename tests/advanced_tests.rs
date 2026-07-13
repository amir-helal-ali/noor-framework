// ============================================================
// Advanced Tests - اختبارات متقدمة
// ============================================================
// Tests for the advanced components added in v1.1+
// اختبارات للمكونات المتقدمة المضافة في v1.1+
// ============================================================

use noor::{
    core::plugins::{PluginManager, Plugin, PluginInfo, DebugPlugin, StatsPlugin},
    core::metrics::{MetricsRegistry, AppMetrics, Counter, Gauge, Histogram, Timer},
    core::storage::{LocalStorage, Storage, StorageManager, Visibility},
    core::pagination::{Pagination, PaginatedResult, PaginationParams, Sort, SortDirection},
    core::health::{HealthChecker, Status as HealthStatus, checks as health_checks},
    core::graphql::{GraphQLResolver, SchemaBuilder, FieldBuilder, GraphQLType},
};
use std::sync::Arc;
use std::time::Duration;

// ============= Plugin System Tests =============

#[test]
fn test_plugin_registration() {
    let manager = PluginManager::new();
    let plugin = Arc::new(DebugPlugin::new());
    
    manager.register(plugin).unwrap();
    manager.boot_all().unwrap();
    
    assert_eq!(manager.count(), 1);
    
    let plugins = manager.list();
    assert_eq!(plugins[0].name, "debug");
    assert_eq!(plugins[0].version, "1.0.0");
}

#[test]
fn test_plugin_enable_disable() {
    let manager = PluginManager::new();
    manager.register(Arc::new(DebugPlugin::new())).unwrap();
    
    assert!(manager.is_enabled("debug"));
    
    manager.disable("debug");
    assert!(!manager.is_enabled("debug"));
    
    manager.enable("debug");
    assert!(manager.is_enabled("debug"));
}

#[test]
fn test_stats_plugin_tracking() {
    let manager = PluginManager::new();
    let stats = Arc::new(StatsPlugin::new());
    
    manager.register(stats.clone()).unwrap();
    manager.boot_all().unwrap();
    
    let request = noor::core::http::Request::new(
        noor::core::http::Method::Get,
        "/".to_string(),
    );
    
    // Simulate 5 requests
    for _ in 0..5 {
        manager.before_request(&request).unwrap();
    }
    
    assert_eq!(stats.request_count(), 5);
}

// ============= Metrics Tests =============

#[test]
fn test_counter_operations() {
    let counter = Counter::new("requests", "Total requests");
    
    counter.inc();
    counter.inc();
    counter.inc_by(5);
    
    assert_eq!(counter.get(), 7);
    
    counter.reset();
    assert_eq!(counter.get(), 0);
}

#[test]
fn test_gauge_operations() {
    let gauge = Gauge::new("active_users", "Active users");
    
    gauge.set(10);
    gauge.inc();
    gauge.inc();
    gauge.dec();
    gauge.add(5);
    
    assert_eq!(gauge.get(), 16); // 10 + 1 + 1 - 1 + 5 = 16
}

#[test]
fn test_histogram_observations() {
    let histogram = Histogram::new("latency", "Request latency", vec![1.0, 5.0, 10.0]);
    
    histogram.observe(0.5);
    histogram.observe(3.0);
    histogram.observe(7.0);
    histogram.observe(15.0);
    
    assert_eq!(histogram.count(), 4);
    assert!((histogram.sum() - 25.5).abs() < 0.001);
    assert!((histogram.avg() - 6.375).abs() < 0.001);
}

#[test]
fn test_metrics_registry() {
    let registry = MetricsRegistry::new();
    
    let counter1 = registry.counter("requests", "Request count");
    let counter2 = registry.counter("requests", "Request count");
    
    // Should return the same counter
    assert!(Arc::ptr_eq(&counter1, &counter2));
    
    counter1.inc();
    assert_eq!(counter2.get(), 1); // Same counter
}

#[test]
fn test_app_metrics() {
    let metrics = AppMetrics::new();
    
    metrics.record_request(Duration::from_millis(50), 1024, false);
    metrics.record_request(Duration::from_millis(100), 2048, false);
    metrics.record_request(Duration::from_millis(200), 512, true);
    
    metrics.record_cache_hit();
    metrics.record_cache_hit();
    metrics.record_cache_hit();
    metrics.record_cache_miss();
    
    assert_eq!(metrics.requests_total.get(), 3);
    assert_eq!(metrics.errors_total.get(), 1);
    assert_eq!(metrics.cache_hits.get(), 3);
    assert_eq!(metrics.cache_misses.get(), 1);
    
    let hit_rate = metrics.cache_hit_rate();
    assert!((hit_rate - 75.0).abs() < 0.1);
}

#[test]
fn test_metrics_export_json() {
    let registry = MetricsRegistry::new();
    
    let counter = registry.counter("test_counter", "Test");
    counter.inc_by(42);
    
    let gauge = registry.gauge("test_gauge", "Test");
    gauge.set(100);
    
    let export = registry.export_json();
    
    assert!(export["counters"].is_array());
    assert!(export["gauges"].is_array());
}

#[test]
fn test_timer() {
    let timer = Timer::start();
    std::thread::sleep(Duration::from_millis(10));
    let elapsed = timer.stop();
    
    assert!(elapsed.as_millis() >= 10);
}

// ============= Storage Tests =============

#[tokio::test]
async fn test_local_storage_operations() {
    let storage = LocalStorage::new("/tmp/noor_advanced_test", "/storage").unwrap();
    
    // Write
    storage.write("test/file.txt", b"Hello, Storage!", Visibility::Public).await.unwrap();
    
    // Read
    let content = storage.read("test/file.txt").await.unwrap();
    assert_eq!(content, b"Hello, Storage!");
    
    // Exists
    assert!(storage.exists("test/file.txt").await.unwrap());
    assert!(!storage.exists("nonexistent.txt").await.unwrap());
    
    // Metadata
    let meta = storage.metadata("test/file.txt").await.unwrap();
    assert_eq!(meta.size, 15);
    assert!(meta.exists);
    assert_eq!(meta.content_type, "text/plain");
    
    // URL
    let url = storage.url("test/file.txt").await.unwrap();
    assert_eq!(url, "/storage/test/file.txt");
    
    // Copy
    storage.copy("test/file.txt", "test/copy.txt").await.unwrap();
    assert!(storage.exists("test/copy.txt").await.unwrap());
    
    // Move
    storage.move_file("test/copy.txt", "test/moved.txt").await.unwrap();
    assert!(!storage.exists("test/copy.txt").await.unwrap());
    assert!(storage.exists("test/moved.txt").await.unwrap());
    
    // List
    let files = storage.list("test").await.unwrap();
    assert!(files.len() >= 2);
    
    // Delete
    assert!(storage.delete("test/file.txt").await.unwrap());
    assert!(storage.delete("test/moved.txt").await.unwrap());
}

#[tokio::test]
async fn test_storage_manager() {
    let mut manager = StorageManager::new();
    
    let local = Arc::new(LocalStorage::new("/tmp/noor_manager_test", "/storage").unwrap());
    manager.disk("local", local);
    manager.set_default("local");
    
    let storage = manager.default_disk();
    assert!(storage.is_some());
    
    let storage = storage.unwrap();
    storage.write("test.txt", b"test", Visibility::Public).await.unwrap();
    
    let content = storage.read("test.txt").await.unwrap();
    assert_eq!(content, b"test");
}

// ============= Pagination Tests =============

#[test]
fn test_pagination_basic() {
    let pagination = Pagination::from(100, 1, 15);
    
    assert_eq!(pagination.current_page, 1);
    assert_eq!(pagination.per_page, 15);
    assert_eq!(pagination.total, 100);
    assert_eq!(pagination.last_page, 7);
    assert_eq!(pagination.from, Some(1));
    assert_eq!(pagination.to, Some(15));
    assert!(pagination.has_more_pages);
}

#[test]
fn test_pagination_last_page() {
    let pagination = Pagination::from(100, 7, 15);
    
    assert_eq!(pagination.from, Some(91));
    assert_eq!(pagination.to, Some(100));
    assert!(!pagination.has_more_pages);
}

#[test]
fn test_pagination_empty() {
    let pagination = Pagination::from(0, 1, 15);
    
    assert_eq!(pagination.last_page, 1);
    assert_eq!(pagination.from, None);
    assert_eq!(pagination.to, None);
    assert!(!pagination.has_more_pages);
}

#[test]
fn test_paginated_result_creation() {
    let data = vec![1, 2, 3, 4, 5];
    let result = PaginatedResult::new(data, 100, 1, 5, "/api/posts");
    
    assert_eq!(result.data.len(), 5);
    assert_eq!(result.meta.total, 100);
    assert_eq!(result.meta.last_page, 20);
    assert!(result.links.next.is_some());
    assert!(result.links.prev.is_none());
}

#[test]
fn test_paginated_result_map() {
    let data = vec![1, 2, 3];
    let result = PaginatedResult::new(data, 100, 1, 3, "/api/numbers");
    
    let mapped = result.map(|n| n * 2);
    
    assert_eq!(mapped.data, vec![2, 4, 6]);
    assert_eq!(mapped.meta.total, 100);
}

#[test]
fn test_sort_parsing() {
    let sort = Sort::from_query("name");
    assert_eq!(sort.column, "name");
    assert_eq!(sort.direction, SortDirection::Asc);
    
    let sort = Sort::from_query("-created_at");
    assert_eq!(sort.column, "created_at");
    assert_eq!(sort.direction, SortDirection::Desc);
}

#[test]
fn test_sort_to_sql() {
    let sort_asc = Sort::new("name", SortDirection::Asc);
    assert_eq!(sort_asc.to_sql(), "name ASC");
    
    let sort_desc = Sort::new("created_at", SortDirection::Desc);
    assert_eq!(sort_desc.to_sql(), "created_at DESC");
}

#[test]
fn test_pagination_params_from_query() {
    use std::collections::HashMap;
    
    let mut query = HashMap::new();
    query.insert("page".to_string(), "3".to_string());
    query.insert("per_page".to_string(), "20".to_string());
    query.insert("sort".to_string(), "-name".to_string());
    
    let params = PaginationParams::from_query(&query);
    
    assert_eq!(params.page, 3);
    assert_eq!(params.per_page, 20);
    assert_eq!(params.offset(), 40);
    assert_eq!(params.limit(), 20);
    assert!(params.sort.is_some());
}

// ============= Health Check Tests =============

#[test]
fn test_health_checker_all_healthy() {
    let checker = HealthChecker::new();
    
    checker.register("database", || health_checks::database());
    checker.register("cache", || health_checks::cache());
    
    let report = checker.check_all();
    
    assert_eq!(report.status, HealthStatus::Healthy);
    assert_eq!(report.checks.len(), 2);
    assert_eq!(report.http_status(), 200);
}

#[test]
fn test_health_checker_with_unhealthy() {
    let checker = HealthChecker::new();
    
    checker.register("healthy_service", || health_checks::always_healthy("ok"));
    checker.register("failing_service", || health_checks::always_unhealthy("test", "Service down"));
    
    let report = checker.check_all();
    
    assert_eq!(report.status, HealthStatus::Unhealthy);
    assert_eq!(report.http_status(), 503);
}

#[test]
fn test_health_checker_with_degraded() {
    let checker = HealthChecker::new();
    
    checker.register("healthy", || health_checks::always_healthy("ok"));
    checker.register("degraded", || noor::core::health::CheckResult {
        name: "degraded".to_string(),
        status: HealthStatus::Degraded,
        message: "Slow response".to_string(),
        duration_ms: 5000,
        timestamp: chrono::Utc::now().timestamp(),
    });
    
    let report = checker.check_all();
    
    assert_eq!(report.status, HealthStatus::Degraded);
    assert_eq!(report.http_status(), 200);
}

#[test]
fn test_health_report_json() {
    let checker = HealthChecker::new();
    checker.register("db", || health_checks::database());
    
    let report = checker.check_all();
    let json = report.to_json();
    
    assert!(json["status"].is_string());
    assert!(json["checks"].is_array());
}

// ============= GraphQL Tests =============

#[test]
fn test_graphql_resolver_basic() {
    let resolver = GraphQLResolver::new();
    
    resolver.resolver("hello", |_| {
        Ok(serde_json::json!("Hello, GraphQL!"))
    });
    
    let response = resolver.execute("{ hello }", std::collections::HashMap::new()).unwrap();
    
    assert!(response.data.is_some());
    assert!(response.errors.is_none());
    
    let data = response.data.unwrap();
    assert_eq!(data["hello"], "Hello, GraphQL!");
}

#[test]
fn test_graphql_missing_field_error() {
    let resolver = GraphQLResolver::new();
    
    let response = resolver.execute("{ nonexistent }", std::collections::HashMap::new()).unwrap();
    
    assert!(response.errors.is_some());
    let errors = response.errors.unwrap();
    assert!(errors[0].message.contains("Cannot query field"));
}

#[test]
fn test_graphql_schema_builder() {
    let schema = SchemaBuilder::new()
        .query_type(noor::core::graphql::GraphQLType_ {
            name: "Query".to_string(),
            fields: vec![
                FieldBuilder::new("user", GraphQLType::Custom("User".to_string()))
                    .argument("id", GraphQLType::ID, true)
                    .build(),
                FieldBuilder::new("posts", GraphQLType::List(Box::new(GraphQLType::Custom("Post".to_string()))))
                    .build(),
            ],
            description: None,
        })
        .build();
    
    assert!(schema.query_type.is_some());
    let query = schema.query_type.unwrap();
    assert_eq!(query.fields.len(), 2);
}

#[test]
fn test_graphql_type_to_str() {
    assert_eq!(GraphQLType::String.to_str(), "String");
    assert_eq!(GraphQLType::Int.to_str(), "Int");
    assert_eq!(GraphQLType::Float.to_str(), "Float");
    assert_eq!(GraphQLType::Boolean.to_str(), "Boolean");
    assert_eq!(GraphQLType::ID.to_str(), "ID");
    assert_eq!(
        GraphQLType::List(Box::new(GraphQLType::String)).to_str(),
        "[String]"
    );
    assert_eq!(
        GraphQLType::Custom("User".to_string()).to_str(),
        "User"
    );
}
