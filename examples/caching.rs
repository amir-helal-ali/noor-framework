// ============================================================
// Example: Caching Strategies
// مثال: استراتيجيات التخزين المؤقت
// ============================================================

use noor::*;
use noor::core::cache::{CacheManager, Cache};
use std::time::Duration;

fn main() -> NoorResult<()> {
    println!("{}", banner());
    println!("\n📦 Caching Strategies Demo\n");
    
    // 1. Weak server mode (file-based)
    let cache = CacheManager::for_weak_server("storage/cache")?;
    
    // Basic set/get
    cache.set("user:1", b"John Doe", 3600)?;
    
    if let Some(value) = cache.get("user:1") {
        println!("✓ Cache hit: {}", String::from_utf8(value).map_err(|e| NoorError::Internal(e.to_string()))?);
    }
    
    // Cache-aside pattern with remember
    let user_name: String = cache.remember("user:2", 3600, || {
        println!("  (Computing value...)");
        Ok("Jane Doe".to_string())
    })?;
    println!("✓ Remember pattern: {}", user_name);
    
    // Second call should use cache
    let cached: String = cache.remember("user:2", 3600, || {
        println!("  (This should NOT print - using cache)");
        Ok("Should not compute".to_string())
    })?;
    println!("✓ Cached value: {}", cached);
    
    // JSON caching
    let user = serde_json::json!({
        "id": 1,
        "name": "John",
        "email": "john@example.com"
    });
    cache.set_json("user:json:1", &user, 3600)?;
    
    if let Some(cached_user) = cache.get_json::<serde_json::Value>("user:json:1") {
        println!("✓ JSON cache: {}", cached_user["name"]);
    }
    
    println!("\n✓ Caching demo completed!");
    Ok(())
}
