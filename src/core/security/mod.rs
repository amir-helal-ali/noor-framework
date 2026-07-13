// ============================================================
// Security Module - وحدة الأمان
// ============================================================
// Built-in protection against common web vulnerabilities:
// - CSRF (Cross-Site Request Forgery)
// - XSS (Cross-Site Scripting)  
// - SQL Injection (via parameterized queries in ORM)
// - Rate Limiting
// - Encryption
// - Input Validation
//
// حماية مدمجة ضد الثغرات الشائعة.
// ============================================================

pub mod csrf;
pub mod xss;
pub mod rate_limit;
pub mod encryption;
pub mod validator;

pub use csrf::Csrf;
pub use xss::Xss;
pub use rate_limit::RateLimit;
pub use encryption::Encryption;
pub use validator::Validator;
