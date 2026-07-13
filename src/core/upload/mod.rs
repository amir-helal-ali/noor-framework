// ============================================================
// File Upload Handler - معالج رفع الملفات
// ============================================================

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadConfig {
    pub max_size: usize,
    pub upload_dir: String,
    pub allowed_extensions: Vec<String>,
    pub forbidden_extensions: Vec<String>,
    pub keep_original_name: bool,
    pub overwrite: bool,
}

impl Default for UploadConfig {
    fn default() -> Self {
        Self {
            max_size: 10 * 1024 * 1024,
            upload_dir: "storage/uploads".to_string(),
            allowed_extensions: vec![
                "jpg".into(), "jpeg".into(), "png".into(), "gif".into(), "webp".into(),
                "pdf".into(), "txt".into(), "csv".into(), "json".into(),
                "zip".into(), "tar".into(), "gz".into(),
                "doc".into(), "docx".into(), "xls".into(), "xlsx".into(),
            ],
            forbidden_extensions: vec![
                "exe".into(), "bat".into(), "sh".into(), "cmd".into(),
                "php".into(), "js".into(), "html".into(), "htm".into(),
                "svg".into(),
            ],
            keep_original_name: false,
            overwrite: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadedFile {
    pub original_name: String,
    pub stored_name: String,
    pub path: String,
    pub size: usize,
    pub mime_type: String,
    pub extension: String,
}

#[derive(Debug)]
pub enum UploadError {
    TooLarge { max: usize, actual: usize },
    ForbiddenExtension(String),
    NotAllowedExtension(String),
    InvalidFileName,
    Io(std::io::Error),
}

impl std::fmt::Display for UploadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge { max, actual } => write!(f, "file too large: {} bytes exceeds {} byte limit", actual, max),
            Self::ForbiddenExtension(ext) => write!(f, "forbidden file extension: .{}", ext),
            Self::NotAllowedExtension(ext) => write!(f, "extension .{} not in allow-list", ext),
            Self::InvalidFileName => write!(f, "invalid file name"),
            Self::Io(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for UploadError {}
impl From<std::io::Error> for UploadError { fn from(e: std::io::Error) -> Self { Self::Io(e) } }
impl From<UploadError> for crate::NoorError { fn from(e: UploadError) -> Self { crate::NoorError::Internal(format!("Upload error: {}", e)) } }

pub struct FileUploader { config: UploadConfig }

impl FileUploader {
    pub fn new(mut config: UploadConfig) -> Result<Self, UploadError> {
        std::fs::create_dir_all(&config.upload_dir)?;
        config.allowed_extensions = config.allowed_extensions.iter().map(|s| s.to_lowercase()).collect();
        config.forbidden_extensions = config.forbidden_extensions.iter().map(|s| s.to_lowercase()).collect();
        Ok(Self { config })
    }

    pub fn set_config(&mut self, config: UploadConfig) { self.config = config; }

    pub fn validate(&self, name: &str, _mime_type: &str, size: usize) -> Result<(), UploadError> {
        if size > self.config.max_size {
            return Err(UploadError::TooLarge { max: self.config.max_size, actual: size });
        }
        let ext = extract_extension(name).ok_or(UploadError::InvalidFileName)?;
        let ext_lower = ext.to_lowercase();
        if self.config.forbidden_extensions.iter().any(|e| e == &ext_lower) {
            return Err(UploadError::ForbiddenExtension(ext_lower));
        }
        if !self.config.allowed_extensions.is_empty() && !self.config.allowed_extensions.iter().any(|e| e == &ext_lower) {
            return Err(UploadError::NotAllowedExtension(ext_lower));
        }
        Ok(())
    }

    pub fn store(&self, name: &str, content: &[u8]) -> Result<UploadedFile, UploadError> {
        self.validate(name, "", content.len())?;
        let ext = extract_extension(name).unwrap_or_default();
        let ext_lower = ext.to_lowercase();
        let stored_name = if self.config.keep_original_name {
            sanitize_filename(name)
        } else {
            let id = uuid::Uuid::new_v4().to_string();
            if ext_lower.is_empty() { id } else { format!("{}.{}", id, ext_lower) }
        };
        let path = PathBuf::from(&self.config.upload_dir).join(&stored_name);
        if path.exists() && !self.config.overwrite {
            let stem = Path::new(&stored_name).file_stem().and_then(|s| s.to_str()).unwrap_or("file");
            let mut counter = 1u32;
            let final_name = loop {
                let candidate = if ext_lower.is_empty() { format!("{}_{}", stem, counter) } else { format!("{}_{}.{}", stem, counter, ext_lower) };
                if !PathBuf::from(&self.config.upload_dir).join(&candidate).exists() { break candidate; }
                counter += 1;
            };
            let final_path = PathBuf::from(&self.config.upload_dir).join(&final_name);
            std::fs::write(&final_path, content)?;
            return Ok(UploadedFile {
                original_name: name.to_string(), stored_name: final_name,
                path: final_path.to_string_lossy().to_string(), size: content.len(),
                mime_type: guess_mime(&ext_lower), extension: ext_lower,
            });
        }
        std::fs::write(&path, content)?;
        Ok(UploadedFile {
            original_name: name.to_string(), stored_name,
            path: path.to_string_lossy().to_string(), size: content.len(),
            mime_type: guess_mime(&ext_lower), extension: ext_lower,
        })
    }

    pub fn delete(&self, stored_name: &str) -> Result<bool, UploadError> {
        let safe = sanitize_filename(stored_name);
        if safe != stored_name { return Err(UploadError::InvalidFileName); }
        let path = PathBuf::from(&self.config.upload_dir).join(stored_name);
        if !path.exists() { return Ok(false); }
        std::fs::remove_file(&path)?;
        Ok(true)
    }

    pub fn format_size(bytes: u64) -> String {
        const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
        if bytes == 0 { return "0 B".to_string(); }
        let mut size = bytes as f64;
        let mut unit_idx = 0;
        while size >= 1024.0 && unit_idx < UNITS.len() - 1 { size /= 1024.0; unit_idx += 1; }
        if unit_idx == 0 { format!("{} {}", bytes, UNITS[0]) } else { format!("{:.2} {}", size, UNITS[unit_idx]) }
    }
}

fn extract_extension(name: &str) -> Option<String> {
    Path::new(name).extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase())
}

fn sanitize_filename(name: &str) -> String {
    let mut s: String = name.chars().map(|c| match c { '/' | '\\' | '\0' => '_', _ => c }).collect();
    while s.starts_with('.') { s.remove(0); }
    if s.is_empty() { s = "unnamed".to_string(); }
    if s.len() > 200 { s.truncate(200); }
    s
}

fn guess_mime(ext: &str) -> String {
    match ext {
        "jpg" | "jpeg" => "image/jpeg", "png" => "image/png", "gif" => "image/gif",
        "webp" => "image/webp", "pdf" => "application/pdf", "txt" => "text/plain",
        "csv" => "text/csv", "json" => "application/json", "zip" => "application/zip",
        "tar" => "application/x-tar", "gz" => "application/gzip",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        _ => "application/octet-stream",
    }.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_uploader(dir: &str) -> FileUploader {
        let mut cfg = UploadConfig::default();
        cfg.upload_dir = dir.to_string();
        cfg.keep_original_name = false;
        FileUploader::new(cfg).unwrap()
    }

    #[test]
    fn test_validate_allows_pdf() {
        let uploader = test_uploader("/tmp/noor_uploads_test1");
        assert!(uploader.validate("doc.pdf", "application/pdf", 1024).is_ok());
    }

    #[test]
    fn test_validate_rejects_exe() {
        let uploader = test_uploader("/tmp/noor_uploads_test2");
        assert!(matches!(uploader.validate("evil.exe", "application/octet-stream", 1024), Err(UploadError::ForbiddenExtension(_))));
    }

    #[test]
    fn test_validate_rejects_oversize() {
        let mut cfg = UploadConfig::default();
        cfg.max_size = 100;
        cfg.upload_dir = "/tmp/noor_uploads_test3".into();
        let uploader = FileUploader::new(cfg).unwrap();
        assert!(matches!(uploader.validate("big.txt", "text/plain", 200), Err(UploadError::TooLarge { .. })));
    }

    #[test]
    fn test_store_writes_file() {
        let dir = "/tmp/noor_uploads_test4";
        std::fs::remove_dir_all(dir).ok();
        let uploader = test_uploader(dir);
        let uploaded = uploader.store("hello.txt", b"hi there").unwrap();
        assert_eq!(uploaded.size, 8);
        assert_eq!(uploaded.extension, "txt");
        assert!(uploaded.path.ends_with(".txt"));
        assert!(Path::new(&uploaded.path).exists());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn test_format_size() {
        assert_eq!(FileUploader::format_size(500), "500 B");
        assert_eq!(FileUploader::format_size(1024), "1.00 KB");
        assert_eq!(FileUploader::format_size(1048576), "1.00 MB");
    }
}
