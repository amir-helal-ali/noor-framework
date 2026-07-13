// ============================================================
// Logging Drivers - مزودات التسجيل
// ============================================================
// Multiple logging backends: File, Stdout, Syslog, Memory
// Supports log levels, formatting, and rotation.
//
// مزودات متعددة للتسجيل: ملف، stdout، syslog، ذاكرة
// ============================================================

use std::collections::HashMap;
use std::fs::{OpenOptions, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::Mutex;
use serde::{Serialize, Deserialize};

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
            Self::Fatal => "FATAL",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "TRACE" => Some(Self::Trace),
            "DEBUG" => Some(Self::Debug),
            "INFO" => Some(Self::Info),
            "WARN" | "WARNING" => Some(Self::Warn),
            "ERROR" => Some(Self::Error),
            "FATAL" => Some(Self::Fatal),
            _ => None,
        }
    }
    
    pub fn color_code(&self) -> &'static str {
        match self {
            Self::Trace => "\x1b[37m",   // White
            Self::Debug => "\x1b[36m",   // Cyan
            Self::Info => "\x1b[32m",    // Green
            Self::Warn => "\x1b[33m",    // Yellow
            Self::Error => "\x1b[31m",   // Red
            Self::Fatal => "\x1b[35m",   // Magenta
        }
    }
}

/// Log record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRecord {
    pub level: LogLevel,
    pub message: String,
    pub timestamp: i64,
    pub target: Option<String>,
    pub fields: HashMap<String, String>,
}

impl LogRecord {
    pub fn new(level: LogLevel, message: &str) -> Self {
        Self {
            level,
            message: message.to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            target: None,
            fields: HashMap::new(),
        }
    }
    
    pub fn with_target(mut self, target: &str) -> Self {
        self.target = Some(target.to_string());
        self
    }
    
    pub fn with_field(mut self, key: &str, value: &str) -> Self {
        self.fields.insert(key.to_string(), value.to_string());
        self
    }
    
    /// Format as plain text
    pub fn format_text(&self) -> String {
        let datetime = chrono::DateTime::from_timestamp_millis(self.timestamp)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S%.3f").to_string())
            .unwrap_or_else(|| self.timestamp.to_string());
        
        let target = self.target.as_deref().unwrap_or("-");
        
        let fields = if self.fields.is_empty() {
            String::new()
        } else {
            let fields: Vec<String> = self.fields
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            format!(" [{}]", fields.join(", "))
        };
        
        format!("[{}] {} {} {}{}\n", datetime, self.level.as_str(), target, self.message, fields)
    }
    
    /// Format as colored text (for terminal)
    pub fn format_colored(&self) -> String {
        let reset = "\x1b[0m";
        let color = self.level.color_code();
        
        format!("{}{}{}", color, self.format_text(), reset)
    }
    
    /// Format as JSON
    pub fn format_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Log driver trait
pub trait LogDriver: Send + Sync {
    fn log(&self, record: &LogRecord);
    fn name(&self) -> &str;
    fn flush(&self) {}
}

/// Console (stdout) log driver
pub struct ConsoleDriver {
    min_level: LogLevel,
    colored: bool,
}

impl ConsoleDriver {
    pub fn new(min_level: LogLevel) -> Self {
        Self {
            min_level,
            colored: true,
        }
    }
    
    pub fn colored(mut self, colored: bool) -> Self {
        self.colored = colored;
        self
    }
}

impl LogDriver for ConsoleDriver {
    fn log(&self, record: &LogRecord) {
        if record.level < self.min_level {
            return;
        }
        
        let output = if self.colored {
            record.format_colored()
        } else {
            record.format_text()
        };
        
        print!("{}", output);
    }
    
    fn name(&self) -> &str {
        "console"
    }
    
    fn flush(&self) {
        let _ = std::io::stdout().flush();
    }
}

/// File log driver with rotation
pub struct FileDriver {
    file_path: PathBuf,
    file: Mutex<File>,
    min_level: LogLevel,
    max_size: u64,
    rotate_count: u32,
}

impl FileDriver {
    pub fn new(file_path: &str, min_level: LogLevel) -> crate::NoorResult<Self> {
        let path = PathBuf::from(file_path);
        
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        
        Ok(Self {
            file_path: path,
            file: Mutex::new(file),
            min_level,
            max_size: 10 * 1024 * 1024, // 10MB default
            rotate_count: 5,
        })
    }
    
    /// Set max file size before rotation
    pub fn max_size(mut self, size: u64) -> Self {
        self.max_size = size;
        self
    }
    
    /// Set number of rotated files to keep
    pub fn rotate_count(mut self, count: u32) -> Self {
        self.rotate_count = count;
        self
    }
    
    /// Check if rotation is needed and perform it
    fn maybe_rotate(&self) {
        let file = self.file.lock();
        
        if let Ok(metadata) = file.metadata() {
            if metadata.len() >= self.max_size {
                drop(file);
                self.rotate();
            }
        }
    }
    
    /// Perform log rotation
    fn rotate(&self) {
        // Close current file
        // Rename: app.log -> app.log.1, app.log.1 -> app.log.2, etc.
        for i in (1..self.rotate_count).rev() {
            let from = self.file_path.with_extension(format!("log.{}", i));
            let to = self.file_path.with_extension(format!("log.{}", i + 1));
            
            if from.exists() {
                let _ = std::fs::rename(&from, &to);
            }
        }
        
        // Rename current file to .1
        let rotated = self.file_path.with_extension("log.1");
        let _ = std::fs::rename(&self.file_path, &rotated);
        
        // Open new file
        if let Ok(new_file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)
        {
            *self.file.lock() = new_file;
        }
    }
}

impl LogDriver for FileDriver {
    fn log(&self, record: &LogRecord) {
        if record.level < self.min_level {
            return;
        }
        
        let output = record.format_text();
        
        let mut file = self.file.lock();
        
        if let Err(e) = file.write_all(output.as_bytes()) {
            eprintln!("Failed to write to log file: {}", e);
        }
        
        drop(file);
        
        // Check if rotation is needed
        self.maybe_rotate();
    }
    
    fn name(&self) -> &str {
        "file"
    }
    
    fn flush(&self) {
        let _ = self.file.lock().flush();
    }
}

/// Memory log driver (for testing)
pub struct MemoryDriver {
    logs: Arc<Mutex<Vec<LogRecord>>>,
    min_level: LogLevel,
}

impl MemoryDriver {
    pub fn new(min_level: LogLevel) -> Self {
        Self {
            logs: Arc::new(Mutex::new(Vec::new())),
            min_level,
        }
    }
    
    /// Get all logged records
    pub fn records(&self) -> Vec<LogRecord> {
        self.logs.lock().clone()
    }
    
    /// Get records of a specific level
    pub fn records_at_level(&self, level: LogLevel) -> Vec<LogRecord> {
        self.logs
            .lock()
            .iter()
            .filter(|r| r.level == level)
            .cloned()
            .collect()
    }
    
    /// Clear all records
    pub fn clear(&self) {
        self.logs.lock().clear();
    }
    
    /// Get the count of records
    pub fn count(&self) -> usize {
        self.logs.lock().len()
    }
    
    /// Check if any record matches a message
    pub fn contains(&self, message: &str) -> bool {
        self.logs.lock().iter().any(|r| r.message.contains(message))
    }
}

impl LogDriver for MemoryDriver {
    fn log(&self, record: &LogRecord) {
        if record.level < self.min_level {
            return;
        }
        
        self.logs.lock().push(record.clone());
    }
    
    fn name(&self) -> &str {
        "memory"
    }
}

/// Multi-driver logger
pub struct Logger {
    drivers: Vec<Arc<dyn LogDriver>>,
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}

impl Logger {
    pub fn new() -> Self {
        Self {
            drivers: Vec::new(),
        }
    }
    
    /// Add a log driver
    pub fn add_driver(mut self, driver: Arc<dyn LogDriver>) -> Self {
        self.drivers.push(driver);
        self
    }
    
    /// Log a message
    pub fn log(&self, level: LogLevel, message: &str) {
        let record = LogRecord::new(level, message);
        
        for driver in &self.drivers {
            driver.log(&record);
        }
    }
    
    /// Log with fields
    pub fn log_with_fields(&self, level: LogLevel, message: &str, fields: HashMap<String, String>) {
        let mut record = LogRecord::new(level, message);
        record.fields = fields;
        
        for driver in &self.drivers {
            driver.log(&record);
        }
    }
    
    pub fn trace(&self, message: &str) {
        self.log(LogLevel::Trace, message);
    }
    
    pub fn debug(&self, message: &str) {
        self.log(LogLevel::Debug, message);
    }
    
    pub fn info(&self, message: &str) {
        self.log(LogLevel::Info, message);
    }
    
    pub fn warn(&self, message: &str) {
        self.log(LogLevel::Warn, message);
    }
    
    pub fn error(&self, message: &str) {
        self.log(LogLevel::Error, message);
    }
    
    pub fn fatal(&self, message: &str) {
        self.log(LogLevel::Fatal, message);
    }
    
    /// Flush all drivers
    pub fn flush(&self) {
        for driver in &self.drivers {
            driver.flush();
        }
    }
    
    /// Get the count of drivers
    pub fn driver_count(&self) -> usize {
        self.drivers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_log_record_format() {
        let record = LogRecord::new(LogLevel::Info, "Test message")
            .with_target("test_module")
            .with_field("user_id", "123");
        
        let text = record.format_text();
        
        assert!(text.contains("INFO"));
        assert!(text.contains("Test message"));
        assert!(text.contains("test_module"));
        assert!(text.contains("user_id=123"));
    }
    
    #[test]
    fn test_memory_driver() {
        let driver = MemoryDriver::new(LogLevel::Debug);
        
        driver.log(&LogRecord::new(LogLevel::Info, "Test info"));
        driver.log(&LogRecord::new(LogLevel::Error, "Test error"));
        driver.log(&LogRecord::new(LogLevel::Trace, "Test trace")); // Below min level
        
        let records = driver.records();
        
        assert_eq!(records.len(), 2); // Trace should be filtered
        assert_eq!(driver.count(), 2);
        assert!(driver.contains("Test info"));
        assert!(driver.contains("Test error"));
        assert!(!driver.contains("Test trace"));
    }
    
    #[test]
    fn test_multi_driver_logger() {
        let memory = Arc::new(MemoryDriver::new(LogLevel::Debug));
        
        let logger = Logger::new()
            .add_driver(memory.clone());
        
        logger.info("Info message");
        logger.warn("Warning message");
        logger.error("Error message");
        
        assert_eq!(memory.count(), 3);
    }
    
    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Error > LogLevel::Warn);
        assert!(LogLevel::Warn > LogLevel::Info);
        assert!(LogLevel::Info > LogLevel::Debug);
        assert!(LogLevel::Debug > LogLevel::Trace);
    }
    
    #[test]
    fn test_log_level_from_str() {
        assert_eq!(LogLevel::from_str("INFO"), Some(LogLevel::Info));
        assert_eq!(LogLevel::from_str("debug"), Some(LogLevel::Debug));
        assert_eq!(LogLevel::from_str("WARNING"), Some(LogLevel::Warn));
        assert_eq!(LogLevel::from_str("invalid"), None);
    }
}
