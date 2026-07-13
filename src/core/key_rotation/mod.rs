// ============================================================
// Encryption Key Rotation - تدوير مفاتيح التشفير
// ============================================================
// Securely rotate encryption keys without data loss.
// Supports multiple active keys during transition period.
//
// تدوير آمن لمفاتيح التشفير بدون فقدان البيانات.
// ============================================================

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

/// Encryption key with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionKey {
    pub id: String,
    pub key: [u8; 32],
    pub created_at: i64,
    pub rotated_at: Option<i64>,
    pub status: KeyStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyStatus {
    /// Active key used for new encryption
    Active,
    /// Previous key, still used for decryption
    Previous,
    /// Deprecated, scheduled for removal
    Deprecated,
    /// Removed
    Revoked,
}

/// Key rotation manager
pub struct KeyRotationManager {
    keys: Arc<RwLock<HashMap<String, EncryptionKey>>>,
    active_key_id: Arc<RwLock<String>>,
    /// How long to keep old keys for decryption (seconds)
    retention_period: i64,
}

impl Default for KeyRotationManager {
    fn default() -> Self {
        Self::new(86400 * 30) // 30 days retention
    }
}

impl KeyRotationManager {
    pub fn new(retention_period_secs: i64) -> Self {
        Self {
            keys: Arc::new(RwLock::new(HashMap::new())),
            active_key_id: Arc::new(RwLock::new(String::new())),
            retention_period: retention_period_secs,
        }
    }
    
    /// Initialize with a new key
    pub fn initialize(&self) -> crate::NoorResult<String> {
        let key = self.generate_key()?;
        self.add_key(key)
    }
    
    /// Generate a new encryption key
    fn generate_key(&self) -> crate::NoorResult<EncryptionKey> {
        let enc = crate::core::security::Encryption::new();
        let key_bytes = enc.generate_key()?;
        
        let key_id = enc.random_string(8)?;
        
        Ok(EncryptionKey {
            id: key_id,
            key: key_bytes,
            created_at: chrono::Utc::now().timestamp(),
            rotated_at: None,
            status: KeyStatus::Active,
        })
    }
    
    /// Add a key to the manager
    fn add_key(&self, mut key: EncryptionKey) -> crate::NoorResult<String> {
        // Mark existing active key as previous
        let active_id = self.active_key_id.read().clone();
        
        if !active_id.is_empty() {
            if let Some(old_key) = self.keys.write().get_mut(&active_id) {
                old_key.status = KeyStatus::Previous;
                old_key.rotated_at = Some(chrono::Utc::now().timestamp());
            }
        }
        
        let key_id = key.id.clone();
        self.keys.write().insert(key_id.clone(), key);
        *self.active_key_id.write() = key_id.clone();
        
        Ok(key_id)
    }
    
    /// Rotate the active key
    pub fn rotate(&self) -> crate::NoorResult<String> {
        tracing::info!("Rotating encryption key...");
        
        let new_key = self.generate_key()?;
        let new_id = self.add_key(new_key)?;
        
        // Clean up expired keys
        self.cleanup_expired();
        
        tracing::info!("Key rotated. New active key: {}", new_id);
        
        Ok(new_id)
    }
    
    /// Get the active key
    pub fn active_key(&self) -> Option<EncryptionKey> {
        let active_id = self.active_key_id.read().clone();
        self.keys.read().get(&active_id).cloned()
    }
    
    /// Get the active key ID
    pub fn active_key_id(&self) -> String {
        self.active_key_id.read().clone()
    }
    
    /// Get a key by ID
    pub fn get_key(&self, key_id: &str) -> Option<EncryptionKey> {
        self.keys.read().get(key_id).cloned()
    }
    
    /// Encrypt data with the active key
    pub fn encrypt(&self, plaintext: &[u8]) -> crate::NoorResult<EncryptedData> {
        let active = self.active_key()
            .ok_or_else(|| crate::NoorError::Security("No active key".to_string()))?;
        
        let enc = crate::core::security::Encryption::new();
        let ciphertext = enc.encrypt(plaintext, &active.key)?;
        
        Ok(EncryptedData {
            key_id: active.id,
            ciphertext,
            encrypted_at: chrono::Utc::now().timestamp(),
        })
    }
    
    /// Decrypt data (tries active key first, then previous keys)
    pub fn decrypt(&self, data: &EncryptedData) -> crate::NoorResult<Vec<u8>> {
        let key = self.get_key(&data.key_id)
            .ok_or_else(|| crate::NoorError::Security(
                format!("Key {} not found or revoked", data.key_id)
            ))?;
        
        if key.status == KeyStatus::Revoked {
            return Err(crate::NoorError::Security("Key has been revoked".to_string()));
        }
        
        let enc = crate::core::security::Encryption::new();
        enc.decrypt(&data.ciphertext, &key.key)
    }
    
    /// Re-encrypt data with the current active key
    pub fn re_encrypt(&self, data: &EncryptedData) -> crate::NoorResult<EncryptedData> {
        let plaintext = self.decrypt(data)?;
        self.encrypt(&plaintext)
    }
    
    /// Re-encrypt all data (bulk operation)
    pub fn re_encrypt_all(&self, data: &[EncryptedData]) -> crate::NoorResult<Vec<EncryptedData>> {
        data.iter()
            .map(|d| self.re_encrypt(d))
            .collect()
    }
    
    /// Revoke a key (can no longer decrypt)
    pub fn revoke_key(&self, key_id: &str) -> bool {
        if let Some(key) = self.keys.write().get_mut(key_id) {
            key.status = KeyStatus::Revoked;
            return true;
        }
        false
    }
    
    /// List all keys
    pub fn list_keys(&self) -> Vec<EncryptionKey> {
        self.keys.read().values().cloned().collect()
    }
    
    /// Get key count
    pub fn key_count(&self) -> usize {
        self.keys.read().len()
    }
    
    /// Clean up expired keys
    pub fn cleanup_expired(&self) -> usize {
        let now = chrono::Utc::now().timestamp();
        let mut cleaned = 0;
        
        let mut keys = self.keys.write();
        
        keys.retain(|_, key| {
            if key.status == KeyStatus::Deprecated {
                if let Some(rotated) = key.rotated_at {
                    if now - rotated > self.retention_period {
                        cleaned += 1;
                        return false;
                    }
                }
            }
            true
        });
        
        cleaned
    }
    
    /// Schedule a key for deprecation
    pub fn deprecate_key(&self, key_id: &str) -> bool {
        if let Some(key) = self.keys.write().get_mut(key_id) {
            if key.status == KeyStatus::Previous {
                key.status = KeyStatus::Deprecated;
                return true;
            }
        }
        false
    }
    
    /// Get rotation statistics
    pub fn stats(&self) -> KeyRotationStats {
        let keys = self.keys.read();
        
        let active = keys.values().filter(|k| k.status == KeyStatus::Active).count();
        let previous = keys.values().filter(|k| k.status == KeyStatus::Previous).count();
        let deprecated = keys.values().filter(|k| k.status == KeyStatus::Deprecated).count();
        let revoked = keys.values().filter(|k| k.status == KeyStatus::Revoked).count();
        
        let last_rotation = keys.values()
            .filter_map(|k| k.rotated_at)
            .max()
            .unwrap_or(0);
        
        KeyRotationStats {
            total_keys: keys.len(),
            active,
            previous,
            deprecated,
            revoked,
            active_key_id: self.active_key_id(),
            last_rotation,
        }
    }
}

/// Encrypted data with key reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    pub key_id: String,
    pub ciphertext: Vec<u8>,
    pub encrypted_at: i64,
}

/// Key rotation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRotationStats {
    pub total_keys: usize,
    pub active: usize,
    pub previous: usize,
    pub deprecated: usize,
    pub revoked: usize,
    pub active_key_id: String,
    pub last_rotation: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_key_initialization() {
        let manager = KeyRotationManager::default();
        
        let key_id = manager.initialize().unwrap();
        
        assert_eq!(manager.key_count(), 1);
        assert_eq!(manager.active_key_id(), key_id);
        assert!(manager.active_key().is_some());
    }
    
    #[test]
    fn test_key_rotation() {
        let manager = KeyRotationManager::default();
        
        let key1 = manager.initialize().unwrap();
        let key2 = manager.rotate().unwrap();
        
        assert_ne!(key1, key2);
        assert_eq!(manager.key_count(), 2);
        assert_eq!(manager.active_key_id(), key2);
        
        // Old key should be marked as previous
        let old_key = manager.get_key(&key1).unwrap();
        assert_eq!(old_key.status, KeyStatus::Previous);
    }
    
    #[test]
    fn test_encrypt_decrypt() {
        let manager = KeyRotationManager::default();
        manager.initialize().unwrap();
        
        let plaintext = b"secret data";
        
        let encrypted = manager.encrypt(plaintext).unwrap();
        let decrypted = manager.decrypt(&encrypted).unwrap();
        
        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    }
    
    #[test]
    fn test_decrypt_with_old_key() {
        let manager = KeyRotationManager::default();
        
        let _key1 = manager.initialize().unwrap();
        
        // Encrypt with first key
        let plaintext = b"old secret";
        let encrypted = manager.encrypt(plaintext).unwrap();
        
        // Rotate key
        let _key2 = manager.rotate().unwrap();
        
        // Should still decrypt with old key
        let decrypted = manager.decrypt(&encrypted).unwrap();
        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    }
    
    #[test]
    fn test_re_encrypt() {
        let manager = KeyRotationManager::default();
        
        let _key1 = manager.initialize().unwrap();
        let encrypted = manager.encrypt(b"data").unwrap();
        
        let _key2 = manager.rotate().unwrap();
        
        let re_encrypted = manager.re_encrypt(&encrypted).unwrap();
        
        // Should use new key
        assert_eq!(re_encrypted.key_id, manager.active_key_id());
        
        // Should decrypt correctly
        let decrypted = manager.decrypt(&re_encrypted).unwrap();
        assert_eq!(decrypted, b"data");
    }
    
    #[test]
    fn test_revoke_key() {
        let manager = KeyRotationManager::default();
        
        let key1 = manager.initialize().unwrap();
        let _key2 = manager.rotate().unwrap();
        
        assert!(manager.revoke_key(&key1));
        
        let key = manager.get_key(&key1).unwrap();
        assert_eq!(key.status, KeyStatus::Revoked);
        
        // Decryption should fail with revoked key
        let encrypted = EncryptedData {
            key_id: key1,
            ciphertext: vec![1, 2, 3],
            encrypted_at: chrono::Utc::now().timestamp(),
        };
        
        assert!(manager.decrypt(&encrypted).is_err());
    }
    
    #[test]
    fn test_stats() {
        let manager = KeyRotationManager::default();
        
        let _ = manager.initialize().unwrap();
        let _ = manager.rotate().unwrap();
        let _ = manager.rotate().unwrap();
        
        let stats = manager.stats();
        
        assert_eq!(stats.total_keys, 3);
        assert_eq!(stats.active, 1);
        assert_eq!(stats.previous, 2);
    }
}
