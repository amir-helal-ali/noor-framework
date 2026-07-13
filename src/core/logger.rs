// ============================================================
// Logger - المسجل
// ============================================================

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::Mutex;

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }
}

/// Logger with file output and rotation
/// مسجل مع إخراج ملفي وتدوير
pub struct Logger {
    log_file: Option<PathBuf>,
    min_level: LogLevel,
    file_lock: Arc<Mutex<()>>,
}

impl Logger {
    pub fn new(log_file: Option<PathBuf>, min_level: LogLevel) -> Self {
        Self {
            log_file,
            min_level,
            file_lock: Arc::new(Mutex::new(())),
        }
    }
    
    pub fn log(&self, level: LogLevel, message: &str) {
        if level < self.min_level {
            return;
        }
        
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S");
        let log_line = format!("[{} UTC] {} - {}\n", timestamp, level.as_str(), message);
        
        // Print to stderr
        eprint!("{}", log_line);
        
        // Write to file if configured
        if let Some(ref path) = self.log_file {
            let _lock = self.file_lock.lock();
            
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
            {
                let _ = file.write_all(log_line.as_bytes());
            }
        }
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
}
