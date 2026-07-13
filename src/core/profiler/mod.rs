// ============================================================
// محلل الأداء | Performance Profiler
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// قسم محلل
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSection {
    pub name: String,
    pub start: i64,
    pub duration_ms: f64,
    pub memory_before: u64,
    pub memory_after: u64,
    pub metadata: HashMap<String, String>,
}

/// ملف الأداء
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub sections: Vec<ProfileSection>,
    pub total_duration_ms: f64,
    pub created_at: i64,
}

/// محلل الأداء
pub struct Profiler {
    profiles: Arc<RwLock<Vec<Profile>>>,
    current_sections: Arc<RwLock<HashMap<String, ProfileSection>>>,
    enabled: Arc<RwLock<bool>>,
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new()
    }
}

impl Profiler {
    pub fn new() -> Self {
        Self {
            profiles: Arc::new(RwLock::new(Vec::new())),
            current_sections: Arc::new(RwLock::new(HashMap::new())),
            enabled: Arc::new(RwLock::new(false)),
        }
    }
    
    /// تفعيل/تعطيل المحلل
    pub fn set_enabled(&self, enabled: bool) {
        *self.enabled.write() = enabled;
    }
    
    /// التحقق من تفعيل المحلل
    pub fn is_enabled(&self) -> bool {
        *self.enabled.read()
    }
    
    /// بدء قسم
    pub fn start(&self, name: &str) {
        if !self.is_enabled() {
            return;
        }
        
        let section = ProfileSection {
            name: name.to_string(),
            start: chrono::Utc::now().timestamp_millis(),
            duration_ms: 0.0,
            memory_before: Self::get_memory_usage(),
            memory_after: 0,
            metadata: HashMap::new(),
        };
        
        self.current_sections.write().insert(name.to_string(), section);
    }
    
    /// إنهاء قسم
    pub fn end(&self, name: &str) -> Option<f64> {
        if !self.is_enabled() {
            return None;
        }
        
        let mut sections = self.current_sections.write();
        
        if let Some(mut section) = sections.remove(name) {
            section.duration_ms = (chrono::Utc::now().timestamp_millis() - section.start) as f64;
            section.memory_after = Self::get_memory_usage();
            
            let duration = section.duration_ms;
            
            // إضافة للملف الحالي
            if let Some(last_profile) = self.profiles.write().last_mut() {
                last_profile.sections.push(section);
                last_profile.total_duration_ms += duration;
            }
            
            return Some(duration);
        }
        
        None
    }
    
    /// قياس دالة
    pub fn measure<F, T>(&self, name: &str, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        self.start(name);
        let result = f();
        self.end(name);
        result
    }
    
    /// بدء ملف أداء جديد
    pub fn start_profile(&self, name: &str) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        
        let profile = Profile {
            id: id.clone(),
            name: name.to_string(),
            sections: vec![],
            total_duration_ms: 0.0,
            created_at: chrono::Utc::now().timestamp(),
        };
        
        self.profiles.write().push(profile);
        id
    }
    
    /// إنهاء ملف الأداء الحالي
    ///
    /// Accepts either the profile `id` returned by `start_profile` **or** the
    /// human-readable `name` that was passed to `start_profile`. This makes
    /// the API usable without forcing callers to thread an opaque UUID
    /// through their code.
    ///
    /// Returns a clone of the matched profile. The profile is **not** removed
    /// from the profiler's internal list, so `summary()` and `get_profiles()`
    /// still report it. Use `clear()` to reset the list.
    pub fn end_profile(&self, profile_id_or_name: &str) -> Option<Profile> {
        let profiles = self.profiles.read();

        // Try by id first, then fall back to name.
        if let Some(p) = profiles.iter().find(|p| p.id == profile_id_or_name) {
            return Some(p.clone());
        }
        if let Some(p) = profiles.iter().find(|p| p.name == profile_id_or_name) {
            return Some(p.clone());
        }

        None
    }
    
    /// الحصول على جميع ملفات الأداء
    pub fn get_profiles(&self) -> Vec<Profile> {
        self.profiles.read().clone()
    }
    
    /// الحصول على ملف أداء محدد
    pub fn get_profile(&self, profile_id: &str) -> Option<Profile> {
        self.profiles.read().iter().find(|p| p.id == profile_id).cloned()
    }
    
    /// مسح جميع ملفات الأداء
    pub fn clear(&self) {
        self.profiles.write().clear();
        self.current_sections.write().clear();
    }
    
    /// الحصول على استخدام الذاكرة (محاكاة)
    fn get_memory_usage() -> u64 {
        // في تطبيق حقيقي، سنستخدم syscall أو مكتبة
        // للوصول لمعلومات الذاكرة
        0
    }
    
    /// تصدير ملفات الأداء بصيغة JSON
    pub fn export_json(&self) -> serde_json::Value {
        serde_json::to_value(self.profiles.read().clone())
            .unwrap_or(serde_json::json!({"error": "Export failed"}))
    }
    
    /// الحصول على ملخص الأداء
    pub fn summary(&self) -> ProfileSummary {
        let profiles = self.profiles.read();
        
        let total_profiles = profiles.len();
        let total_sections: usize = profiles.iter().map(|p| p.sections.len()).sum();
        let avg_duration = if profiles.is_empty() {
            0.0
        } else {
            profiles.iter().map(|p| p.total_duration_ms).sum::<f64>() / profiles.len() as f64
        };
        
        let slowest_section = profiles
            .iter()
            .flat_map(|p| &p.sections)
            .max_by(|a, b| a.duration_ms.partial_cmp(&b.duration_ms).unwrap_or(std::cmp::Ordering::Equal))
            .map(|s| (s.name.clone(), s.duration_ms));
        
        ProfileSummary {
            total_profiles,
            total_sections,
            avg_duration_ms: avg_duration,
            slowest_section,
        }
    }
}

/// ملخص الأداء
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSummary {
    pub total_profiles: usize,
    pub total_sections: usize,
    pub avg_duration_ms: f64,
    pub slowest_section: Option<(String, f64)>,
}

/// ماكرو لقياس دالة
#[macro_export]
macro_rules! profile {
    ($profiler:expr, $name:expr, $body:expr) => {{
        $profiler.start($name);
        let result = $body;
        $profiler.end($name);
        result
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_profiler_basic() {
        let profiler = Profiler::new();
        profiler.set_enabled(true);
        
        profiler.start_profile("test_profile");
        
        profiler.start("operation1");
        std::thread::sleep(Duration::from_millis(10));
        profiler.end("operation1");
        
        profiler.start("operation2");
        std::thread::sleep(Duration::from_millis(5));
        profiler.end("operation2");
        
        let profile = profiler.end_profile("test_profile").unwrap();
        
        assert_eq!(profile.name, "test_profile");
        assert_eq!(profile.sections.len(), 2);
        assert!(profile.total_duration_ms > 0.0);
    }
    
    #[test]
    fn test_profiler_measure() {
        let profiler = Profiler::new();
        profiler.set_enabled(true);
        
        profiler.start_profile("test");
        
        let result = profiler.measure("computation", || {
            std::thread::sleep(Duration::from_millis(10));
            42
        });
        
        assert_eq!(result, 42);
    }
    
    #[test]
    fn test_profiler_disabled() {
        let profiler = Profiler::new();
        // المحلل معطل افتراضياً
        
        profiler.start("test");
        profiler.end("test");
        
        // لا ينبغي تسجيل أي شيء
        assert_eq!(profiler.get_profiles().len(), 0);
    }
    
    #[test]
    fn test_profiler_summary() {
        let profiler = Profiler::new();
        profiler.set_enabled(true);
        
        profiler.start_profile("profile1");
        profiler.measure("fast", || std::thread::sleep(Duration::from_millis(5)));
        profiler.measure("slow", || std::thread::sleep(Duration::from_millis(20)));
        profiler.end_profile("profile1");
        
        let summary = profiler.summary();
        
        assert_eq!(summary.total_profiles, 1);
        assert_eq!(summary.total_sections, 2);
        assert!(summary.slowest_section.is_some());
        assert_eq!(summary.slowest_section.unwrap().0, "slow");
    }
}
