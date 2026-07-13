// ============================================================
// File Watcher - مراقب الملفات
// ============================================================
// Watches for file changes and triggers callbacks.
// Useful for hot reload during development.
//
// يراقب تغييرات الملفات ويطلق callbacks.
// ============================================================

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use parking_lot::RwLock;
use std::time::SystemTime;

/// File change event
#[derive(Debug, Clone)]
pub struct FileChangeEvent {
    pub path: PathBuf,
    pub event_type: FileEventType,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileEventType {
    Created,
    Modified,
    Deleted,
    Renamed,
}

/// File watcher callback type
type FileCallback = Arc<dyn Fn(&FileChangeEvent) + Send + Sync>;

/// File watcher
pub struct FileWatcher {
    watched_paths: Arc<RwLock<Vec<PathBuf>>>,
    callbacks: Arc<RwLock<HashMap<String, Vec<FileCallback>>>>,
    file_mtimes: Arc<RwLock<HashMap<PathBuf, SystemTime>>>,
    running: Arc<AtomicBool>,
    /// Extensions to watch (empty = all)
    extensions: Arc<RwLock<Vec<String>>>,
    /// Paths to ignore
    ignore_patterns: Arc<RwLock<Vec<String>>>,
}

impl Default for FileWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl FileWatcher {
    pub fn new() -> Self {
        Self {
            watched_paths: Arc::new(RwLock::new(Vec::new())),
            callbacks: Arc::new(RwLock::new(HashMap::new())),
            file_mtimes: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(AtomicBool::new(false)),
            extensions: Arc::new(RwLock::new(Vec::new())),
            ignore_patterns: Arc::new(RwLock::new(vec![
                ".git".to_string(),
                "target".to_string(),
                "node_modules".to_string(),
                ".noor".to_string(),
            ])),
        }
    }
    
    /// Add a path to watch
    ///
    /// Note: we deliberately do NOT pre-scan files here. The first call to
    /// `check_changes()` will report every existing file as `Created`, which
    /// is the expected "initial state" behaviour. Pre-scanning would swallow
    /// those initial `Created` events.
    pub fn watch(&self, path: &str) -> &Self {
        let path = PathBuf::from(path);

        if path.exists() {
            self.watched_paths.write().push(path);
        }

        self
    }
    
    /// Add extension filter
    pub fn extension(&self, ext: &str) -> &Self {
        let ext = ext.trim_start_matches('.');
        self.extensions.write().push(ext.to_lowercase());
        self
    }
    
    /// Add ignore pattern
    pub fn ignore(&self, pattern: &str) -> &Self {
        self.ignore_patterns.write().push(pattern.to_string());
        self
    }
    
    /// Register a callback for file changes
    pub fn on_change<F>(&self, callback: F) -> &Self
    where
        F: Fn(&FileChangeEvent) + Send + Sync + 'static,
    {
        self.callbacks
            .write()
            .entry("change".to_string())
            .or_insert_with(Vec::new)
            .push(Arc::new(callback));
        self
    }
    
    /// Scan all files and record modification times
    fn scan_files(&self) {
        let paths = self.watched_paths.read().clone();
        let mut mtimes = self.file_mtimes.write();
        
        for path in &paths {
            self.scan_directory(path, &mut mtimes);
        }
    }
    
    /// Recursively scan a directory
    fn scan_directory(&self, dir: &Path, mtimes: &mut HashMap<PathBuf, SystemTime>) {
        if !dir.is_dir() {
            return;
        }
        
        // Check if directory should be ignored
        if let Some(name) = dir.file_name().and_then(|n| n.to_str()) {
            if self.ignore_patterns.read().iter().any(|p| name.contains(p)) {
                return;
            }
        }
        
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                
                if path.is_dir() {
                    self.scan_directory(&path, mtimes);
                } else if path.is_file() {
                    if self.should_watch(&path) {
                        if let Ok(metadata) = entry.metadata() {
                            if let Ok(mtime) = metadata.modified() {
                                mtimes.insert(path, mtime);
                            }
                        }
                    }
                }
            }
        }
    }
    
    /// Check if a file should be watched
    fn should_watch(&self, path: &Path) -> bool {
        // Check ignore patterns
        let path_str = path.to_string_lossy();
        let ignore_patterns = self.ignore_patterns.read();
        
        for pattern in ignore_patterns.iter() {
            if path_str.contains(pattern) {
                return false;
            }
        }
        
        // Check extension filter
        let extensions = self.extensions.read();
        
        if extensions.is_empty() {
            return true;
        }
        
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            return extensions.contains(&ext.to_lowercase());
        }
        
        false
    }
    
    /// Check for changes (single pass)
    pub fn check_changes(&self) -> Vec<FileChangeEvent> {
        let mut events = Vec::new();
        let paths = self.watched_paths.read().clone();
        
        let mut current_mtimes: HashMap<PathBuf, SystemTime> = HashMap::new();
        
        for path in &paths {
            self.scan_directory(path, &mut current_mtimes);
        }
        
        let mut stored_mtimes = self.file_mtimes.write();
        
        // Check for new or modified files
        for (path, mtime) in &current_mtimes {
            match stored_mtimes.get(path) {
                Some(stored_mtime) => {
                    if stored_mtime != mtime {
                        events.push(FileChangeEvent {
                            path: path.clone(),
                            event_type: FileEventType::Modified,
                            timestamp: SystemTime::now(),
                        });
                    }
                }
                None => {
                    events.push(FileChangeEvent {
                        path: path.clone(),
                        event_type: FileEventType::Created,
                        timestamp: SystemTime::now(),
                    });
                }
            }
        }
        
        // Check for deleted files
        let deleted_paths: Vec<PathBuf> = stored_mtimes
            .keys()
            .filter(|p| !current_mtimes.contains_key(*p))
            .cloned()
            .collect();
        
        for path in deleted_paths {
            events.push(FileChangeEvent {
                path: path.clone(),
                event_type: FileEventType::Deleted,
                timestamp: SystemTime::now(),
            });
            stored_mtimes.remove(&path);
        }
        
        // Update stored mtimes
        for (path, mtime) in current_mtimes {
            stored_mtimes.insert(path, mtime);
        }
        
        // Fire callbacks
        if !events.is_empty() {
            let callbacks = self.callbacks.read();
            
            if let Some(change_callbacks) = callbacks.get("change") {
                for event in &events {
                    for callback in change_callbacks {
                        callback(event);
                    }
                }
            }
        }
        
        events
    }
    
    /// Start watching (blocking loop)
    pub fn start(&self, poll_interval_ms: u64) {
        self.running.store(true, Ordering::SeqCst);
        
        let running = self.running.clone();
        let watcher = Arc::new(Self::new());
        
        // Copy configuration
        {
            let paths = self.watched_paths.read();
            for path in paths.iter() {
                watcher.watch(path.to_str().unwrap());
            }
        }
        
        {
            let extensions = self.extensions.read();
            for ext in extensions.iter() {
                watcher.extension(ext);
            }
        }
        
        // In a real implementation, this would use notify crate
        // For now, we poll periodically
        while running.load(Ordering::SeqCst) {
            let events = self.check_changes();
            
            if !events.is_empty() {
                let callbacks = self.callbacks.read();
                if let Some(change_callbacks) = callbacks.get("change") {
                    for event in &events {
                        for callback in change_callbacks {
                            callback(event);
                        }
                    }
                }
            }
            
            std::thread::sleep(std::time::Duration::from_millis(poll_interval_ms));
        }
    }
    
    /// Stop watching
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
    
    /// Check if watching is active
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
    
    /// Get the number of watched files
    pub fn watched_count(&self) -> usize {
        self.file_mtimes.read().len()
    }
    
    /// Get all watched paths
    pub fn paths(&self) -> Vec<PathBuf> {
        self.watched_paths.read().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    #[test]
    fn test_file_watcher_creation() {
        let watcher = FileWatcher::new();
        
        assert_eq!(watcher.watched_count(), 0);
        assert!(!watcher.is_running());
    }
    
    #[test]
    fn test_should_watch() {
        let watcher = FileWatcher::new();
        watcher.extension("rs");
        
        assert!(watcher.should_watch(Path::new("src/main.rs")));
        assert!(!watcher.should_watch(Path::new("README.md")));
    }
    
    #[test]
    fn test_ignore_patterns() {
        let watcher = FileWatcher::new();
        
        assert!(!watcher.should_watch(Path::new("target/debug/app")));
        assert!(!watcher.should_watch(Path::new(".git/config")));
    }
    
    #[test]
    fn test_scan_directory() {
        let watcher = FileWatcher::new();
        watcher.extension("rs");
        
        // Create a temp directory
        let temp_dir = "/tmp/noor_watcher_test";
        std::fs::create_dir_all(temp_dir).ok();
        std::fs::write(format!("{}/test.rs", temp_dir), "fn main() {}").ok();
        std::fs::write(format!("{}/test.txt", temp_dir), "hello").ok();
        
        let mut mtimes = HashMap::new();
        let watcher_ref = FileWatcher::new();
        watcher_ref.extension("rs");
        watcher_ref.scan_directory(Path::new(temp_dir), &mut mtimes);
        
        // Should only watch .rs files
        assert!(mtimes.values().len() > 0);
        
        // Clean up
        std::fs::remove_dir_all(temp_dir).ok();
    }
    
    #[test]
    fn test_check_changes() {
        let temp_dir = "/tmp/noor_watcher_test2";
        std::fs::create_dir_all(temp_dir).ok();
        std::fs::write(format!("{}/test.rs", temp_dir), "fn main() {}").ok();
        
        let watcher = FileWatcher::new();
        watcher.extension("rs");
        watcher.watch(temp_dir);
        
        // First check should detect the file as created
        let events = watcher.check_changes();
        assert!(events.iter().any(|e| e.event_type == FileEventType::Created));
        
        // Modify the file
        std::thread::sleep(std::time::Duration::from_millis(100));
        std::fs::write(format!("{}/test.rs", temp_dir), "fn main() { println!(\"hello\"); }").ok();
        
        // Second check should detect modification
        let events = watcher.check_changes();
        assert!(events.iter().any(|e| e.event_type == FileEventType::Modified));
        
        // Delete the file
        std::fs::remove_file(format!("{}/test.rs", temp_dir)).ok();
        
        // Third check should detect deletion
        let events = watcher.check_changes();
        assert!(events.iter().any(|e| e.event_type == FileEventType::Deleted));
        
        // Clean up
        std::fs::remove_dir_all(temp_dir).ok();
    }
}
