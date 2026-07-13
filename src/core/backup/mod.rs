// ============================================================
// نظام النسخ الاحتياطي والاستعادة | Backup & Restore System
// ============================================================

use std::path::{Path, PathBuf};
use std::io::Write;
use std::sync::Arc;
use parking_lot::RwLock;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Serialize, Deserialize};

/// نوع النسخة الاحتياطية
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BackupType {
    Database,
    Files,
    Full,
}

/// حالة النسخة الاحتياطية
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    pub id: String,
    pub filename: String,
    pub backup_type: BackupType,
    pub size_bytes: u64,
    pub created_at: i64,
    pub path: String,
    pub checksum: String,
}

/// إعدادات النسخ الاحتياطي
#[derive(Debug, Clone)]
pub struct BackupConfig {
    pub backup_dir: String,
    pub max_backups: usize,
    pub compress: bool,
    pub include_database: bool,
    pub include_files: bool,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            backup_dir: "storage/backups".to_string(),
            max_backups: 10,
            compress: true,
            include_database: true,
            include_files: false,
        }
    }
}

/// مدير النسخ الاحتياطي
pub struct BackupManager {
    config: BackupConfig,
    backups: Arc<RwLock<Vec<BackupInfo>>>,
}

impl BackupManager {
    pub fn new(config: BackupConfig) -> crate::NoorResult<Self> {
        let backup_dir = PathBuf::from(&config.backup_dir);
        std::fs::create_dir_all(&backup_dir)?;
        
        let manager = Self {
            config,
            backups: Arc::new(RwLock::new(Vec::new())),
        };
        
        // تحميل النسخ الموجودة
        manager.load_existing_backups()?;
        
        Ok(manager)
    }
    
    /// إنشاء نسخة احتياطية
    pub fn create_backup(&self, backup_type: BackupType) -> crate::NoorResult<BackupInfo> {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let id = uuid::Uuid::new_v4().to_string();
        let extension = if self.config.compress { "tar.gz" } else { "tar" };
        let filename = format!("backup_{}_{}.{}", timestamp, backup_type_name(&backup_type), extension);
        let path = PathBuf::from(&self.config.backup_dir).join(&filename);
        
        let mut backup_data = Vec::new();
        
        // نسخ احتياطي لقاعدة البيانات
        if self.config.include_database && backup_type != BackupType::Files {
            let db_content = self.backup_database()?;
            backup_data.push(("database.sql".to_string(), db_content));
        }
        
        // نسخ احتياطي للملفات
        if self.config.include_files && backup_type != BackupType::Database {
            let files = self.backup_files()?;
            backup_data.extend(files);
        }
        
        // كتابة الأرشيف
        let size_bytes = if self.config.compress {
            self.write_compressed(&path, &backup_data)?
        } else {
            self.write_uncompressed(&path, &backup_data)?
        };
        
        // حساب checksum
        let content = std::fs::read(&path)?;
        let checksum = crate::core::security::Encryption::sha256_hex(&content);
        
        let backup_info = BackupInfo {
            id: id.clone(),
            filename: filename.clone(),
            backup_type,
            size_bytes,
            created_at: chrono::Utc::now().timestamp(),
            path: path.to_string_lossy().to_string(),
            checksum,
        };
        
        // إضافة للقائمة
        self.backups.write().push(backup_info.clone());
        
        // تنظيف النسخ القديمة
        self.cleanup_old_backups();
        
        tracing::info!("Backup created: {}", filename);
        
        Ok(backup_info)
    }
    
    /// نسخ احتياطي لقاعدة البيانات (محاكاة)
    fn backup_database(&self) -> crate::NoorResult<Vec<u8>> {
        // في تطبيق حقيقي، سنستخدم pg_dump أو mysqldump أو نسخ ملف SQLite
        let sql_content = "-- Noor Framework Database Backup\n";
        let content = format!("-- Created at: {}\n\n-- Tables and data would be here\n", 
            chrono::Utc::now().to_rfc3339());
        
        Ok((sql_content.to_string() + &content).into_bytes())
    }
    
    /// نسخ احتياطي للملفات (محاكاة)
    fn backup_files(&self) -> crate::NoorResult<Vec<(String, Vec<u8>)>> {
        let mut files = Vec::new();
        
        // نسخ ملفات التحميل
        let uploads_dir = PathBuf::from("storage/uploads");
        if uploads_dir.exists() {
            for entry in walkdir::WalkDir::new(&uploads_dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let relative = entry.path()
                    .strip_prefix(&uploads_dir)
                    .unwrap_or(entry.path())
                    .to_string_lossy()
                    .to_string();
                
                if let Ok(content) = std::fs::read(entry.path()) {
                    files.push((format!("uploads/{}", relative), content));
                }
            }
        }
        
        Ok(files)
    }
    
    /// كتابة أرشيف مضغوط
    fn write_compressed(&self, path: &Path, files: &[(String, Vec<u8>)]) -> crate::NoorResult<u64> {
        let tar_file = std::fs::File::create(path)?;
        let encoder = GzEncoder::new(tar_file, Compression::default());
        let mut tar = tar::Builder::new(encoder);
        
        for (name, content) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            
            tar.append_data(&mut header, name, std::io::Cursor::new(content))?;
        }
        
        let encoder = tar.into_inner()?;
        encoder.finish()?;
        
        Ok(std::fs::metadata(path)?.len())
    }
    
    /// كتابة أرشيف غير مضغوط
    fn write_uncompressed(&self, path: &Path, files: &[(String, Vec<u8>)]) -> crate::NoorResult<u64> {
        let tar_file = std::fs::File::create(path)?;
        let mut tar = tar::Builder::new(tar_file);
        
        for (name, content) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            
            tar.append_data(&mut header, name, std::io::Cursor::new(content))?;
        }
        
        tar.finish()?;
        
        Ok(std::fs::metadata(path)?.len())
    }
    
    /// استعادة نسخة احتياطية
    pub fn restore_backup(&self, backup_id: &str) -> crate::NoorResult<()> {
        let backup = self.backups.read()
            .iter()
            .find(|b| b.id == backup_id)
            .cloned()
            .ok_or_else(|| crate::NoorError::Internal("Backup not found".to_string()))?;
        
        let path = PathBuf::from(&backup.path);
        
        if !path.exists() {
            return Err(crate::NoorError::Internal("Backup file not found".to_string()));
        }
        
        // التحقق من checksum
        let content = std::fs::read(&path)?;
        let checksum = crate::core::security::Encryption::sha256_hex(&content);
        
        if checksum != backup.checksum {
            return Err(crate::NoorError::Security("Backup checksum mismatch".to_string()));
        }
        
        // فك الضغط والاستعادة
        // في تطبيق حقيقي، سنستخرج ونستورد البيانات
        
        tracing::info!("Backup restored: {}", backup.filename);
        
        Ok(())
    }
    
    /// حذف نسخة احتياطية
    pub fn delete_backup(&self, backup_id: &str) -> crate::NoorResult<bool> {
        let mut backups = self.backups.write();
        
        if let Some(pos) = backups.iter().position(|b| b.id == backup_id) {
            let backup = backups.remove(pos);
            
            let path = PathBuf::from(&backup.path);
            if path.exists() {
                std::fs::remove_file(&path)?;
            }
            
            return Ok(true);
        }
        
        Ok(false)
    }
    
    /// الحصول على جميع النسخ الاحتياطية
    pub fn list_backups(&self) -> Vec<BackupInfo> {
        self.backups.read().clone()
    }
    
    /// الحصول على نسخة احتياطية محددة
    pub fn get_backup(&self, backup_id: &str) -> Option<BackupInfo> {
        self.backups.read().iter().find(|b| b.id == backup_id).cloned()
    }
    
    /// تنظيف النسخ القديمة
    fn cleanup_old_backups(&self) {
        let mut backups = self.backups.write();
        
        if backups.len() > self.config.max_backups {
            // ترتيب حسب التاريخ (الأقدم أولاً)
            backups.sort_by_key(|b| b.created_at);
            
            // حذف الأقدم
            let to_remove = backups.len() - self.config.max_backups;
            for _ in 0..to_remove {
                if let Some(backup) = backups.first().cloned() {
                    let path = PathBuf::from(&backup.path);
                    if path.exists() {
                        std::fs::remove_file(&path).ok();
                    }
                    backups.remove(0);
                }
            }
        }
    }
    
    /// تحميل النسخ الموجودة
    fn load_existing_backups(&self) -> crate::NoorResult<()> {
        let backup_dir = PathBuf::from(&self.config.backup_dir);
        
        if !backup_dir.exists() {
            return Ok(());
        }
        
        let mut backups = Vec::new();
        
        for entry in std::fs::read_dir(&backup_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                let filename = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                
                if filename.starts_with("backup_") {
                    let metadata = std::fs::metadata(&path)?;
                    let content = std::fs::read(&path)?;
                    let checksum = crate::core::security::Encryption::sha256_hex(&content);
                    
                    let backup_type = if filename.contains("database") {
                        BackupType::Database
                    } else if filename.contains("files") {
                        BackupType::Files
                    } else {
                        BackupType::Full
                    };
                    
                    backups.push(BackupInfo {
                        id: uuid::Uuid::new_v4().to_string(),
                        filename: filename.to_string(),
                        backup_type,
                        size_bytes: metadata.len(),
                        created_at: metadata.created()
                            .ok()
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_secs() as i64)
                            .unwrap_or(0),
                        path: path.to_string_lossy().to_string(),
                        checksum,
                    });
                }
            }
        }
        
        // ترتيب حسب التاريخ (الأحدث أولاً)
        backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        *self.backups.write() = backups;
        
        Ok(())
    }
    
    /// الحصول على حجم جميع النسخ الاحتياطية
    pub fn total_size(&self) -> u64 {
        self.backups.read().iter().map(|b| b.size_bytes).sum()
    }
    
    /// الحصول على عدد النسخ الاحتياطية
    pub fn count(&self) -> usize {
        self.backups.read().len()
    }
}

fn backup_type_name(backup_type: &BackupType) -> &'static str {
    match backup_type {
        BackupType::Database => "database",
        BackupType::Files => "files",
        BackupType::Full => "full",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_backup_manager_creation() {
        let config = BackupConfig {
            backup_dir: "/tmp/noor_backups_test".to_string(),
            max_backups: 5,
            compress: true,
            include_database: true,
            include_files: false,
        };
        
        let manager = BackupManager::new(config).unwrap();
        
        assert_eq!(manager.count(), 0);
    }
    
    #[test]
    fn test_create_backup() {
        let backup_dir = "/tmp/noor_backups_test2";
        // Start from a clean directory so previous test runs don't inflate
        // the count of pre-existing backups that load_existing_backups() picks up.
        std::fs::remove_dir_all(backup_dir).ok();
        let config = BackupConfig {
            backup_dir: backup_dir.to_string(),
            max_backups: 5,
            compress: true,
            include_database: true,
            include_files: false,
        };

        let manager = BackupManager::new(config).unwrap();

        let backup = manager.create_backup(BackupType::Database).unwrap();

        assert!(!backup.id.is_empty());
        assert!(backup.size_bytes > 0);
        assert!(!backup.checksum.is_empty());
        assert_eq!(manager.count(), 1);
    }

    #[test]
    fn test_list_backups() {
        let backup_dir = "/tmp/noor_backups_test3";
        std::fs::remove_dir_all(backup_dir).ok();
        let config = BackupConfig {
            backup_dir: backup_dir.to_string(),
            ..Default::default()
        };

        let manager = BackupManager::new(config).unwrap();

        manager.create_backup(BackupType::Database).unwrap();
        manager.create_backup(BackupType::Full).unwrap();

        let backups = manager.list_backups();
        assert_eq!(backups.len(), 2);
    }
}
