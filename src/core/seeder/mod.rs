// ============================================================
// Seeder Module - وحدة تعبئة البيانات
// ============================================================
// Database seeding for development and testing.
// Provides a structured way to populate initial data.
//
// تعبئة قاعدة البيانات للتطوير والاختبار.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// Seeder type
/// نوع المعبئ
pub type SeederFn = Arc<dyn Fn() -> crate::NoorResult<()> + Send + Sync>;

/// A database seeder
/// معبئ قاعدة البيانات
pub struct Seeder {
    seeders: Arc<RwLock<Vec<SeederEntry>>>,
    /// Whether seeding has been run
    seeded: Arc<RwLock<bool>>,
}

struct SeederEntry {
    name: String,
    description: String,
    handler: SeederFn,
    enabled: bool,
    order: u32,
}

impl Default for Seeder {
    fn default() -> Self {
        Self::new()
    }
}

impl Seeder {
    pub fn new() -> Self {
        Self {
            seeders: Arc::new(RwLock::new(Vec::new())),
            seeded: Arc::new(RwLock::new(false)),
        }
    }
    
    /// Register a seeder
    /// تسجيل معبئ
    pub fn register(&self, name: &str, description: &str, handler: SeederFn) -> &Self {
        self.seeders.write().push(SeederEntry {
            name: name.to_string(),
            description: description.to_string(),
            handler,
            enabled: true,
            order: 100,
        });
        self
    }
    
    /// Register a seeder with specific order
    pub fn register_ordered(&self, name: &str, description: &str, order: u32, handler: SeederFn) -> &Self {
        self.seeders.write().push(SeederEntry {
            name: name.to_string(),
            description: description.to_string(),
            handler,
            enabled: true,
            order,
        });
        // Sort by order
        self.seeders.write().sort_by_key(|s| s.order);
        self
    }
    
    /// Disable a seeder
    pub fn disable(&self, name: &str) {
        let mut seeders = self.seeders.write();
        if let Some(s) = seeders.iter_mut().find(|s| s.name == name) {
            s.enabled = false;
        }
    }
    
    /// Enable a seeder
    pub fn enable(&self, name: &str) {
        let mut seeders = self.seeders.write();
        if let Some(s) = seeders.iter_mut().find(|s| s.name == name) {
            s.enabled = true;
        }
    }
    
    /// Run all registered seeders
    /// تشغيل جميع المعبئات المسجلة
    pub fn run(&self) -> crate::NoorResult<()> {
        if *self.seeded.read() {
            tracing::warn!("Seeders have already been run");
            return Ok(());
        }
        
        let seeders = self.seeders.read();
        
        for seeder in seeders.iter() {
            if !seeder.enabled {
                continue;
            }
            
            tracing::info!("Running seeder: {} - {}", seeder.name, seeder.description);
            
            if let Err(e) = (seeder.handler)() {
                tracing::error!("Seeder '{}' failed: {}", seeder.name, e);
                return Err(e);
            }
            
            tracing::info!("Seeder '{}' completed", seeder.name);
        }
        
        *self.seeded.write() = true;
        Ok(())
    }
    
    /// Run a specific seeder by name
    pub fn run_one(&self, name: &str) -> crate::NoorResult<()> {
        let seeders = self.seeders.read();
        
        for seeder in seeders.iter() {
            if seeder.name == name {
                if !seeder.enabled {
                    return Err(crate::NoorError::Internal(
                        format!("Seeder '{}' is disabled", name)
                    ));
                }
                return (seeder.handler)();
            }
        }
        
        Err(crate::NoorError::Internal(
            format!("Seeder '{}' not found", name)
        ))
    }
    
    /// Reset the seeded state
    pub fn reset(&self) {
        *self.seeded.write() = false;
    }
    
    /// List all registered seeders
    pub fn list(&self) -> Vec<(String, String, bool)> {
        self.seeders
            .read()
            .iter()
            .map(|s| (s.name.clone(), s.description.clone(), s.enabled))
            .collect()
    }
    
    /// Check if seeding has been done
    pub fn is_seeded(&self) -> bool {
        *self.seeded.read()
    }
}

/// Data record for seeding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedRecord {
    pub table: String,
    pub data: serde_json::Value,
}

impl SeedRecord {
    pub fn new(table: &str, data: serde_json::Value) -> Self {
        Self {
            table: table.to_string(),
            data,
        }
    }
}

/// Factory for generating fake data
/// مصنع لتوليد بيانات وهمية
pub struct Factory;

impl Factory {
    /// Generate a fake user
    pub fn user(name: &str, email: &str) -> serde_json::Value {
        serde_json::json!({
            "name": name,
            "email": email,
            "password_hash": "$argon2id$example_hash_placeholder",
            "role": "user",
            "created_at": chrono::Utc::now().to_rfc3339(),
            "updated_at": chrono::Utc::now().to_rfc3339(),
        })
    }
    
    /// Generate a fake post
    pub fn post(title: &str, content: &str, author_id: i64) -> serde_json::Value {
        serde_json::json!({
            "title": title,
            "content": content,
            "author_id": author_id,
            "status": "published",
            "views": 0,
            "created_at": chrono::Utc::now().to_rfc3339(),
            "updated_at": chrono::Utc::now().to_rfc3339(),
        })
    }
    
    /// Generate a fake category
    pub fn category(name: &str, description: &str) -> serde_json::Value {
        serde_json::json!({
            "name": name,
            "description": description,
            "slug": name.to_lowercase().replace(" ", "-"),
            "created_at": chrono::Utc::now().to_rfc3339(),
        })
    }
    
    /// Generate multiple records
    pub fn batch(count: usize, generator: impl Fn(usize) -> serde_json::Value) -> Vec<serde_json::Value> {
        (0..count).map(generator).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    #[test]
    fn test_seeder() {
        let seeder = Seeder::new();
        let counter = Arc::new(AtomicUsize::new(0));
        
        let counter_clone = counter.clone();
        seeder.register("test_seeder", "Test seeder", Arc::new(move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }));
        
        seeder.run().unwrap();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
        assert!(seeder.is_seeded());
    }
    
    #[test]
    fn test_factory_user() {
        let user = Factory::user("John Doe", "john@example.com");
        assert_eq!(user["name"], "John Doe");
        assert_eq!(user["email"], "john@example.com");
    }
}
