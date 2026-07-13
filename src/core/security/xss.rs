// ============================================================
// XSS Protection - الحماية من XSS
// ============================================================
// Sanitizes user input to prevent Cross-Site Scripting attacks.
// Escapes HTML entities and removes dangerous content.
//
// يعقم المدخلات لمنع هجمات XSS.
// ============================================================

use regex::Regex;

/// XSS sanitizer
/// معقم XSS
pub struct Xss {
    // Configurable allowed tags for rich text
    allowed_tags: Vec<&'static str>,
    strip_scripts: bool,
    strip_event_handlers: bool,
}

impl Default for Xss {
    fn default() -> Self {
        Self::new()
    }
}

impl Xss {
    pub fn new() -> Self {
        Self {
            allowed_tags: vec![],
            strip_scripts: true,
            strip_event_handlers: true,
        }
    }
    
    /// Allow specific tags (for rich text editors)
    /// السماح بـ tags محددة
    pub fn allow_tags(mut self, tags: Vec<&'static str>) -> Self {
        self.allowed_tags = tags;
        self
    }
    
    /// Escape HTML entities in a string
    /// تحويل HTML entities في النص
    pub fn escape(input: &str) -> String {
        let mut output = String::with_capacity(input.len());
        for c in input.chars() {
            match c {
                '&' => output.push_str("&amp;"),
                '<' => output.push_str("&lt;"),
                '>' => output.push_str("&gt;"),
                '"' => output.push_str("&quot;"),
                '\'' => output.push_str("&#x27;"),
                '/' => output.push_str("&#x2F;"),
                _ => output.push(c),
            }
        }
        output
    }
    
    /// Unescape HTML entities
    /// عكس تحويل HTML entities
    pub fn unescape(input: &str) -> String {
        input
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#x27;", "'")
            .replace("&#x2F;", "/")
    }
    
    /// Sanitize input by removing dangerous content
    /// تعقيم المدخلات بإزالة المحتوى الخطير
    pub fn sanitize(&self, input: &str) -> String {
        let mut result = input.to_string();
        
        if self.strip_scripts {
            // Remove <script> tags and their content
            let script_re = Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap();
            result = script_re.replace_all(&result, "").to_string();
            
            // Remove standalone <script> tags
            let script_tag_re = Regex::new(r"(?i)<script[^>]*>").unwrap();
            result = script_tag_re.replace_all(&result, "").to_string();
        }
        
        if self.strip_event_handlers {
            // Remove on* event handlers (onclick, onload, etc.)
            let event_re = Regex::new(r#"(?i)\s+on\w+\s*=\s*["'][^"']*["']"#).unwrap();
            result = event_re.replace_all(&result, "").to_string();
            
            // Remove unquoted event handlers
            let unquoted_re = Regex::new(r"(?i)\s+on\w+\s*=\s*[^\s>]+").unwrap();
            result = unquoted_re.replace_all(&result, "").to_string();
        }
        
        // Remove javascript: URLs
        let js_url_re = Regex::new(r"(?i)javascript:").unwrap();
        result = js_url_re.replace_all(&result, "").to_string();
        
        // Remove data: URLs that could be malicious
        let data_url_re = Regex::new(r#"(?i)data:text/html"#).unwrap();
        result = data_url_re.replace_all(&result, "").to_string();
        
        // Remove vbscript: URLs
        let vbscript_re = Regex::new(r"(?i)vbscript:").unwrap();
        result = vbscript_re.replace_all(&result, "").to_string();
        
        result
    }
    
    /// Sanitize and escape - the safest option
    /// تعقيم وتحويل - الخيار الأكثر أماناً
    pub fn clean(&self, input: &str) -> String {
        Self::escape(&self.sanitize(input))
    }
    
    /// Validate that input doesn't contain XSS patterns
    /// التحقق من أن المدخلات لا تحتوي على أنماط XSS
    pub fn is_safe(input: &str) -> bool {
        let dangerous_patterns = [
            "<script", "</script", "javascript:", "onerror=", "onload=",
            "onclick=", "onmouseover=", "<iframe", "<object", "<embed",
            "vbscript:", "data:text/html",
        ];
        
        let lower = input.to_lowercase();
        !dangerous_patterns.iter().any(|p| lower.contains(p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_escape() {
        // The escape function also escapes '/' to '&#x2F;' per OWASP guidance
        // (prevents </script> tag injection). Update the expectation accordingly.
        assert_eq!(
            Xss::escape("<script>alert('xss')</script>"),
            "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;&#x2F;script&gt;"
        );
    }
    
    #[test]
    fn test_sanitize() {
        let xss = Xss::new();
        let input = "<script>alert('xss')</script><p>hello</p>";
        let result = xss.sanitize(input);
        assert!(!result.contains("<script>"));
        assert!(result.contains("<p>hello</p>"));
    }
    
    #[test]
    fn test_is_safe() {
        assert!(Xss::is_safe("hello world"));
        assert!(!Xss::is_safe("<script>alert(1)</script>"));
        assert!(!Xss::is_safe("javascript:alert(1)"));
    }
}
