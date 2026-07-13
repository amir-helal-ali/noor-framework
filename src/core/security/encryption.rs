// ============================================================
// Encryption - التشفير
// ============================================================
// Provides AES-256-GCM encryption for sensitive data and
// Argon2 for password hashing.
//
// يوفر تشفير AES-256-GCM و Argon2 للباسورد.
// ============================================================

use ring::aead;
use ring::pbkdf2;
use ring::rand::{SecureRandom, SystemRandom};
use argon2::{self, Algorithm, Argon2, Version, Params};
use constant_time_eq::constant_time_eq;

/// Encryption utility
/// أداة التشفير
pub struct Encryption {
    rng: SystemRandom,
}

impl Default for Encryption {
    fn default() -> Self {
        Self::new()
    }
}

impl Encryption {
    pub fn new() -> Self {
        Self {
            rng: SystemRandom::new(),
        }
    }
    
    /// Generate a random key
    /// توليد مفتاح عشوائي
    pub fn generate_key(&self) -> crate::NoorResult<[u8; 32]> {
        let mut key = [0u8; 32];
        self.rng
            .fill(&mut key)
            .map_err(|e| crate::NoorError::Security(format!("RNG error: {}", e)))?;
        Ok(key)
    }
    
    /// Generate a random string of given length
    /// توليد نص عشوائي بطول معين
    pub fn random_string(&self, length: usize) -> crate::NoorResult<String> {
        let mut bytes = vec![0u8; length];
        self.rng
            .fill(&mut bytes)
            .map_err(|e| crate::NoorError::Security(format!("RNG error: {}", e)))?;
        Ok(hex::encode(bytes))
    }
    
    /// Encrypt data using AES-256-GCM
    /// تشفير البيانات باستخدام AES-256-GCM
    ///
    /// Output layout: `[nonce (12 bytes) || encrypted_plaintext || tag (16 bytes)]`.
    /// The nonce is prepended in plaintext so the receiver can reuse it for
    /// decryption. Critically, the nonce must NOT be part of the buffer passed
    /// to `seal_in_place_append_tag`, otherwise it gets encrypted in place and
    /// decryption fails.
    pub fn encrypt(&self, plaintext: &[u8], key: &[u8; 32]) -> crate::NoorResult<Vec<u8>> {
        let mut nonce = [0u8; 12];
        self.rng
            .fill(&mut nonce)
            .map_err(|e| crate::NoorError::Security(format!("RNG error: {}", e)))?;

        let unbound_key = aead::UnboundKey::new(&aead::AES_256_GCM, key)
            .map_err(|e| crate::NoorError::Security(format!("Key error: {}", e)))?;
        let sealing_key = aead::LessSafeKey::new(unbound_key);

        // Encrypt ONLY the plaintext in place, then prepend the nonce.
        let mut encrypted_payload = plaintext.to_vec();
        sealing_key
            .seal_in_place_append_tag(
                aead::Nonce::assume_unique_for_key(nonce),
                aead::Aad::empty(),
                &mut encrypted_payload,
            )
            .map_err(|e| crate::NoorError::Security(format!("Encryption error: {}", e)))?;

        let mut ciphertext = Vec::with_capacity(12 + encrypted_payload.len());
        ciphertext.extend_from_slice(&nonce);
        ciphertext.extend_from_slice(&encrypted_payload);

        Ok(ciphertext)
    }
    
    /// Decrypt data using AES-256-GCM
    /// فك تشفير البيانات
    pub fn decrypt(&self, ciphertext: &[u8], key: &[u8; 32]) -> crate::NoorResult<Vec<u8>> {
        if ciphertext.len() < 28 {
            return Err(crate::NoorError::Security("Ciphertext too short".to_string()));
        }
        
        let (nonce_bytes, encrypted) = ciphertext.split_at(12);
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(nonce_bytes);
        
        let unbound_key = aead::UnboundKey::new(&aead::AES_256_GCM, key)
            .map_err(|e| crate::NoorError::Security(format!("Key error: {}", e)))?;
        let opening_key = aead::LessSafeKey::new(unbound_key);
        
        let mut plaintext = encrypted.to_vec();
        
        opening_key
            .open_in_place(aead::Nonce::assume_unique_for_key(nonce), aead::Aad::empty(), &mut plaintext)
            .map_err(|e| crate::NoorError::Security(format!("Decryption error: {}", e)))?;
        
        let plaintext_len = plaintext.len() - aead::AES_256_GCM.tag_len();
        plaintext.truncate(plaintext_len);
        
        Ok(plaintext)
    }
    
    /// Hash a password using Argon2id
    /// تشفير كلمة المرور باستخدام Argon2id
    pub fn hash_password(password: &str) -> crate::NoorResult<String> {
        use argon2::password_hash::{rand_core::OsRng, SaltString, PasswordHasher};

        // 64MB memory, 3 iterations, 4 lanes - adjustable for weak servers
        let params = Params::new(65536, 3, 4, Some(32))
            .map_err(|e| crate::NoorError::Security(format!("Argon2 params error: {}", e)))?;

        let argon2 = Argon2::new(Algorithm::Argon2id, Version::default(), params);
        let salt = SaltString::generate(&mut OsRng);

        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| crate::NoorError::Security(format!("Argon2 error: {}", e)))?;

        Ok(password_hash.to_string())
    }

    /// Verify a password against a hash
    /// التحقق من كلمة المرور
    pub fn verify_password(password: &str, hash: &str) -> bool {
        use argon2::password_hash::{PasswordHash, PasswordVerifier};

        let parsed_hash = match PasswordHash::new(hash) {
            Ok(h) => h,
            Err(_) => return false,
        };
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok()
    }
    
    /// Hash data using SHA-256
    /// تشفير البيانات باستخدام SHA-256
    pub fn sha256(data: &[u8]) -> [u8; 32] {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().into()
    }
    
    /// Hash data using SHA-256 and return hex string
    /// تشفير البيانات وإرجاع hex
    pub fn sha256_hex(data: &[u8]) -> String {
        hex::encode(Self::sha256(data))
    }
    
    /// HMAC-SHA256 for signing
    /// HMAC-SHA256 للتوقيع
    pub fn hmac_sha256(key: &[u8], data: &[u8]) -> crate::NoorResult<Vec<u8>> {
        use ring::hmac;
        let key = hmac::Key::new(hmac::HMAC_SHA256, key);
        let tag = hmac::sign(&key, data);
        Ok(tag.as_ref().to_vec())
    }
    
    /// Constant-time comparison to prevent timing attacks
    /// مقارنة بزمن ثابت لمنع هجمات التوقيت
    pub fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
        constant_time_eq(a, b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_password_hashing() {
        let password = "my_secret_password";
        let hash = Encryption::hash_password(password).unwrap();
        assert!(Encryption::verify_password(password, &hash));
        assert!(!Encryption::verify_password("wrong_password", &hash));
    }
    
    #[test]
    fn test_encryption() {
        let enc = Encryption::new();
        let key = enc.generate_key().unwrap();
        let plaintext = b"Hello, World!";
        let ciphertext = enc.encrypt(plaintext, &key).unwrap();
        let decrypted = enc.decrypt(&ciphertext, &key).unwrap();
        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    }
    
    #[test]
    fn test_sha256() {
        let hash = Encryption::sha256_hex(b"hello");
        assert_eq!(hash, "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824");
    }
}
