# Contributing to Noor Framework | المساهمة في إطار عمل نور

Thank you for your interest in contributing to Noor! This document provides guidelines for contributing.

شكراً لاهتمامك بالمساهمة في نور! يوفر هذا المستند إرشادات المساهمة.

## 🌟 Ways to Contribute | طرق المساهمة

- 🐛 **Bug Reports** - Report bugs via GitHub Issues
- 💡 **Feature Requests** - Suggest new features
- 📝 **Documentation** - Improve docs (Arabic or English)
- 💻 **Code** - Submit pull requests
- 🧪 **Tests** - Add test cases
- 🌍 **Translation** - Help translate docs

## 🛠️ Development Setup | إعداد التطوير

### Prerequisites | المتطلبات

- Rust 1.75+
- Zig 0.11+ (optional, for performance modules)
- Git

### Getting Started | البداية

```bash
# Fork and clone the repo
git clone https://github.com/your-username/noor.git
cd noor

# Create a feature branch
git checkout -b feature/my-new-feature

# Build the project
cargo build

# Run tests
cargo test

# Run the demo
cargo run --bin noor-server
```

## 📝 Code Style | أسلوب الكود

### Rust Code | كود Rust

- Follow `rustfmt` formatting
- Use `clippy` for linting
- Add doc comments (in Arabic + English for public APIs)
- Write tests for new features

```rust
/// Generates a new CSRF token | يولد رمز CSRF جديد
/// 
/// # Arguments
/// * `lifetime_secs` - Token lifetime in seconds
/// 
/// # Returns
/// Returns the generated token as a hex string
/// 
/// # Errors
/// Returns an error if the RNG fails
pub fn generate_token(&self, lifetime_secs: u64) -> NoorResult<String> {
    // Implementation
}
```

### Zig Code | كود Zig

- Follow Zig's standard formatting (`zig fmt`)
- Add doc comments
- Write tests with `test` blocks

## 🧪 Testing | الاختبار

All new features must include tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_new_feature() {
        // Test implementation
        assert!(true);
    }
}
```

Run tests with | تشغيل الاختبارات بـ:

```bash
cargo test                    # All tests
cargo test -- --nocapture     # With output
cargo test security           # Specific module
```

## 🔄 Pull Request Process | عملية Pull Request

1. **Fork** the repository
2. **Create** a feature branch (`git checkout -b feature/amazing-feature`)
3. **Commit** your changes (`git commit -m 'Add amazing feature'`)
4. **Push** to the branch (`git push origin feature/amazing-feature`)
5. **Open** a Pull Request

### PR Checklist | قائمة PR

- [ ] Code follows the style guide
- [ ] Tests added/updated and passing
- [ ] Documentation updated (both Arabic and English)
- [ ] Commit messages are clear and descriptive
- [ ] PR description explains the changes

## 🐛 Bug Reports | تقارير الأخطاء

When reporting bugs, please include:

1. **Description** - Clear description of the issue
2. **Steps to Reproduce** - Minimal reproduction steps
3. **Expected Behavior** - What should happen
4. **Actual Behavior** - What actually happens
5. **Environment** - OS, Rust version, Zig version
6. **Code Sample** - If applicable

## 💡 Feature Requests | طلبات الميزات

When requesting features, please include:

1. **Problem** - What problem does this solve?
2. **Solution** - What do you suggest?
3. **Alternatives** - Have you considered alternatives?
4. **Additional Context** - Any other information

## 📜 Code of Conduct | مدونة قواعد السلوك

Be respectful and inclusive. We are committed to providing a welcoming and harassment-free experience for everyone.

كن محترماً وشاملاً. نحن ملتزمون بتقديم تجربة ترحيبية خالية من المضايقات للجميع.

## 📞 Contact | التواصل

- **GitHub Issues** - For bugs and features
- **Discord** - For discussions
- **Email** - noor-framework@example.com

Thank you for contributing! | شكراً لمساهمتك!
