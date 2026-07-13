// ============================================================
// Example: Security Features
// مثال: ميزات الأمان
// ============================================================

use noor::*;
use noor::core::security::{Csrf, Xss, RateLimit, Encryption, Validator};

fn main() -> NoorResult<()> {
    println!("{}", banner());
    println!("\n🔒 Security Features Demo\n");
    
    // 1. CSRF Protection
    let csrf = Csrf::new(3600);
    let token = csrf.generate_token()?;
    println!("✓ CSRF Token: {}...", &token[..16]);
    println!("  Valid: {}", csrf.validate_token(&token));
    
    // 2. XSS Protection
    let malicious = "<script>alert('xss')</script><p>hello</p>";
    let xss = Xss::new();
    let cleaned = xss.sanitize(malicious);
    let escaped = Xss::escape(malicious);
    println!("\n✓ XSS Protection:");
    println!("  Original: {}", malicious);
    println!("  Sanitized: {}", cleaned);
    println!("  Escaped: {}", escaped);
    
    // 3. Rate Limiting
    let limiter = RateLimit::new(5, 60);
    println!("\n✓ Rate Limiting (5 req/min):");
    for i in 1..=7 {
        let result = limiter.check("192.168.1.1");
        let status = if result.allowed { "ALLOWED" } else { "BLOCKED" };
        println!("  Request {}: {} (remaining: {})", i, status, result.remaining);
    }
    
    // 4. Password Hashing
    let password = "my_secure_password_123";
    let hash = Encryption::hash_password(password)?;
    println!("\n✓ Password Hashing:");
    println!("  Password: {}", password);
    println!("  Hash: {}...", &hash[..30]);
    println!("  Verify (correct): {}", Encryption::verify_password(password, &hash));
    println!("  Verify (wrong): {}", Encryption::verify_password("wrong", &hash));
    
    // 5. Encryption
    let enc = Encryption::new();
    let key = enc.generate_key()?;
    let plaintext = b"Sensitive data to encrypt";
    let ciphertext = enc.encrypt(plaintext, &key)?;
    let decrypted = enc.decrypt(&ciphertext, &key)?;
    println!("\n✓ AES-256-GCM Encryption:");
    println!("  Plaintext: {}", String::from_utf8_lossy(plaintext));
    println!("  Decrypted: {}", String::from_utf8_lossy(&decrypted));
    println!("  Match: {}", plaintext.as_slice() == decrypted.as_slice());
    
    // 6. Input Validation
    println!("\n✓ Input Validation:");
    println!("  Email 'user@example.com': {}", Validator::is_email("user@example.com"));
    println!("  Email 'invalid': {}", Validator::is_email("invalid"));
    println!("  Strong password 'Str0ng!Pass': {}", Validator::is_strong_password("Str0ng!Pass"));
    println!("  Strong password 'weak': {}", Validator::is_strong_password("weak"));
    
    println!("\n✅ All security features working correctly!");
    Ok(())
}
