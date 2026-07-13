// ============================================================
// Cache System - نظام التخزين المؤقت
// ============================================================
// File-based cache for weak servers (no Redis required).
// Memory cache for high-performance scenarios.
// Automatic fallback from memory to file.
//
// تخزين مؤقت ملفي للسيرفرات الضعيفة.
// ============================================================

pub mod file;
pub mod memory;
pub mod manager;

pub use file::FileCache;
pub use memory::MemoryCache;
pub use manager::CacheManager;

/// Cache trait that all cache drivers implement
/// Trait يطبقه جميع drivers الـ cache
pub trait Cache: Send + Sync {
    /// Get a value from the cache
    /// الحصول على قيمة من الـ cache
    fn get(&self, key: &str) -> Option<Vec<u8>>;
    
    /// Set a value in the cache with TTL (seconds)
    /// تعيين قيمة في الـ cache مع TTL
    fn set(&self, key: &str, value: &[u8], ttl_secs: u64) -> crate::NoorResult<()>;
    
    /// Delete a value from the cache
    /// حذف قيمة من الـ cache
    fn delete(&self, key: &str) -> crate::NoorResult<()>;
    
    /// Check if a key exists
    /// فحص إذا كان المفتاح موجوداً
    fn has(&self, key: &str) -> bool {
        self.get(key).is_some()
    }
    
    /// Clear all cached values
    /// مسح جميع القيم
    fn clear(&self) -> crate::NoorResult<()>;
    
    /// Get the cache driver name
    /// الحصول على اسم driver الـ cache
    fn driver_name(&self) -> &str;
}
