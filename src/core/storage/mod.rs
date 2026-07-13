// ============================================================
// File Storage Abstraction - تجريد تخزين الملفات
// ============================================================
// Unified file storage interface supporting multiple backends:
// - Local filesystem
// - S3-compatible (AWS S3, MinIO, DigitalOcean Spaces)
// - FTP/FTPS (planned)
//
// واجهة موحدة لتخزين الملفات مع دعم متعدد الخلفيات.
// ============================================================

use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};
use async_trait::async_trait;

/// File metadata
/// معلومات الملف
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub path: String,
    pub size: u64,
    pub content_type: String,
    pub last_modified: i64,
    pub etag: String,
    pub exists: bool,
}

/// Storage visibility
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Private,
}

/// Storage trait that all backends must implement
/// Trait يجب على جميع خلفيات التخزين تطبيقه
#[async_trait]
pub trait Storage: Send + Sync {
    /// Read file content as bytes
    async fn read(&self, path: &str) -> crate::NoorResult<Vec<u8>>;
    
    /// Write content to a file
    async fn write(&self, path: &str, content: &[u8], visibility: Visibility) -> crate::NoorResult<()>;
    
    /// Delete a file
    async fn delete(&self, path: &str) -> crate::NoorResult<bool>;
    
    /// Check if file exists
    async fn exists(&self, path: &str) -> crate::NoorResult<bool>;
    
    /// Get file metadata
    async fn metadata(&self, path: &str) -> crate::NoorResult<FileMetadata>;
    
    /// List files in a directory
    async fn list(&self, directory: &str) -> crate::NoorResult<Vec<String>>;
    
    /// Copy a file
    async fn copy(&self, from: &str, to: &str) -> crate::NoorResult<()>;
    
    /// Move/rename a file
    async fn move_file(&self, from: &str, to: &str) -> crate::NoorResult<()>;
    
    /// Get a public URL for the file (if public)
    async fn url(&self, path: &str) -> crate::NoorResult<String>;
    
    /// Get a temporary download URL (for private files)
    async fn temporary_url(&self, path: &str, expires_secs: u64) -> crate::NoorResult<String>;
    
    /// Get the storage driver name
    fn driver_name(&self) -> &str;
}

/// Local filesystem storage
/// تخزين نظام الملفات المحلي
pub struct LocalStorage {
    root: PathBuf,
    public_url_prefix: String,
}

impl LocalStorage {
    pub fn new(root: &str, public_url_prefix: &str) -> crate::NoorResult<Self> {
        let root = PathBuf::from(root);
        std::fs::create_dir_all(&root)?;
        
        Ok(Self {
            root,
            public_url_prefix: public_url_prefix.to_string(),
        })
    }
    
    fn resolve_path(&self, path: &str) -> PathBuf {
        // Prevent directory traversal attacks
        let safe_path = path.replace("..", "").replace("//", "/");
        self.root.join(safe_path.trim_start_matches('/'))
    }
    
    fn guess_content_type(path: &str) -> String {
        let extension = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        
        match extension.to_lowercase().as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "svg" => "image/svg+xml",
            "pdf" => "application/pdf",
            "txt" => "text/plain",
            "html" | "htm" => "text/html",
            "css" => "text/css",
            "js" => "application/javascript",
            "json" => "application/json",
            "xml" => "application/xml",
            "zip" => "application/zip",
            "mp4" => "video/mp4",
            "mp3" => "audio/mpeg",
            "woff" => "font/woff",
            "woff2" => "font/woff2",
            _ => "application/octet-stream",
        }
        .to_string()
    }
    
    fn compute_etag(content: &[u8]) -> String {
        let hash = crate::core::security::Encryption::sha256_hex(content);
        format!("\"{}\"", &hash[..16])
    }
}

#[async_trait]
impl Storage for LocalStorage {
    async fn read(&self, path: &str) -> crate::NoorResult<Vec<u8>> {
        let full_path = self.resolve_path(path);
        std::fs::read(&full_path)
            .map_err(|e| crate::NoorError::Io(e))
    }
    
    async fn write(&self, path: &str, content: &[u8], _visibility: Visibility) -> crate::NoorResult<()> {
        let full_path = self.resolve_path(path);
        
        // Create parent directories if needed
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        std::fs::write(&full_path, content)?;
        Ok(())
    }
    
    async fn delete(&self, path: &str) -> crate::NoorResult<bool> {
        let full_path = self.resolve_path(path);
        if full_path.exists() {
            std::fs::remove_file(&full_path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    async fn exists(&self, path: &str) -> crate::NoorResult<bool> {
        let full_path = self.resolve_path(path);
        Ok(full_path.exists() && full_path.is_file())
    }
    
    async fn metadata(&self, path: &str) -> crate::NoorResult<FileMetadata> {
        let full_path = self.resolve_path(path);
        
        if !full_path.exists() {
            return Ok(FileMetadata {
                path: path.to_string(),
                size: 0,
                content_type: String::new(),
                last_modified: 0,
                etag: String::new(),
                exists: false,
            });
        }
        
        let metadata = std::fs::metadata(&full_path)?;
        let content = std::fs::read(&full_path).unwrap_or_default();
        
        Ok(FileMetadata {
            path: path.to_string(),
            size: metadata.len(),
            content_type: Self::guess_content_type(path),
            last_modified: metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
            etag: Self::compute_etag(&content),
            exists: true,
        })
    }
    
    async fn list(&self, directory: &str) -> crate::NoorResult<Vec<String>> {
        let full_path = self.resolve_path(directory);
        
        if !full_path.exists() {
            return Ok(vec![]);
        }
        
        let mut files = Vec::new();
        
        for entry in std::fs::read_dir(&full_path)? {
            let entry = entry?;
            let relative = entry
                .path()
                .strip_prefix(&self.root)
                .unwrap_or(&entry.path())
                .to_string_lossy()
                .to_string();
            
            files.push(relative);
        }
        
        Ok(files)
    }
    
    async fn copy(&self, from: &str, to: &str) -> crate::NoorResult<()> {
        let from_path = self.resolve_path(from);
        let to_path = self.resolve_path(to);
        
        if let Some(parent) = to_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        std::fs::copy(&from_path, &to_path)?;
        Ok(())
    }
    
    async fn move_file(&self, from: &str, to: &str) -> crate::NoorResult<()> {
        let from_path = self.resolve_path(from);
        let to_path = self.resolve_path(to);
        
        if let Some(parent) = to_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        std::fs::rename(&from_path, &to_path)?;
        Ok(())
    }
    
    async fn url(&self, path: &str) -> crate::NoorResult<String> {
        Ok(format!("{}/{}", self.public_url_prefix, path.trim_start_matches('/')))
    }
    
    async fn temporary_url(&self, path: &str, _expires_secs: u64) -> crate::NoorResult<String> {
        // For local storage, we don't have temporary URLs
        // In a real implementation, we could generate signed URLs
        self.url(path).await
    }
    
    fn driver_name(&self) -> &str {
        "local"
    }
}

/// S3-compatible storage configuration
/// إعدادات تخزين S3
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Config {
    pub key: String,
    pub secret: String,
    pub region: String,
    pub bucket: String,
    pub endpoint: Option<String>,  // For MinIO, DigitalOcean Spaces, etc.
    pub use_path_style: bool,
    pub public_url_prefix: Option<String>,
}

/// S3-compatible storage (simulated - real impl would use aws-sdk-s3)
/// تخزين S3 (محاكى)
pub struct S3Storage {
    config: S3Config,
    /// In-memory simulation for demo purposes
    files: Arc<RwLock<std::collections::HashMap<String, Vec<u8>>>>,
}

impl S3Storage {
    pub fn new(config: S3Config) -> Self {
        Self {
            config,
            files: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }
}

#[async_trait]
impl Storage for S3Storage {
    async fn read(&self, path: &str) -> crate::NoorResult<Vec<u8>> {
        self.files
            .read()
            .get(path)
            .cloned()
            .ok_or_else(|| crate::NoorError::Internal(format!("File not found: {}", path)))
    }
    
    async fn write(&self, path: &str, content: &[u8], _visibility: Visibility) -> crate::NoorResult<()> {
        // In a real implementation: use aws-sdk-s3 to upload
        self.files.write().insert(path.to_string(), content.to_vec());
        Ok(())
    }
    
    async fn delete(&self, path: &str) -> crate::NoorResult<bool> {
        Ok(self.files.write().remove(path).is_some())
    }
    
    async fn exists(&self, path: &str) -> crate::NoorResult<bool> {
        Ok(self.files.read().contains_key(path))
    }
    
    async fn metadata(&self, path: &str) -> crate::NoorResult<FileMetadata> {
        let files = self.files.read();
        if let Some(content) = files.get(path) {
            Ok(FileMetadata {
                path: path.to_string(),
                size: content.len() as u64,
                content_type: LocalStorage::guess_content_type(path),
                last_modified: chrono::Utc::now().timestamp(),
                etag: LocalStorage::compute_etag(content),
                exists: true,
            })
        } else {
            Ok(FileMetadata {
                path: path.to_string(),
                size: 0,
                content_type: String::new(),
                last_modified: 0,
                etag: String::new(),
                exists: false,
            })
        }
    }
    
    async fn list(&self, directory: &str) -> crate::NoorResult<Vec<String>> {
        let files = self.files.read();
        Ok(files
            .keys()
            .filter(|k| k.starts_with(directory))
            .cloned()
            .collect())
    }
    
    async fn copy(&self, from: &str, to: &str) -> crate::NoorResult<()> {
        let content = self.read(from).await?;
        self.write(to, &content, Visibility::Private).await
    }
    
    async fn move_file(&self, from: &str, to: &str) -> crate::NoorResult<()> {
        self.copy(from, to).await?;
        self.delete(from).await?;
        Ok(())
    }
    
    async fn url(&self, path: &str) -> crate::NoorResult<String> {
        if let Some(ref prefix) = self.config.public_url_prefix {
            Ok(format!("{}/{}", prefix, path.trim_start_matches('/')))
        } else {
            Ok(format!(
                "https://{}.s3.{}.amazonaws.com/{}",
                self.config.bucket, self.config.region, path
            ))
        }
    }
    
    async fn temporary_url(&self, path: &str, _expires_secs: u64) -> crate::NoorResult<String> {
        // In a real implementation, generate a signed URL
        self.url(path).await
    }
    
    fn driver_name(&self) -> &str {
        "s3"
    }
}

/// Storage manager for managing multiple storage disks
/// مدير التخزين لإدارة أقراص متعددة
pub struct StorageManager {
    disks: Arc<RwLock<std::collections::HashMap<String, Arc<dyn Storage>>>>,
    default_disk: String,
}

impl Default for StorageManager {
    fn default() -> Self {
        Self::new()
    }
}

impl StorageManager {
    pub fn new() -> Self {
        Self {
            disks: Arc::new(RwLock::new(std::collections::HashMap::new())),
            default_disk: "local".to_string(),
        }
    }
    
    /// Register a storage disk
    pub fn disk(&self, name: &str, storage: Arc<dyn Storage>) {
        self.disks.write().insert(name.to_string(), storage);
    }
    
    /// Get a storage disk by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn Storage>> {
        self.disks.read().get(name).cloned()
    }
    
    /// Get the default disk
    pub fn default_disk(&self) -> Option<Arc<dyn Storage>> {
        self.get(&self.default_disk)
    }
    
    /// Set the default disk
    pub fn set_default(&mut self, name: &str) {
        self.default_disk = name.to_string();
    }
    
    /// Create a default local storage manager
    pub fn local_default(root: &str, url_prefix: &str) -> crate::NoorResult<Self> {
        let mut manager = Self::new();
        let local = Arc::new(LocalStorage::new(root, url_prefix)?);
        manager.disk("local", local);
        manager.set_default("local");
        Ok(manager)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_local_storage() {
        let storage = LocalStorage::new("/tmp/noor_test_storage", "/storage").unwrap();
        
        // Write
        storage.write("test.txt", b"Hello, World!", Visibility::Public).await.unwrap();
        
        // Read
        let content = storage.read("test.txt").await.unwrap();
        assert_eq!(content, b"Hello, World!");
        
        // Exists
        assert!(storage.exists("test.txt").await.unwrap());
        assert!(!storage.exists("nonexistent.txt").await.unwrap());
        
        // Metadata
        let meta = storage.metadata("test.txt").await.unwrap();
        assert_eq!(meta.size, 13);
        assert!(meta.exists);
        assert_eq!(meta.content_type, "text/plain");
        
        // URL
        let url = storage.url("test.txt").await.unwrap();
        assert_eq!(url, "/storage/test.txt");
        
        // Delete
        assert!(storage.delete("test.txt").await.unwrap());
        assert!(!storage.exists("test.txt").await.unwrap());
    }
    
    #[tokio::test]
    async fn test_s3_storage() {
        let config = S3Config {
            key: "test_key".to_string(),
            secret: "test_secret".to_string(),
            region: "us-east-1".to_string(),
            bucket: "test-bucket".to_string(),
            endpoint: None,
            use_path_style: false,
            public_url_prefix: Some("https://cdn.example.com".to_string()),
        };
        
        let storage = S3Storage::new(config);
        
        storage.write("test.txt", b"Hello, S3!", Visibility::Public).await.unwrap();
        
        let content = storage.read("test.txt").await.unwrap();
        assert_eq!(content, b"Hello, S3!");
        
        let url = storage.url("test.txt").await.unwrap();
        assert_eq!(url, "https://cdn.example.com/test.txt");
    }
}
